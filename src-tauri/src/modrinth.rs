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
}

#[derive(Deserialize)]
struct SearchResults { hits: Vec<ModrinthHit> }

fn http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("FRE-LAUNCH/0.3 (https://github.com/Freeze0102/FRE-LAUNCH)")
        .timeout(std::time::Duration::from_secs(35))
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
pub async fn search_modrinth(query: String, version: Option<String>, project_type: String, offset: Option<u32>) -> Result<Vec<ModrinthHit>, String> {
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
    let facets = serde_json::to_string(&facets).map_err(|e| e.to_string())?;
    let offset_text = offset.to_string();
    let res = http()?.get("https://api.modrinth.com/v2/search")
        .query(&[("query", query.as_str()),("facets", facets.as_str()),("limit", "24"),("offset",offset_text.as_str())])
        .send().await.map_err(|e| e.to_string())?
        .error_for_status().map_err(|e| e.to_string())?
        .json::<SearchResults>().await.map_err(|e| e.to_string())?;
    Ok(res.hits)
}

/// Only install verified Modrinth ZIP resource packs, never execute third-party code.
#[tauri::command]
pub async fn install_resourcepack(app: AppHandle, instance_id: String, project_id: String) -> Result<String, String> {
    validate_project_id(&project_id)?;
    let instance = super::instances::read(&app, &instance_id)?;
    let http = http()?;
    let project_url = format!("https://api.modrinth.com/v2/project/{project_id}");
    let project: Value = http.get(&project_url).send().await.map_err(|e| e.to_string())?
        .error_for_status().map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;
    if project["project_type"].as_str() != Some("resourcepack") {
        return Err("Only resource packs can be installed by this feature".into());
    }
    let url = format!("https://api.modrinth.com/v2/project/{project_id}/version");
    let compatible = serde_json::to_string(&vec![&instance.version]).map_err(|e| e.to_string())?;
    let versions: Vec<Value> = http.get(&url)
        .query(&[("game_versions", compatible.as_str()),("include_changelog","false")])
        .send().await.map_err(|e| e.to_string())?
        .error_for_status().map_err(|e| e.to_string())?
        .json().await.map_err(|e| e.to_string())?;
    let version = versions.first().ok_or("No compatible resource pack found")?;
    let files = version["files"].as_array().ok_or("Missing resource pack files")?;
    let file = files.iter().find(|f| f["primary"].as_bool() == Some(true)).or_else(|| files.first()).ok_or("No downloadable file")?;
    let filename = file["filename"].as_str().ok_or("Invalid file name")?;
    if filename.len() > 160 || !filename.to_lowercase().ends_with(".zip")
        || filename.contains('/') || filename.contains('\\')
        || filename == "." || filename == ".." {
        return Err("Unsafe resource pack filename".into());
    }
    let url = file["url"].as_str().ok_or("Missing download URL")?;
    let url_parsed = reqwest::Url::parse(url).map_err(|e| e.to_string())?;
    if url_parsed.scheme() != "https" || url_parsed.host_str() != Some("cdn.modrinth.com") {
        return Err("Resource pack files must come from the official Modrinth CDN".into());
    }
    let expected = file["hashes"]["sha512"].as_str().ok_or("Missing SHA-512 hash")?;
    let size = file["size"].as_u64().ok_or("Missing resource pack size")?;
    if size == 0 || size > 256 * 1024 * 1024 { return Err("Resource pack exceeds the 256 MB safety limit or is empty".into()); }
    let res = http.get(url).send().await.map_err(|e| e.to_string())?
        .error_for_status().map_err(|e| e.to_string())?;
    // A missing Content-Length must not allow unlimited allocations.
    let mut bytes = Vec::new();
    let mut received = 0u64;
    let mut chunks = res.bytes_stream();
    while let Some(chunk) = chunks.next().await {
        let chunk=chunk.map_err(|e|format!("Resource pack stream interrupted: {e}"))?;
        received=received.checked_add(chunk.len() as u64).ok_or("Resource pack size overflow")?;
        if received>size {
            return Err("Resource pack download exceeded the declared size".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    if received!=size {return Err("Downloaded size does not match Modrinth metadata".into());}
    if format!("{:x}", Sha512::digest(&bytes)) != expected.to_ascii_lowercase() {
        return Err("Resource pack hash validation failed".into());
    }
    let dir: PathBuf = super::instances::game_directory(&app, &instance_id, &instance.version)?
        .join("resourcepacks");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(filename);
    if path.exists() { return Err("Resource pack already exists; remove it before reinstalling".into()); }
    let temporary = dir.join(format!("{filename}.fre-download"));
    fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
    fs::rename(&temporary, &path).map_err(|e| e.to_string())?;
    Ok(format!("Installed resource pack: {filename}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_are_sanitized() {
        assert!(validate_project_id("sodium").is_ok());
        assert!(validate_project_id("../evil").is_err());
    }
}
