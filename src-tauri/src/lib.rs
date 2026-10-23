mod minecraft;
mod downloads;
mod fabric;
mod servers;
mod mods;
mod instances;
mod modrinth;
mod backups;
mod accounts;
mod java;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Serialize, Deserialize)]
pub struct MinecraftVersion {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    pub time: String,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}
#[derive(Debug, Deserialize)]
struct VersionManifest { versions: Vec<MinecraftVersion> }

#[tauri::command]
async fn get_versions() -> Result<Vec<MinecraftVersion>, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build().map_err(|e| e.to_string())?;
    let manifest = client.get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
        .send().await.map_err(|e| format!("Network error: {e}"))?
        .error_for_status().map_err(|e| format!("HTTP error: {e}"))?
        .json::<VersionManifest>().await.map_err(|e| format!("Invalid version manifest: {e}"))?;
    Ok(manifest.versions)
}

#[tauri::command]
fn detect_java() -> Result<String, String> {
    let output = Command::new("java").arg("-version").output()
        .map_err(|e| format!("Java not found on PATH: {e}"))?;
    let version = String::from_utf8_lossy(&output.stderr).to_string();
    if output.status.success() { Ok(version.lines().next().unwrap_or("Java detected").to_string()) }
    else { Err(format!("Java returned exit code {:?}", output.status.code())) }
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_versions, detect_java, minecraft::java_info, minecraft::install_version, minecraft::is_installed, minecraft::launch_game, instances::list_instances, instances::create_instance, instances::update_instance, instances::delete_instance, modrinth::search_modrinth, modrinth::install_resourcepack, backups::create_world_backup, backups::list_world_backups, backups::restore_world_backup, accounts::list_offline_accounts, accounts::create_offline_account, accounts::delete_offline_account, java::list_java_runtimes, fabric::list_fabric_loaders, instances::set_instance_loader, servers::list_servers, servers::add_server, servers::delete_server, servers::ping_server, mods::install_fabric_mod, mods::list_installed_mods, mods::toggle_installed_mod, mods::remove_installed_mod])
        .run(tauri::generate_context!())
        .expect("error while running FRE LAUNCH");
}
