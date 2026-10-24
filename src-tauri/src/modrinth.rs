use serde::{Deserialize, Serialize};
use futures::StreamExt;
use serde_json::Value;
use sha2::{Digest, Sha512};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

#[derive(Clone, Serialize, Deserialize)]
pub struct ModrinthHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub project_type: String,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
}

#[derive(Deserialize)]
struct SearchResults { hits: Vec<ModrinthHit> }

fn http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(concat!("FRE-LAUNCH/",env!("CARGO_PKG_VERSION")," (+https://github.com/Freeze0102/FRE-LAUNCH)"))
        .timeout(std::time::Duration::from_secs(35))
        // A Modrinth CDN link must not silently redirect the launcher to
        // an unrelated download host.
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len()>4 {
                return attempt.error("Too many Modrinth redirects");
            }
            let uri=attempt.url();
            if uri.scheme()=="https" &&
                matches!(uri.host_str(),Some("api.modrinth.com" | "cdn.modrinth.com")) {
                attempt.follow()
            } else {
                attempt.error("Untrusted Modrinth redirect")
            }
        }))
        .build()
        .map_err(|e| e.to_string())
}

fn validate_project_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 80 || !id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_') {
        return Err("Invalid Modrinth project ID".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn search_modrinth(query: String, version: Option<String>, project_type: String, offset: Option<u32>, loader: Option<String>, sort: Option<String>) -> Result<Vec<ModrinthHit>, String> {
    if query.chars().count() > 100 { return Err("Search is too long".into()); }
    if !["mod", "resourcepack", "shader", "modpack"].contains(&project_type.as_str()) {
        return Err("Invalid Modrinth search category".into());
    }
    let offset = offset.unwrap_or(0);
    if offset > 1000 { return Err("Search offset is too large".into()); }
    let mut facets = vec![vec![format!("project_type:{project_type}")]];
    if let Some(version) = version.filter(|s| !s.is_empty()) {
        super::minecraft::validate_version(&version)?;
        facets.push(vec![format!("versions:{version}")]);
    }
    if project_type=="mod" {
        if let Some(loader) = loader.filter(|s|!s.is_empty()) {
            if !matches!(loader.as_str(),"fabric"|"quilt"|"forge"|"neoforge") {
                return Err("Unsupported mod loader filter".into());
            }
            facets.push(vec![format!("categories:{loader}")]);
        }
    }
    let sort = sort.unwrap_or_else(|| "relevance".into());
    if !matches!(sort.as_str(),"relevance"|"downloads"|"follows"|"newest"|"updated") {
        return Err("Invalid search sort".into());
    }
    let facets = serde_json::to_string(&facets).map_err(|e| e.to_string())?;
    let offset_text = offset.to_string();
    let res = http()?.get("https://api.modrinth.com/v2/search")
        .query(&[("query", query.as_str()),("facets", facets.as_str()),("limit", "24"),
                  ("offset",offset_text.as_str()),("index",sort.as_str())])
        .send().await.map_err(|e| e.to_string())?
        .error_for_status().map_err(|e| e.to_string())?
        .json::<SearchResults>().await.map_err(|e| e.to_string())?;
    Ok(res.hits)
}

/// Project details and versions for the catalog detail drawer.
#[tauri::command]
pub async fn modrinth_project_details(project_id: String) -> Result<Value, String> {
    validate_project_id(&project_id)?;
    let client = http()?;
    let project: Value = client.get(format!("https://api.modrinth.com/v2/project/{project_id}"))
        .send().await.map_err(|e|e.to_string())?
        .error_for_status().map_err(|e|e.to_string())?
        .json().await.map_err(|e|e.to_string())?;
    let versions: Vec<Value> = client.get(format!("https://api.modrinth.com/v2/project/{project_id}/version"))
        .send().await.map_err(|e|e.to_string())?
        .error_for_status().map_err(|e|e.to_string())?
        .json().await.map_err(|e|e.to_string())?;
    Ok(serde_json::json!({"project":project,"versions":versions}))
}

/// Validate archive metadata and decompress entries to detect corruption, zip bombs
/// and executable-bearing downloads. This is NOT a malware/Java-code scanner.
fn inspect_pack_archive(bytes: &[u8]) -> Result<(), String> {
    if !bytes.starts_with(b"PK\x03\x04") {
        return Err("Downloaded pack is not a valid ZIP archive".into());
    }
    let reader = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader).map_err(|e| format!("ZIP is damaged: {e}"))?;
    if archive.len() == 0 || archive.len() > 20_000 {
        return Err("Unsafe number of ZIP entries".into());
    }
    let mut unpacked = 0u64;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).map_err(|e| e.to_string())?;
        let path = file.name().replace('\\', "/");
        if path.starts_with('/') || path.split('/').any(|part| part == "..") {
            return Err("Unsafe path inside pack ZIP".into());
        }
        if file.unix_mode().is_some_and(|mode| mode & 0o170000 == 0o120000) {
            return Err("Pack ZIP cannot contain symlinks".into());
        }
        unpacked = unpacked.checked_add(file.size()).ok_or("ZIP size overflow")?;
        if unpacked > 768 * 1024 * 1024 { return Err("Pack ZIP expands beyond safety limit".into()); }
        let lower = path.to_ascii_lowercase();
        if [".exe", ".dll", ".bat", ".cmd", ".ps1", ".vbs", ".scr", ".jar"]
            .iter().any(|suffix| lower.ends_with(suffix)) {
            return Err(format!("Pack contains executable code: {path}"));
        }
        if !file.is_dir() {
            std::io::copy(&mut file, &mut std::io::sink())
                .map_err(|e| format!("Corrupt ZIP entry: {e}"))?;
        }
    }
    Ok(())
}

fn safe_pack_filename(name: &str) -> bool {
    !name.is_empty() && name.len() <= 180 && name.to_ascii_lowercase().ends_with(".zip")
        && !name.starts_with('.') && !name.contains("..")
        && name.chars().all(|c| c.is_alphanumeric() || matches!(c, '.' | '-' | '_' | '+' | '(' | ')' | ' '))
}

fn pack_folder(kind: &str) -> Result<&'static str, String> {
    match kind {
        "resourcepack" => Ok("resourcepacks"),
        "shader" => Ok("shaderpacks"),
        _ => Err("Unsupported content pack type".into()),
    }
}

async fn install_pack(app: AppHandle, instance_id: String, project_id: String,
                      version_id: Option<String>, kind: &'static str) -> Result<String, String> {
    validate_project_id(&project_id)?;
    if let Some(ref id) = version_id { validate_project_id(id)?; }
    let instance = super::instances::read(&app, &instance_id)?;
    let client = http()?;
    let project: Value = client.get(format!("https://api.modrinth.com/v2/project/{project_id}"))
        .send().await.map_err(|e| e.to_string())?
        .error_for_status().map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;
    if project["project_type"].as_str() != Some(kind) {
        return Err("Modrinth project has a different content type".into());
    }

    let game_versions = serde_json::to_string(&vec![&instance.version]).map_err(|e| e.to_string())?;
    let versions: Vec<Value> = client.get(format!("https://api.modrinth.com/v2/project/{project_id}/version"))
        .query(&[("game_versions", game_versions.as_str()), ("include_changelog", "false")])
        .send().await.map_err(|e| e.to_string())?
        .error_for_status().map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;
    let version = match version_id.as_deref() {
        Some(id) => versions.iter().find(|v| v["id"].as_str() == Some(id))
            .ok_or("Selected release is incompatible with this Minecraft version")?,
        None => versions.first().ok_or("No compatible content release found")?,
    };
    let files = version["files"].as_array().ok_or("Modrinth release has no downloadable files")?;
    let file = files.iter().find(|f| f["primary"].as_bool() == Some(true)
        && f["filename"].as_str().is_some_and(safe_pack_filename))
        .or_else(|| files.iter().find(|f| f["filename"].as_str().is_some_and(safe_pack_filename)))
        .ok_or("No compatible ZIP download found")?;
    let filename = file["filename"].as_str().ok_or("Missing ZIP filename")?;
    let download_url = file["url"].as_str().ok_or("Missing Modrinth download URL")?;
    let url = reqwest::Url::parse(download_url).map_err(|e| e.to_string())?;
    if url.scheme() != "https" || url.host_str() != Some("cdn.modrinth.com") {
        return Err("Pack must be downloaded from the official Modrinth CDN".into());
    }
    let digest = file["hashes"]["sha512"].as_str().ok_or("Missing SHA-512 checksum")?;
    if digest.len() != 128 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Malformed SHA-512 checksum".into());
    }
    let expected_size = file["size"].as_u64().ok_or("Missing ZIP size")?;
    if expected_size == 0 || expected_size > 256 * 1024 * 1024 {
        return Err("Content pack exceeds the 256 MB safety limit".into());
    }
    let response = client.get(download_url).send().await.map_err(|e| e.to_string())?
        .error_for_status().map_err(|e| e.to_string())?;
    if response.content_length().is_some_and(|n| n > expected_size) {
        return Err("Downloaded size exceeds Modrinth metadata".into());
    }
    let mut bytes = Vec::new();
    let mut received = 0u64;
    let mut chunks = response.bytes_stream();
    while let Some(chunk) = chunks.next().await {
        let chunk = chunk.map_err(|e| format!("Download interrupted: {e}"))?;
        received = received.checked_add(chunk.len() as u64).ok_or("ZIP size overflow")?;
        if received > expected_size { return Err("Download exceeds declared size".into()); }
        bytes.extend_from_slice(&chunk);
    }
    if received != expected_size { return Err("Download truncated".into()); }
    if !format!("{:x}", Sha512::digest(&bytes)).eq_ignore_ascii_case(digest) {
        return Err("Content pack SHA-512 mismatch".into());
    }
    let bytes = tokio::task::spawn_blocking(move || {
        inspect_pack_archive(&bytes)?;
        Ok::<Vec<u8>, String>(bytes)
    }).await.map_err(|e| format!("ZIP inspection interrupted: {e}"))??;

    let dir: PathBuf = super::instances::game_directory(&app, &instance_id, &instance.version)?
        .join(pack_folder(kind)?);
    super::game_folders::prepare_folders(dir.parent().ok_or("Invalid game directory")?)
        .map_err(|e| format!("Cannot prepare Minecraft folders: {e}"))?;
    let path = dir.join(filename);
    if fs::symlink_metadata(&path).is_ok() {
        return Err("Content pack already exists; remove it before reinstalling".into());
    }
    // create_new + a randomized staging name prevents following an existing
    // .fre-download symlink and avoids overwriting another install's partial file.
    let temporary = dir.join(format!(".fre-{}-{}.download", uuid::Uuid::new_v4(), filename));
    let result = (|| -> Result<(), String> {
        use std::io::Write;
        let mut output = fs::OpenOptions::new().create_new(true).write(true)
            .open(&temporary).map_err(|e| e.to_string())?;
        output.write_all(&bytes).map_err(|e| e.to_string())?;
        output.sync_all().map_err(|e| e.to_string())?;
        drop(output);
        if fs::symlink_metadata(&path).is_ok() {
            return Err("Pack appeared during download; refusing overwrite".into());
        }
        fs::rename(&temporary, &path).map_err(|e| e.to_string())
    })();
    if result.is_err() { let _ = fs::remove_file(&temporary); }
    result?;
    Ok(format!("Installed {}: {}", kind, filename))
}

#[tauri::command]
pub async fn install_resourcepack(app: AppHandle, instance_id: String,
                                  project_id: String, version_id: Option<String>) -> Result<String, String> {
    install_pack(app, instance_id, project_id, version_id, "resourcepack").await
}

#[tauri::command]
pub async fn install_shaderpack(app: AppHandle, instance_id: String,
                                project_id: String, version_id: Option<String>) -> Result<String, String> {
    install_pack(app, instance_id, project_id, version_id, "shader").await
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledPack {
    filename: String,
    bytes: u64,
    enabled: bool,
}

fn content_dir(app: &AppHandle, instance_id: &str, kind: &str) -> Result<PathBuf, String> {
    let instance = super::instances::read(app, instance_id)?;
    let path = super::instances::game_directory(app, instance_id, &instance.version)?;
    // Never let a junction/symlink silently redirect content operations away
    // from this instance. Also materialize standard folders if missing.
    super::game_folders::prepare_folders(&path)?;
    Ok(path.join(pack_folder(kind)?))
}

fn safe_local_pack(name: &str) -> bool {
    let bare = name.strip_suffix(".disabled").unwrap_or(name);
    safe_pack_filename(bare) && (name.ends_with(".zip") || name.ends_with(".zip.disabled"))
}

#[tauri::command]
pub fn list_installed_packs(app: AppHandle, instance_id: String, kind: String)
    -> Result<Vec<InstalledPack>, String> {
    let dir = content_dir(&app, &instance_id, &kind)?;
    if !dir.exists() { return Ok(Vec::new()); }
    let mut found = Vec::new();
    for item in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let item = item.map_err(|e| e.to_string())?;
        let name = item.file_name().to_string_lossy().to_string();
        if !safe_local_pack(&name) { continue; }
        let metadata = fs::symlink_metadata(item.path()).map_err(|e| e.to_string())?;
        if metadata.file_type().is_symlink() || !metadata.is_file() { continue; }
        found.push(InstalledPack { filename: name.clone(), bytes: metadata.len(),
                                   enabled: name.ends_with(".zip") });
    }
    found.sort_by(|a,b| a.filename.to_lowercase().cmp(&b.filename.to_lowercase()));
    Ok(found)
}

#[tauri::command]
pub fn toggle_installed_pack(app: AppHandle, instance_id: String, kind: String, filename: String)
    -> Result<(), String> {
    if !safe_local_pack(&filename) { return Err("Unsafe content filename".into()); }
    let dir = content_dir(&app, &instance_id, &kind)?;
    let current = dir.join(&filename);
    let metadata = fs::symlink_metadata(&current).map_err(|e| e.to_string())?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {return Err("Invalid pack file".into());}
    let target = if filename.ends_with(".zip.disabled") {
        dir.join(filename.trim_end_matches(".disabled"))
    } else { dir.join(format!("{filename}.disabled")) };
    if fs::symlink_metadata(&target).is_ok() {return Err("Target content file already exists".into());}
    fs::rename(current,target).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn remove_installed_pack(app: AppHandle, instance_id: String, kind: String, filename: String)
    -> Result<(), String> {
    if !safe_local_pack(&filename) {return Err("Unsafe content filename".into());}
    let dir = content_dir(&app, &instance_id, &kind)?;
    let path = dir.join(filename);
    let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {return Err("Invalid pack file".into());}
    fs::remove_file(path).map_err(|e|e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn protects_pack_filenames() {
        assert!(safe_pack_filename("ComplementaryShaders_v5.2.zip"));
        assert!(safe_pack_filename("мій-ресурспак.zip"));
        assert!(!safe_pack_filename("../evil.zip"));
        assert!(!safe_pack_filename("evil\\name.zip"));
        assert!(!safe_pack_filename("pack.jar"));
        assert!(!safe_local_pack("pack.zip/../../"));
        assert!(safe_local_pack("pack.zip.disabled"));
    }
    #[test]
    fn rejects_executable_zip_entry() {
        use std::io::Write;
        let buffer = std::io::Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(buffer);
        writer.start_file("pack.mcmeta", zip::write::SimpleFileOptions::default()).unwrap();
        writer.write_all(b"{}").unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        assert!(inspect_pack_archive(&bytes).is_ok());

        let buffer = std::io::Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(buffer);
        writer.start_file("payload.exe", zip::write::SimpleFileOptions::default()).unwrap();
        writer.write_all(b"unsafe").unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        assert!(inspect_pack_archive(&bytes).is_err());
    }
    #[test]
    fn ids_are_sanitized() {
        assert!(validate_project_id("sodium").is_ok());
        assert!(validate_project_id("../evil").is_err());
    }
}
