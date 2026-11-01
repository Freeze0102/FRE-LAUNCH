//! Optional CurseForge source for FRE LAUNCH.
//! Requires an approved CurseForge third-party API key. Never embeds or proxies
//! a shared secret, bypasses author distribution controls, or scrapes the site.
use serde::{Deserialize, Serialize};
use sha1::{Digest as _, Sha1};
use std::time::Duration;
use tauri_plugin_http::reqwest::{self, Client};

const MINECRAFT_GAME_ID: &str = "432";
const MINECRAFT_MOD_CLASS_ID: &str = "6";
const MAX_MOD_BYTES: usize = 128 * 1024 * 1024;
static INSTALL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("fre-curseforge")
        .invoke_handler(tauri::generate_handler![cf_search, cf_install])
        .build()
}

#[derive(Deserialize)]
struct Envelope<T> {
    data: T,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeMod {
    id: u64,
    name: String,
    summary: String,
    slug: String,
    logo: Option<CurseForgeLogo>,
    allow_mod_distribution: Option<bool>,
    is_available: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CurseForgeLogo {
    thumbnail_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CurseForgeFile {
    id: u64,
    mod_id: u64,
    game_id: u64,
    file_name: String,
    file_date: String,
    file_length: u64,
    is_available: bool,
    game_versions: Vec<String>,
    hashes: Vec<CurseForgeHash>,
    download_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CurseForgeHash {
    algo: u8,
    value: String,
}

fn validated_api_key(session_key: Option<String>) -> Result<String, String> {
    let raw = session_key
        .filter(|v| !v.trim().is_empty())
        .or_else(|| std::env::var("FRE_CURSEFORGE_API_KEY").ok())
        .ok_or_else(|| "CurseForge needs an approved API key. Enter yours for this session or set FRE_CURSEFORGE_API_KEY.".to_string())?;
    let key = raw.trim();
    if key.len() < 12 || key.len() > 256 || key.chars().any(char::is_control) {
        return Err("Invalid CurseForge API key format.".to_string());
    }
    Ok(key.to_string())
}

fn http() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(35))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))
}

async fn cf_get<T: serde::de::DeserializeOwned>(
    client: &Client,
    key: &str,
    path: &str,
    query: &[(&str, String)],
) -> Result<T, String> {
    let response = client
        .get(format!("https://api.curseforge.com/v1/{path}"))
        .header("x-api-key", key)
        .query(query)
        .send()
        .await
        .map_err(|e| format!("CurseForge connection error: {e}"))?;
    let status = response.status();
    if !status.is_success() {
        return Err(if status.as_u16() == 403 || status.as_u16() == 401 {
            "CurseForge rejected the API key or this project is not available for third-party distribution.".into()
        } else {
            format!("CurseForge API returned HTTP {status}")
        });
    }
    response
        .json::<Envelope<T>>()
        .await
        .map(|envelope| envelope.data)
        .map_err(|e| format!("Invalid CurseForge API response: {e}"))
}

fn loader_type(loader: &str) -> Option<&'static str> {
    match loader.to_ascii_lowercase().as_str() {
        "forge" => Some("1"),
        "fabric" => Some("4"),
        "quilt" => Some("5"),
        "neoforge" => Some("6"),
        _ => None,
    }
}

#[tauri::command]
async fn cf_search(
    query: String,
    game_version: Option<String>,
    mod_loader: Option<String>,
    api_key: Option<String>,
) -> Result<Vec<CurseForgeMod>, String> {
    let key = validated_api_key(api_key)?;
    let query = query.trim();
    if query.chars().count() > 100 {
        return Err("Search text is too long.".into());
    }
    let mut params = vec![
        ("gameId", MINECRAFT_GAME_ID.to_string()),
        ("classId", MINECRAFT_MOD_CLASS_ID.to_string()),
        ("pageSize", "24".to_string()),
        ("sortField", "2".to_string()),
    ];
    if !query.is_empty() {
        params.push(("searchFilter", query.to_string()));
    }
    if let Some(version) = game_version.as_deref().filter(|v| !v.is_empty()) {
        params.push(("gameVersion", version.to_string()));
        if let Some(loader) = mod_loader.as_deref().and_then(loader_type) {
            params.push(("modLoaderType", loader.to_string()));
        }
    }
    cf_get::<Vec<CurseForgeMod>>(&http()?, &key, "mods/search", &params).await
}

fn compatible(file: &CurseForgeFile, game_version: &str, loader: &str) -> bool {
    file.is_available
        && file.game_id == 432
        && file.file_name.to_ascii_lowercase().ends_with(".jar")
        && file.game_versions.iter().any(|s| s == game_version)
        && file.game_versions.iter().any(|s| s.eq_ignore_ascii_case(loader))
        && file.file_length > 0
        && file.file_length <= MAX_MOD_BYTES as u64
}

fn verify_hash(data: &[u8], hashes: &[CurseForgeHash]) -> Result<(), String> {
    // CurseForge documents only 1 = SHA-1 and 2 = MD5; do not invent algo 3.
    let sha1 = hashes.iter().find(|h| h.algo == 1 && h.value.len() == 40);
    if let Some(hash) = sha1 {
        let value = format!("{:x}", Sha1::digest(data));
        if !value.eq_ignore_ascii_case(&hash.value) {
            return Err("CurseForge SHA-1 verification failed. File not installed.".into());
        }
    } else {
        return Err("CurseForge did not provide a supported file checksum. File not installed.".into());
    }
    Ok(())
}

#[tauri::command]
async fn cf_install(
    mod_id: u64,
    instance_id: String,
    api_key: Option<String>,
) -> Result<String, String> {
    let _install = INSTALL.try_lock().map_err(|_| "Another CurseForge installation is still running.")?;
    let key = validated_api_key(api_key)?;
    let metadata = theseus::instance::get(&instance_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("Minecraft instance not found.")?;
    if metadata.quarantined || metadata.shared_instance.is_some() {
        return Err("Installing into a quarantined or shared instance is not supported.".into());
    }
    let game_version = metadata.applied_content_set.game_version.clone();
    let loader = metadata.applied_content_set.loader.as_str();
    let loader_id = loader_type(loader)
        .ok_or("Choose a Fabric, Forge, NeoForge, or Quilt instance before installing mods.")?;
    let client = http()?;
    let project = cf_get::<CurseForgeMod>(
        &client, &key, &format!("mods/{mod_id}"), &[],
    ).await?;
    if project.is_available == Some(false) {
        return Err("CurseForge reports this mod is unavailable.".into());
    }
    if project.allow_mod_distribution != Some(true) {
        return Err("The author has not enabled third-party distribution for this mod.".into());
    }

    let files = cf_get::<Vec<CurseForgeFile>>(
        &client,
        &key,
        &format!("mods/{mod_id}/files"),
        &[
            ("gameVersion", game_version.clone()),
            ("modLoaderType", loader_id.to_string()),
            ("pageSize", "50".to_string()),
        ],
    ).await?;
    let file = files.iter()
        .filter(|f| f.mod_id == mod_id && compatible(f, &game_version, loader))
        .max_by(|a, b| a.file_date.cmp(&b.file_date))
        .ok_or("No compatible downloadable JAR for this Minecraft version and mod loader.")?;
    validate_file_name(&file.file_name)?;
    let download = if let Some(url) = &file.download_url {
        url.clone()
    } else {
        cf_get::<String>(
            &client,
            &key,
            &format!("mods/{mod_id}/files/{}/download-url", file.id),
            &[],
        ).await.map_err(|_| "CurseForge has not permitted a direct download for this file.")?
    };
    let url = download_url(&download)?;
    // The API key belongs only on api.curseforge.com, never on the file CDN.
    let mut response = client.get(url).send()
        .await.map_err(|e| format!("CurseForge download error: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("CurseForge file download failed (HTTP {})", response.status()));
    }
    if response.content_length().is_some_and(|v| v > MAX_MOD_BYTES as u64) {
        return Err("CurseForge file exceeds the 128 MiB safety limit.".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len().saturating_add(chunk.len()) > MAX_MOD_BYTES {
            return Err("Download exceeded the safety limit.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    if bytes.len() as u64 != file.file_length {
        return Err("CurseForge file size mismatch. File not installed.".into());
    }
    if !bytes.starts_with(b"PK\x03\x04") {
        return Err("The downloaded file is not a valid JAR/ZIP header.".into());
    }
    verify_hash(&bytes, &file.hashes)?;
    let bytes = tokio::task::spawn_blocking(move || {
        validate_jar(&bytes)?;
        Ok::<_, String>(bytes)
    }).await.map_err(|e| e.to_string())??;
    let current = theseus::instance::get(&instance_id).await.map_err(|e| e.to_string())?
        .ok_or("Minecraft instance was removed while downloading.")?;
    if current.quarantined || current.shared_instance.is_some()
        || current.applied_content_set.game_version != game_version
        || current.applied_content_set.loader.as_str() != loader
    {
        return Err("Instance changed while downloading. Refresh and try again.".into());
    }
    if !theseus::process::get_by_instance_id(&instance_id).await.map_err(|e| e.to_string())?.is_empty() {
        return Err("Close Minecraft before changing this instance's mods.".into());
    }
    let existing = theseus::instance::sync_content_files(&instance_id).await.map_err(|e| e.to_string())?;
    let hash = format!("{:x}", Sha1::digest(&bytes));
    if existing.iter().any(|f| f.sha1.eq_ignore_ascii_case(&hash)
        || f.file_name.eq_ignore_ascii_case(&file.file_name)) {
        return Err("This JAR or filename is already in the instance. Manage it in the Content tab; no file was replaced.".into());
    }
    // Files API deliberately makes mods/ read-only in this upstream revision.
    // Import through the content store so the JAR is registered and manageable.
    let staging = tempfile::tempdir().map_err(|e| e.to_string())?;
    let staged_file = staging.path().join(&file.file_name);
    tokio::fs::write(&staged_file, &bytes).await.map_err(|e| e.to_string())?;
    theseus::instance::add_project_from_path(
        &instance_id, &staged_file, Some(theseus::data::ProjectType::Mod),
    ).await.map_err(|e| format!("Could not import mod: {e}"))?;
    Ok(format!("Installed {} from CurseForge into the selected Minecraft instance.", file.file_name))
}

fn validate_file_name(name: &str) -> Result<(), String> {
    if !path_util::is_safe_file_name(name)
        || name.len() > 180
        || !name.to_ascii_lowercase().ends_with(".jar")
        || name.chars().any(|ch| ch.is_control() || "<>:\"|?*".contains(ch))
    {
        return Err("Unsafe CurseForge file name.".into());
    }
    Ok(())
}

fn validate_jar(bytes: &[u8]) -> Result<(), String> {
    let mut jar = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|_| "Downloaded JAR is not a complete ZIP archive.")?;
    if jar.is_empty() || jar.len() > 100_000 {
        return Err("JAR has an invalid number of entries.".into());
    }
    let mut total = 0u64;
    for index in 0..jar.len() {
        let entry = jar.by_index(index).map_err(|e| format!("Invalid JAR entry: {e}"))?;
        if entry.enclosed_name().is_none() || entry.name().contains('\\') || entry.is_symlink() {
            return Err("JAR contains an unsafe entry path.".into());
        }
        total = total.saturating_add(entry.size());
        if total > 512 * 1024 * 1024 {
            return Err("JAR exceeds the unpacked size limit.".into());
        }
        let expected = entry.size();
        let actual = std::io::copy(&mut std::io::Read::take(entry, expected + 1), &mut std::io::sink())
            .map_err(|e| format!("JAR integrity check failed: {e}"))?;
        if actual != expected { return Err("JAR entry size mismatch.".into()); }
    }
    Ok(())
}

fn download_url(download: &str) -> Result<reqwest::Url, String> {
    let url = reqwest::Url::parse(download).map_err(|_| "Invalid file URL")?;
    let host = url.host_str().ok_or("File URL has no host")?;
    if url.scheme() != "https"
        || !(host == "forgecdn.net" || host.ends_with(".forgecdn.net"))
        || !url.username().is_empty() || url.password().is_some()
        || url.port_or_known_default() != Some(443)
    {
        return Err("CurseForge returned an unapproved download URL.".into());
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loader_mapping() {
        assert_eq!(loader_type("fabric"), Some("4"));
        assert_eq!(loader_type("neoforge"), Some("6"));
        assert_eq!(loader_type("vanilla"), None);
    }
    #[test]
    fn rejects_mismatched_loader_and_version() {
        let file = CurseForgeFile {
            id: 1,
            mod_id: 2,
            game_id: 432,
            file_name: "example.jar".into(),
            file_date: "2026-01-01".into(),
            file_length: 2,
            is_available: true,
            game_versions: vec!["1.21.1".into(), "Fabric".into()],
            hashes: vec![],
            download_url: None,
        };
        assert!(compatible(&file, "1.21.1", "fabric"));
        assert!(!compatible(&file, "1.21.2", "fabric"));
        assert!(!compatible(&file, "1.21.1", "forge"));
    }
    #[test]
    fn refuses_unverified_bytes() {
        assert!(verify_hash(b"test", &[]).is_err());
        let hash = CurseForgeHash { algo: 1, value: format!("{:x}", Sha1::digest(b"test")) };
        assert!(verify_hash(b"test", std::slice::from_ref(&hash)).is_ok());
        assert!(verify_hash(b"changed", &[hash]).is_err());
    }
    #[test]
    fn refuses_unsafe_file_names_and_urls() {
        for name in ["../mod.jar", "C:mod.jar", "CON.jar", "mod?.jar", "mod.jar:stream", "a.exe"] {
            assert!(validate_file_name(name).is_err(), "{name}");
        }
        assert!(validate_file_name("fabric-api-1.0.jar").is_ok());
        assert!(download_url("https://edge.forgecdn.net/files/1/mod.jar").is_ok());
        for url in ["http://edge.forgecdn.net/a", "https://forgecdn.net.evil.test/a", "https://evil.test/a", "https://user@edge.forgecdn.net/a", "https://edge.forgecdn.net:444/a"] {
            assert!(download_url(url).is_err(), "{url}");
        }
    }
    #[test]
    fn rejects_truncated_or_unsafe_archives() {
        use std::io::{Cursor, Write};
        assert!(validate_jar(b"PK\x03\x04not-a-jar").is_err());
        let archive = |name: &str| {
            let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
            zip.start_file(name, zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored)).unwrap();
            zip.write_all(b"example").unwrap();
            zip.finish().unwrap().into_inner()
        };
        assert!(validate_jar(&archive("fabric.mod.json")).is_ok());
        assert!(validate_jar(&archive("../escape.class")).is_err());
        let mut bad_crc = archive("test.class");
        let name_length = u16::from_le_bytes([bad_crc[26], bad_crc[27]]) as usize;
        let extra_length = u16::from_le_bytes([bad_crc[28], bad_crc[29]]) as usize;
        bad_crc[30 + name_length + extra_length] ^= 1;
        assert!(validate_jar(&bad_crc).is_err());
    }
}
