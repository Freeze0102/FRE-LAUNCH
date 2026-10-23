//! Fabric Loader metadata integration. Never runs an untrusted installer JAR.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;
use tokio::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct FabricLoader {
    pub version: String,
    pub stable: bool,
}

#[derive(Deserialize)]
struct FabricGameLoader {
    loader: FabricLoader,
}

#[tauri::command]
pub async fn list_fabric_loaders(game_version: String) -> Result<Vec<FabricLoader>, String> {
    super::minecraft::validate_version(&game_version)?;
    let client = super::downloads::client()?;
    let url = format!("https://meta.fabricmc.net/v2/versions/loader/{game_version}");
    let results: Vec<FabricGameLoader> = client.get(&url).send().await.map_err(|e|e.to_string())?
        .error_for_status().map_err(|e|format!("Fabric API: {e}"))?
        .json().await.map_err(|e|e.to_string())?;
    Ok(results.into_iter().map(|row|row.loader).collect())
}

pub async fn load_profile(base: &Path, game_version: &str, loader_version: &str) -> Result<Value, String> {
    super::minecraft::validate_version(game_version)?;
    super::minecraft::validate_version(loader_version)?;
    let folder = base.join("fabric").join(game_version);
    let path = folder.join(format!("{loader_version}.json"));
    if fs::try_exists(&path).await.map_err(|e|e.to_string())? {
        let bytes = fs::read(&path).await.map_err(|e|e.to_string())?;
        let profile: Value = serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;
        validate_profile(&profile, game_version)?;
        return Ok(profile);
    }
    let http = super::downloads::client()?;
    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{game_version}/{loader_version}/profile/json"
    );
    let profile: Value = http.get(&url).send().await.map_err(|e|format!("Fabric metadata: {e}"))?
        .error_for_status().map_err(|e|format!("Fabric profile: {e}"))?
        .json().await.map_err(|e|e.to_string())?;
    validate_profile(&profile, game_version)?;
    fs::create_dir_all(&folder).await.map_err(|e|e.to_string())?;
    let temp = folder.join(format!("{loader_version}.temporary"));
    fs::write(&temp,serde_json::to_vec_pretty(&profile).map_err(|e|e.to_string())?)
        .await.map_err(|e|e.to_string())?;
    fs::rename(&temp,&path).await.map_err(|e|e.to_string())?;
    Ok(profile)
}

fn validate_profile(profile: &Value, game_version: &str) -> Result<(), String> {
    if profile["inheritsFrom"].as_str() != Some(game_version) {
        return Err("Fabric profile targets a different Minecraft version".into());
    }
    let main = profile["mainClass"].as_str().ok_or("Fabric profile lacks mainClass")?;
    if main != "net.fabricmc.loader.impl.launch.knot.KnotClient"
        && main != "net.fabricmc.loader.launch.knot.KnotClient" {
        return Err("Unsupported Fabric client main class".into());
    }
    let libs = profile["libraries"].as_array().ok_or("Fabric profile missing libraries")?;
    if libs.is_empty() || libs.len() > 100 {return Err("Invalid Fabric library list".into());}
    Ok(())
}

pub fn merge_profile(base: &mut Value, profile: &Value) -> Result<(), String> {
    let main_class = profile["mainClass"].as_str().ok_or("Fabric client main class missing")?;
    let extra = profile["libraries"].as_array().ok_or("Fabric libraries missing")?;
    let existing = base["libraries"].as_array_mut().ok_or("Vanilla libraries missing")?;
    let mut newlibs = extra.clone();
    newlibs.extend(existing.iter().cloned());
    *existing = newlibs;
    base["mainClass"] = Value::String(main_class.to_owned());
    for key in ["game", "jvm"] {
        if let Some(extra_args) = profile["arguments"][key].as_array() {
            let base_args = base["arguments"][key].as_array().cloned().unwrap_or_default();
            let mut combined = extra_args.clone();
            combined.extend(base_args);
            base["arguments"][key] = Value::Array(combined);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_mismatched_profile() {
        let value = serde_json::json!({
            "inheritsFrom":"1.20.1",
            "mainClass":"net.fabricmc.loader.impl.launch.knot.KnotClient",
            "libraries":[{"name":"net.fabricmc:fabric-loader:0.16.0"}]
        });
        assert!(validate_profile(&value,"1.21.4").is_err());
        assert!(validate_profile(&value,"1.20.1").is_ok());
    }
    #[test]
    fn merges_fabric_libraries_first() {
        let mut vanilla = serde_json::json!({
            "mainClass":"net.minecraft.client.main.Main",
            "libraries":[{"name":"vanilla:core:1"}],
            "arguments":{"game":["--username","Player"],"jvm":["-cp","${classpath}"]}
        });
        let profile=serde_json::json!({
            "libraries":[{"name":"net.fabricmc:fabric-loader:1.0"}],
            "mainClass":"net.fabricmc.loader.impl.launch.knot.KnotClient"
        });
        merge_profile(&mut vanilla,&profile).unwrap();
        assert_eq!(vanilla["libraries"][0]["name"],"net.fabricmc:fabric-loader:1.0");
        assert_eq!(vanilla["mainClass"],"net.fabricmc.loader.impl.launch.knot.KnotClient");
    }
}
