//! Quilt Loader metadata integration: only signed-up official Quilt Meta/Maven hosts.
//! Launcher profiles are JSON data, not executable installation scripts.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use tokio::fs;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuiltLoader {
    pub version: String,
    #[serde(default)]
    pub stable: bool,
}
#[derive(Debug, Deserialize)]
struct QuiltGameLoader {
    loader: QuiltLoader,
}

/// Query compatible Quilt Loader versions from the official v3 Quilt Meta API.
#[tauri::command]
pub async fn list_quilt_loaders(game_version: String) -> Result<Vec<QuiltLoader>, String> {
    super::minecraft::validate_version(&game_version)?;
    let client = super::downloads::client()?;
    let url = format!("https://meta.quiltmc.org/v3/versions/loader/{game_version}");
    let rows: Vec<QuiltGameLoader> = client.get(url).send().await
        .map_err(|e|format!("Quilt Meta: {e}"))?
        .error_for_status().map_err(|e|format!("Quilt Meta HTTP: {e}"))?
        .json().await.map_err(|e|format!("Quilt Meta JSON: {e}"))?;
    Ok(rows.into_iter().map(|x|x.loader).collect())
}

fn normalize_libraries(profile: &mut Value) -> Result<(), String> {
    let libs = profile["libraries"].as_array_mut().ok_or("Quilt profile has no libraries")?;
    if libs.is_empty() || libs.len()>180 {return Err("Invalid Quilt library list size".into());}
    for lib in libs {
        if let Some(coordinate) = lib["name"].as_str().map(str::to_owned) {
            if let Some(version) = coordinate.strip_prefix("org.quiltmc:hashed:") {
                // Quilt's hashed Minecraft mapping resolves to Fabric intermediary.
                super::minecraft::validate_version(version)?;
                lib["name"] = Value::String(format!("net.fabricmc:intermediary:{version}"));
                lib["url"] = Value::String("https://maven.fabricmc.net/".into());
            }
        }
    }
    Ok(())
}

fn validate_profile(profile: &Value, game_version: &str) -> Result<(), String> {
    if profile["inheritsFrom"].as_str()!=Some(game_version) {
        return Err("Quilt profile does not match the selected Minecraft version".into());
    }
    let main = profile["mainClass"].as_str().ok_or("Quilt mainClass is missing")?;
    if main!="org.quiltmc.loader.impl.launch.knot.KnotClient" &&
       main!="org.quiltmc.loader.launch.knot.KnotClient" {
        return Err(format!("Unsupported Quilt client mainClass: {main}"));
    }
    let libs=profile["libraries"].as_array().ok_or("Quilt libraries missing")?;
    if libs.is_empty()||libs.len()>180 {return Err("Invalid Quilt library list".into());}
    for lib in libs {
        let coordinate=lib["name"].as_str().ok_or("Missing Quilt library coordinate")?;
        let segments:Vec<&str>=coordinate.split(':').collect();
        if segments.len()<3 || segments.iter().any(|v|v.is_empty()) {
            return Err("Invalid Quilt Maven coordinate".into());
        }
        if let Some(repository)=lib["url"].as_str() {
            super::downloads::trusted_url(repository)?;
        }
    }
    Ok(())
}

pub async fn load_profile(base: &Path, game_version: &str, loader_version: &str) -> Result<Value, String> {
    super::minecraft::validate_version(game_version)?;
    super::minecraft::validate_version(loader_version)?;
    let folder=base.join("quilt").join(game_version);
    let target=folder.join(format!("{loader_version}.json"));
    if fs::try_exists(&target).await.map_err(|e|e.to_string())? {
        let bytes=fs::read(&target).await.map_err(|e|e.to_string())?;
        let cached:Value=serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;
        validate_profile(&cached,game_version)?;
        return Ok(cached);
    }
    let client=super::downloads::client()?;
    let url=format!("https://meta.quiltmc.org/v3/versions/loader/{game_version}/{loader_version}/profile/json");
    let mut profile:Value=client.get(&url).send().await.map_err(|e|format!("Quilt profile request: {e}"))?
        .error_for_status().map_err(|e|format!("Quilt profile response: {e}"))?
        .json().await.map_err(|e|format!("Quilt profile JSON: {e}"))?;
    normalize_libraries(&mut profile)?;
    validate_profile(&profile,game_version)?;
    fs::create_dir_all(&folder).await.map_err(|e|e.to_string())?;
    let temporary=folder.join(format!("{loader_version}.temporary"));
    fs::write(&temporary,serde_json::to_vec_pretty(&profile).map_err(|e|e.to_string())?)
        .await.map_err(|e|e.to_string())?;
    fs::rename(&temporary,&target).await.map_err(|e|e.to_string())?;
    Ok(profile)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_cross_version_profile() {
        let value=serde_json::json!({
            "inheritsFrom":"1.21.1",
            "mainClass":"org.quiltmc.loader.impl.launch.knot.KnotClient",
            "libraries":[{"name":"org.quiltmc:quilt-loader:0.27.0","url":"https://maven.quiltmc.org/repository/release/"}]
        });
        assert!(validate_profile(&value,"1.21.1").is_ok());
        assert!(validate_profile(&value,"1.21.4").is_err());
    }
    #[test]
    fn maps_hashed_minecraft_library_to_fabric_intermediary() {
        let mut value=serde_json::json!({"libraries":[{"name":"org.quiltmc:hashed:1.21.1"}]});
        normalize_libraries(&mut value).unwrap();
        assert_eq!(value["libraries"][0]["name"],"net.fabricmc:intermediary:1.21.1");
        assert_eq!(value["libraries"][0]["url"],"https://maven.fabricmc.net/");
    }
    #[test]
    fn rejects_untrusted_quilt_maven_domain() {
        let value=serde_json::json!({
            "inheritsFrom":"1.21.1",
            "mainClass":"org.quiltmc.loader.impl.launch.knot.KnotClient",
            "libraries":[{"name":"org.quiltmc:quilt-loader:0.27.0","url":"https://evil.example/maven/"}]
        });
        assert!(validate_profile(&value,"1.21.1").is_err());
    }
}
