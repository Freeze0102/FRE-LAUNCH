//! Verified, bounded, streamed downloads used for Mojang assets and libraries.
//! The client is restricted to trusted upstream HTTPS hosts, including redirects.
use futures::StreamExt;
use sha1::{Digest, Sha1};
use std::{path::Path, time::Duration};
use tokio::{fs, io::{AsyncReadExt, AsyncWriteExt}};
use uuid::Uuid;

pub const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ATTEMPTS: u32 = 3;

fn retryable_download_error(error: &str) -> bool {
    if error.starts_with("Download failed:") ||
       error.starts_with("Download stream interrupted:") ||
       error.starts_with("Downloaded file hash mismatch") {return true;}
    // HTTP status is emitted as a bare numeric code below. Avoid brittle
    // matching against reqwest's human-readable error representation.
    error.strip_prefix("HTTP error: ")
        .and_then(|detail| detail.get(..3))
        .and_then(|code|code.parse::<u16>().ok())
        .is_some_and(|code|code == 408 || code == 429 || (500..=599).contains(&code))
}

pub fn trusted_url(url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url).map_err(|e| e.to_string())?;
    if parsed.scheme() != "https" { return Err("Downloads must use HTTPS".into()); }
    let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();
    if !matches!(host.as_str(),
        "piston-meta.mojang.com" | "piston-data.mojang.com" |
        "resources.download.minecraft.net" | "libraries.minecraft.net" |
        "launcher.mojang.com" | "launchermeta.mojang.com" |
        "meta.mojang.com" | "s3.amazonaws.com" |
        "maven.fabricmc.net" | "meta.fabricmc.net" |
        "meta.quiltmc.org" | "maven.quiltmc.org") {
        return Err(format!("Untrusted Minecraft download domain: {host}"));
    }
    Ok(())
}

pub fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(concat!("FRE-LAUNCH/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(180))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 {
                return attempt.error("Too many redirects");
            }
            match trusted_url(attempt.url().as_str()) {
                Ok(()) => attempt.follow(),
                Err(reason) => attempt.error(reason),
            }
        }))
        .build().map_err(|e| e.to_string())
}

fn is_zip_header(bytes: &[u8]) -> bool {
    matches!(bytes, [b'P',b'K',3,4] | [b'P',b'K',5,6] | [b'P',b'K',7,8])
}
async fn looks_like_jar(path: &Path) -> Result<bool, String> {
    let mut file=fs::File::open(path).await.map_err(|e|e.to_string())?;
    let mut header=[0u8;4];
    match file.read_exact(&mut header).await {
        Ok(_) => Ok(is_zip_header(&header)),
        Err(error) if error.kind()==std::io::ErrorKind::UnexpectedEof => Ok(false),
        Err(error)=>Err(error.to_string())
    }
}
/// For historical libraries without Mojang SHA-1 metadata, reuse only
/// existing, non-empty JARs with a ZIP header (not partial HTML error pages).
pub async fn usable_unhashed_cache(path:&Path) -> Result<bool,String> {
    if !fs::try_exists(path).await.map_err(|e|e.to_string())? {return Ok(false);}
    let info=fs::metadata(path).await.map_err(|e|e.to_string())?;
    if !info.is_file() || info.len()==0 {return Ok(false);}
    let is_jar=path.extension().and_then(|ext|ext.to_str())
        .is_some_and(|ext|ext.eq_ignore_ascii_case("jar"));
    if is_jar {looks_like_jar(path).await} else {Ok(true)}
}

pub async fn verify_file(path: &Path, expected_sha1: &str) -> Result<bool, String> {
    if !fs::try_exists(path).await.map_err(|e| e.to_string())? { return Ok(false); }
    let mut file = fs::File::open(path).await.map_err(|e| e.to_string())?;
    let mut buffer = [0u8; 65536];
    let mut hasher = Sha1::new();
    loop {
        let n = file.read(&mut buffer).await.map_err(|e| e.to_string())?;
        if n == 0 { break; }
        hasher.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()).eq_ignore_ascii_case(expected_sha1))
}

pub async fn download_one(
    client: &reqwest::Client, url: &str, destination: &Path, sha1: Option<&str>,
) -> Result<(), String> {
    // Interrupted Mojang/CDN connections are common on slower links. Retry only
    // network errors and transient HTTP responses; never retry disk failures.
    let mut last_error = String::new();
    for attempt in 1..=MAX_ATTEMPTS {
        match download_once(client, url, destination, sha1).await {
            Ok(()) => return Ok(()),
            Err(error) if attempt < MAX_ATTEMPTS && retryable_download_error(&error) => {
                last_error = error;
                tokio::time::sleep(Duration::from_millis(400u64 * 2u64.pow(attempt - 1))).await;
            }
            Err(error) => return Err(format!("Download failed after {attempt} attempt(s): {error}")),
        }
    }
    Err(format!("Download stopped: {last_error}"))
}

async fn download_once(
    client: &reqwest::Client, url: &str, destination: &Path, sha1: Option<&str>,
) -> Result<(), String> {
    trusted_url(url)?;
    if let Some(sha) = sha1 {
        if sha.len() != 40 || !sha.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err("Invalid expected SHA-1 digest".into());
        }
        if verify_file(destination, sha).await? { return Ok(()); }
    } else if usable_unhashed_cache(destination).await? {
        return Ok(());
    }

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
    }
    let response = client.get(url).send().await.map_err(|e| format!("Download failed: {e}"))?;
    if !response.status().is_success() {
        let status=response.status();
        return Err(format!("HTTP error: {} {}", status.as_u16(),
            status.canonical_reason().unwrap_or("unknown status")));
    }
    if let Some(size) = response.content_length() {
        if size > MAX_FILE_BYTES { return Err("Minecraft dependency exceeds file size limit".into()); }
    }

    let filename = destination.file_name().ok_or("Invalid download destination")?.to_string_lossy();
    let temp = destination.with_file_name(format!(".{filename}.{}.download", Uuid::new_v4()));
    let result = async {
        let mut file = fs::File::create(&temp).await.map_err(|e| e.to_string())?;
        let mut chunks = response.bytes_stream();
        let mut size = 0u64;
        let mut digest = Sha1::new();
        while let Some(chunk) = chunks.next().await {
            let chunk = chunk.map_err(|e| format!("Download stream interrupted: {e}"))?;
            size = size.checked_add(chunk.len() as u64).ok_or("Download size overflow")?;
            if size > MAX_FILE_BYTES { return Err("Downloaded file is too large".into()); }
            digest.update(&chunk);
            file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        }
        file.flush().await.map_err(|e| e.to_string())?;
        drop(file);
        if size == 0 { return Err("Downloaded file is empty".into()); }
        if let Some(expected) = sha1 {
            if !format!("{:x}", digest.finalize()).eq_ignore_ascii_case(expected) {
                return Err("Downloaded file hash mismatch".into());
            }
        } else if destination.extension().and_then(|ext|ext.to_str())
            .is_some_and(|ext|ext.eq_ignore_ascii_case("jar"))
            && !looks_like_jar(&temp).await? {
            return Err("Downloaded Maven artifact has an invalid JAR signature".into());
        }

        // Concurrent downloads may finish the same artifact; never replace a
        // correct file that another worker already installed.
        if let Some(expected) = sha1 {
            if verify_file(destination, expected).await? { return Ok(()); }
        }
        if fs::try_exists(destination).await.map_err(|e| e.to_string())? {
            fs::remove_file(destination).await.map_err(|e| e.to_string())?;
        }
        fs::rename(&temp, destination).await.map_err(|e| e.to_string())?;
        Ok(())
    }.await;
    // Cleanup even on network or integrity errors.
    let _ = fs::remove_file(&temp).await;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retry_only_transient_download_errors() {
        assert!(retryable_download_error("HTTP error: 503 Service Unavailable"));
        assert!(retryable_download_error("HTTP error: 408 Request Timeout"));
        assert!(retryable_download_error("HTTP error: 520 Unknown Error"));
        assert!(retryable_download_error("Download stream interrupted: timeout"));
        assert!(retryable_download_error("Downloaded file hash mismatch"));
        assert!(!retryable_download_error("HTTP error: 404 Not Found"));
        assert!(!retryable_download_error("Permission denied: cannot write"));
        assert!(!retryable_download_error("Invalid expected SHA-1 digest"));
    }
    #[test]
    fn maven_jar_requires_valid_archive_signature() {
        assert!(is_zip_header(&[80,75,3,4]));
        assert!(!is_zip_header(b"<htm"));
    }
    #[tokio::test]
    async fn cached_maven_html_error_page_is_rejected() {
        let path=std::env::temp_dir().join(format!("fre-bad-artifact-{}.jar",Uuid::new_v4()));
        fs::write(&path,b"<html>Service Unavailable</html>").await.unwrap();
        assert!(!looks_like_jar(&path).await.unwrap());
        assert!(!usable_unhashed_cache(&path).await.unwrap());
        fs::write(&path,&[80,75,3,4,0,0]).await.unwrap();
        assert!(looks_like_jar(&path).await.unwrap());
        assert!(usable_unhashed_cache(&path).await.unwrap());
        fs::remove_file(path).await.unwrap();
    }
    #[test]
    fn trusted_host_is_exact() {
        assert!(trusted_url("https://piston-data.mojang.com/v1/objects/foo").is_ok());
        assert!(trusted_url("https://maven.fabricmc.net/net/fabricmc/foo.jar").is_ok());
        assert!(trusted_url("https://meta.quiltmc.org/v3/versions/loader").is_ok());
        assert!(trusted_url("https://maven.quiltmc.org/repository/release/org/quiltmc/loader.jar").is_ok());
        assert!(trusted_url("https://maven.quiltmc.org.attacker.test/").is_err());
        assert!(trusted_url("http://piston-data.mojang.com/client.jar").is_err());
        assert!(trusted_url("https://piston-data.mojang.com.evil.test/").is_err());
        assert!(trusted_url("https://evil.test/?host=libraries.minecraft.net").is_err());
    }
    #[tokio::test]
    async fn verify_hash_matches_file() {
        let name = std::env::temp_dir().join(format!("fre-download-test-{}", Uuid::new_v4()));
        fs::write(&name, b"hello").await.unwrap();
        assert!(verify_file(&name, "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d").await.unwrap());
        assert!(!verify_file(&name, "0000000000000000000000000000000000000000").await.unwrap());
        fs::remove_file(name).await.unwrap();
    }
}
