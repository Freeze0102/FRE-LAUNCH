//! A private JDK 25 for FRE LAUNCH's pinned Minecraft resource exporter.
//! A JRE is not enough: the original MIT exporter Java sources require javac.
//! Existing JDKs are reused. A missing JDK is fetched from Eclipse Adoptium
//! over HTTPS with a publisher-provided SHA-256 and extracted only under FRE's
//! own application data. Neither external launcher nor a global Java install
//! is required. No JVM bytes are redistributed with the FRE installer.
use std::{fs, io::Write, path::{Component, Path, PathBuf}, time::Duration};
use sha2::{Digest, Sha256};
use tokio::{io::AsyncWriteExt, process::Command};

const META_URL: &str = "https://api.adoptium.net/v3/assets/latest/25/hotspot?architecture=x64&image_type=jdk&os=windows&vendor=eclipse";
const MAX_ARCHIVE: u64 = 460 * 1024 * 1024;
const MAX_EXPANDED: u64 = 1700 * 1024 * 1024;

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|c| c.is_ascii_hexdigit())
}

fn binary_download_url(value: &str) -> Result<(), String> {
    let url = url::Url::parse(value).map_err(|_| "Adoptium supplied a malformed download URL.")?;
    if url.scheme() != "https" || url.host_str() != Some("github.com")
        || !url.path().starts_with("/adoptium/temurin25-binaries/releases/download/")
        || !url.path().ends_with(".zip") || url.port().is_some()
        || !url.username().is_empty() || url.password().is_some() || url.fragment().is_some()
    {
        return Err("Refusing a JDK download outside the official Adoptium release.".into());
    }
    Ok(())
}

#[cfg(windows)]
fn hidden(command: &mut Command) {
    command.creation_flags(0x08000000);
}
#[cfg(not(windows))]
fn hidden(_command: &mut Command) {}

async fn version(output: &Path, argument: &str) -> bool {
    if !output.is_file() { return false; }
    let mut cmd = Command::new(output);
    cmd.arg(argument).kill_on_drop(true);
    hidden(&mut cmd);
    let Ok(Ok(result)) = tokio::time::timeout(Duration::from_secs(10), cmd.output()).await
    else { return false };
    if !result.status.success() { return false; }
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    reports_java_25(&text, argument == "--version")
}

fn reports_java_25(text: &str, compiler: bool) -> bool {
    text.lines().any(|line| {
        let line = line.trim();
        let value = if compiler {
            line.strip_prefix("javac ")
        } else {
            line.strip_prefix("openjdk version ").or_else(|| line.strip_prefix("java version "))
        };
        value.and_then(|value| value.trim_start_matches('"').split(['.', '-', '+', '"', ' ']).next())
            .is_some_and(|major| major == "25")
    })
}

async fn valid_jdk(home: &Path) -> bool {
    if !home.symlink_metadata().is_ok_and(|m| m.file_type().is_dir()) { return false; }
    #[cfg(windows)]
    let (java, javac) = (home.join("bin/java.exe"), home.join("bin/javac.exe"));
    #[cfg(not(windows))]
    let (java, javac) = (home.join("bin/java"), home.join("bin/javac"));
    version(&javac, "--version").await && version(&java, "-version").await
}

async fn installed_jdk() -> Option<PathBuf> {
    for variable in ["JAVA_HOME", "JDK_HOME"] {
        if let Some(home) = std::env::var_os(variable) {
            let path = PathBuf::from(home);
            if valid_jdk(&path).await { return Some(path); }
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        for bin in std::env::split_paths(&path) {
            if let Some(home) = bin.parent() {
                if valid_jdk(home).await { return Some(home.to_owned()); }
            }
        }
    }
    #[cfg(windows)]
    for folder in [
        r"C:\Program Files\Eclipse Adoptium",
        r"C:\Program Files\Java",
        r"C:\Program Files\Microsoft",
    ] {
        if let Ok(versions) = fs::read_dir(folder) {
            for entry in versions.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if name.starts_with("jdk-25") || name.starts_with("microsoft-jdk-25") {
                    let path = entry.path();
                    if valid_jdk(&path).await { return Some(path); }
                }
            }
        }
    }
    None
}

fn staging_path(root: &Path, name: &str) -> Result<PathBuf, String> {
    let path = root.join(name);
    if path.symlink_metadata().is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("Refusing an unsafe link in FRE's managed Java cache.".into());
    }
    Ok(path)
}

const JDK_STAGING_MARKER: &[u8] = b"FRE-LAUNCH-MANAGED-JDK-25-STAGING-V1";
fn clean_own_stage(root: &Path, stage: &Path) -> Result<(), String> {
    if !stage.exists() { return Ok(()); }
    let real_root = root.canonicalize().map_err(|e| e.to_string())?;
    let real_stage = stage.canonicalize().map_err(|e| e.to_string())?;
    if real_stage.parent() != Some(real_root.as_path())
        || !stage.symlink_metadata().is_ok_and(|m| m.file_type().is_dir())
        || fs::read(stage.join(".fre-jdk-staging")).ok().as_deref() != Some(JDK_STAGING_MARKER)
    {
        return Err("Refusing to erase an unrecognized or redirected Java staging folder.".into());
    }
    fs::remove_dir_all(stage).map_err(|e| format!("Cannot clean incomplete JDK installation: {e}"))
}
fn extract_archive(archive: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|e| e.to_string())?;
    fs::OpenOptions::new().create_new(true).write(true)
        .open(destination.join(".fre-jdk-staging"))
        .and_then(|mut file| file.write_all(JDK_STAGING_MARKER))
        .map_err(|e| format!("Cannot initialize FRE Java staging folder: {e}"))?;
    let file = fs::File::open(archive).map_err(|e| format!("Cannot open downloaded JDK: {e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("Invalid JDK ZIP archive: {e}"))?;
    if zip.len() == 0 || zip.len() > 35000 {
        return Err("Unexpected number of files in the JDK archive.".into());
    }
    let mut total = 0u64;
    for index in 0..zip.len() {
        let mut item = zip.by_index(index).map_err(|e| format!("Invalid JDK entry: {e}"))?;
        let name = item.enclosed_name().ok_or("Unsafe path in the JDK ZIP archive.")?;
        if item.name().len() > 480 {
            return Err("Overlong file name in the JDK ZIP archive.".into());
        }
        let mut parts = name.components();
        let Some(Component::Normal(root)) = parts.next() else {
            return Err("Invalid JDK ZIP archive root.".into());
        };
        if !root.to_string_lossy().to_ascii_lowercase().starts_with("jdk-25") {
            return Err("JDK ZIP archive has an unexpected root folder.".into());
        }
        let relative: PathBuf = parts.collect();
        if relative.as_os_str().is_empty() { continue; }
        if !relative.components().all(|part| matches!(part, Component::Normal(_))) {
            return Err("Refusing JDK ZIP archive path traversal.".into());
        }
        if item.unix_mode().is_some_and(|mode| (mode & 0o170000) == 0o120000) {
            return Err("Refusing symbolic links inside a downloaded JDK ZIP.".into());
        }
        total = total.checked_add(item.size()).ok_or("JDK expanded size overflow.")?;
        if total > MAX_EXPANDED || item.size() > 350 * 1024 * 1024 {
            return Err("JDK ZIP archive exceeds safe extraction limits.".into());
        }
        let target = destination.join(relative);
        if item.is_dir() {
            fs::create_dir_all(&target).map_err(|e| e.to_string())?;
            continue;
        }
        let parent = target.parent().ok_or("Invalid JDK ZIP target.")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let mut output = fs::OpenOptions::new().write(true).create_new(true)
            .open(&target).map_err(|e| format!("Cannot write managed Java: {e}"))?;
        let count = std::io::copy(&mut item, &mut output)
            .map_err(|e| format!("Cannot extract managed Java: {e}"))?;
        if count != item.size() {
            return Err("JDK ZIP entry length changed during extraction.".into());
        }
        output.flush().map_err(|e| e.to_string())?;
    }
    Ok(())
}

async fn download_jdk(archive: &Path) -> Result<(), String> {
    // GitHub releases redirect to a GitHub-owned asset CDN. Never follow a
    // cross-provider or HTTP redirect, even if a remote response asks for one.
    let redirect = reqwest::redirect::Policy::custom(|attempt| {
        let url = attempt.url();
        let host = url.host_str().unwrap_or("");
        if attempt.previous().len() > 5 || url.scheme() != "https"
            || !matches!(host, "github.com" | "release-assets.githubusercontent.com" | "objects.githubusercontent.com")
        {
            attempt.error("Refusing an untrusted JDK download redirect.")
        } else {
            attempt.follow()
        }
    });
    let client = reqwest::Client::builder()
        .user_agent("FRE-LAUNCH/0.5.11 managed-JDK")
        .timeout(Duration::from_secs(20 * 60))
        .redirect(redirect)
        .build().map_err(|e| e.to_string())?;
    let mut meta = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(20))
        .build().map_err(|e| e.to_string())?
        .get(META_URL).send().await
        .map_err(|e| format!("Could not query Eclipse Adoptium: {e}"))?
        .error_for_status().map_err(|e| format!("Eclipse Adoptium unavailable: {e}"))?;
    if meta.content_length().is_some_and(|len| len > 2 * 1024 * 1024) {
        return Err("Unexpected Adoptium metadata size.".into());
    }
    let mut raw = Vec::new();
    while let Some(chunk) = meta.chunk().await.map_err(|e| e.to_string())? {
        if raw.len() + chunk.len() > 2 * 1024 * 1024 {
            return Err("Unexpected Adoptium metadata size.".into());
        }
        raw.extend_from_slice(&chunk);
    }
    let releases: serde_json::Value = serde_json::from_slice(&raw)
        .map_err(|e| format!("Invalid Adoptium JDK metadata: {e}"))?;
    let binary = releases.as_array().and_then(|versions| versions.first())
        .and_then(|version| version.get("binary"))
        .ok_or("Adoptium did not publish a Windows x64 JDK 25.")?;
    if binary["os"] != "windows" || binary["architecture"] != "x64"
        || binary["image_type"] != "jdk" {
        return Err("Adoptium returned the wrong Java platform or image type.".into());
    }
    let pkg = &binary["package"];
    let url = pkg["link"].as_str().ok_or("Missing Adoptium JDK URL.")?;
    let expected = pkg["checksum"].as_str().ok_or("Missing Adoptium JDK SHA-256.")?;
    binary_download_url(url)?;
    if !valid_digest(expected) { return Err("Invalid Adoptium JDK SHA-256.".into()); }
    let response = client.get(url).send().await
        .map_err(|e| format!("Could not download JDK 25: {e}"))?
        .error_for_status().map_err(|e| format!("JDK download rejected: {e}"))?;
    if response.content_length().is_some_and(|n| n > MAX_ARCHIVE) {
        return Err("JDK archive exceeds maximum expected size.".into());
    }
    let mut response = response;
    let mut file = tokio::fs::File::create(archive).await
        .map_err(|e| format!("Cannot stage managed JDK archive: {e}"))?;
    let mut hasher = Sha256::new();
    let mut size = 0u64;
    while let Some(chunk) = response.chunk().await.map_err(|e| format!("JDK transfer interrupted: {e}"))? {
        size = size.checked_add(chunk.len() as u64).ok_or("JDK archive length overflow.")?;
        if size > MAX_ARCHIVE { return Err("JDK archive exceeds maximum size.".into()); }
        hasher.update(&chunk);
        file.write_all(&chunk).await.map_err(|e| format!("Cannot write JDK archive: {e}"))?;
    }
    file.flush().await.map_err(|e| e.to_string())?;
    drop(file);
    if size < 1024 * 1024 || !format!("{:x}", hasher.finalize()).eq_ignore_ascii_case(expected) {
        return Err("Managed JDK download checksum mismatch or truncated archive.".into());
    }
    Ok(())
}

/// Returns a full Java home with both java.exe AND javac.exe.
pub async fn ensure(data: &Path) -> Result<PathBuf, String> {
    if let Some(home) = installed_jdk().await { return Ok(home); }
    #[cfg(not(windows))]
    return Err("A JDK 25 is required on this platform; set JAVA_HOME before preparing native Minecraft resources.".into());

    #[cfg(windows)]
    {
        let root = data.join("managed-java");
        fs::create_dir_all(&root).map_err(|e| format!("Cannot create FRE Java cache: {e}"))?;
        if !root.symlink_metadata().is_ok_and(|m| m.file_type().is_dir()) {
            return Err("Unsafe FRE Java cache root.".into());
        }
        let target = staging_path(&root, "jdk-25")?;
        if valid_jdk(&target).await { return Ok(target); }
        if target.exists() {
            return Err("FRE's private JDK 25 cache is incomplete; back it up or rename it before retrying.".into());
        }
        let stage = staging_path(&root, "jdk-25.pending")?;
        clean_own_stage(&root, &stage)?;
        let archive = staging_path(&root, "jdk-25.download")?;
        if archive.exists() {
            if !archive.is_file() { return Err("Unsafe incomplete JDK archive.".into()); }
            fs::remove_file(&archive).map_err(|e| e.to_string())?;
        }
        let result: Result<(), String> = async {
            download_jdk(&archive).await?;
            let zip = archive.clone();
            let out = stage.clone();
            tokio::task::spawn_blocking(move || extract_archive(&zip, &out))
                .await.map_err(|e| format!("Could not unpack JDK: {e}"))??;
            if !valid_jdk(&stage).await {
                return Err("Downloaded JDK archive does not contain a valid Java 25 compiler.".into());
            }
            fs::rename(&stage, &target).map_err(|e| format!("Cannot activate FRE's managed JDK: {e}"))?;
            Ok(())
        }.await;
        let _ = fs::remove_file(&archive);
        if result.is_err() { let _ = clean_own_stage(&root, &stage); }
        result?;
        Ok(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_untrusted_java_urls_and_hashes() {
        assert!(binary_download_url("https://github.com/adoptium/temurin25-binaries/releases/download/jdk-25.0.5%2B8/OpenJDK25U-jdk_x64_windows_hotspot_25.0.5_8.zip").is_ok());
        assert!(binary_download_url("http://github.com/adoptium/temurin25-binaries/releases/download/file.zip").is_err());
        assert!(binary_download_url("https://example.com/adoptium/fake.zip").is_err());
        assert!(!valid_digest("deadbeef"));
    }
    #[test]
    fn accepts_exact_java_25_ga_and_patch_releases() {
        assert!(reports_java_25("openjdk version \"25\" 2025-09-16", false));
        assert!(reports_java_25("openjdk version \"25.0.1\" 2025-10-21", false));
        assert!(reports_java_25("javac 25", true));
        assert!(reports_java_25("javac 25.0.1", true));
        assert!(!reports_java_25("javac 250", true));
        assert!(!reports_java_25("openjdk version \"21.0.9\"", false));
    }
}
