mod minecraft;
mod startup;
mod updates;
pub use startup::prepare_windows_webview;
mod diagnostics;
mod windows_crash;
mod preflight;
mod downloads;
mod fabric;
mod servers;
mod mods;
mod managed_java;
mod screenshots;
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
    startup::log_bootstrap("VERSIONS_START", "Fetching Mojang version metadata");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build().map_err(|e| e.to_string())?;
    let manifest = client.get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
        .send().await.map_err(|e| format!("Network error: {e}"))?
        .error_for_status().map_err(|e| format!("HTTP error: {e}"))?
        .json::<VersionManifest>().await.map_err(|e| format!("Invalid version manifest: {e}"))?;
    startup::log_bootstrap("VERSIONS_DONE", &format!("Loaded {} versions", manifest.versions.len()));
    Ok(manifest.versions)
}

#[tauri::command]
fn detect_java() -> Result<String, String> {
    startup::log_bootstrap("JAVA_DETECT_START", "Looking for Java on PATH");
    let output = Command::new("java").arg("-version").output()
        .map_err(|e| format!("Java not found on PATH: {e}"))?;
    let version = String::from_utf8_lossy(&output.stderr).to_string();
    if output.status.success() { Ok(version.lines().next().unwrap_or("Java detected").to_string()) }
    else { Err(format!("Java returned exit code {:?}", output.status.code())) }
}

pub fn run() {
    let result = tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();
            // Ensure the diagnostics page has a real startup breadcrumb even when a
            // previous installation stopped before the Rust install command began.
            diagnostics::write(&handle, "APP_START", "FRE LAUNCH process started");
            startup::log_bootstrap("TAURI_SETUP", "Tauri initialized main window");
            if startup::software_rendering() {
                diagnostics::write(&handle,"RECOVERY_MODE",
                    "Previous startup ended too early; WebView2 GPU acceleration is disabled");
            }
            let previous_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |panic| {
                // Log Rust panics that happen outside the Minecraft installer too.
                diagnostics::write(&handle, "RUST_PANIC", &panic.to_string());
                previous_hook(panic);
            }));
            Ok(())
        })
        .on_window_event(|_window,event|{
            if matches!(event,tauri::WindowEvent::CloseRequested { .. }) {
                startup::clear_boot_marker_for_expected_exit();
            }
        })
        .invoke_handler(tauri::generate_handler![startup::mark_frontend_ready, startup::mark_frontend_mounted, startup::software_rendering, startup::disable_software_recovery, updates::check_launcher_update, updates::install_launcher_update, get_versions, detect_java, minecraft::java_info, minecraft::install_version, diagnostics::read_diagnostics, diagnostics::record_interrupted_install, windows_crash::read_windows_crash_report, preflight::install_preflight, minecraft::is_installed, minecraft::launch_game, instances::list_instances, instances::create_instance, instances::update_instance, instances::delete_instance, modrinth::search_modrinth, modrinth::install_resourcepack, backups::create_world_backup, backups::list_world_backups, backups::restore_world_backup, accounts::list_offline_accounts, accounts::create_offline_account, accounts::delete_offline_account, java::list_java_runtimes, fabric::list_fabric_loaders, instances::set_instance_loader, servers::list_servers, servers::save_server, servers::delete_server, servers::check_server_status, mods::install_fabric_mod, mods::list_installed_mods, mods::toggle_installed_mod, mods::remove_installed_mod, managed_java::install_managed_java, screenshots::list_screenshots, screenshots::read_screenshot, screenshots::delete_screenshot])
        .run(tauri::generate_context!());
    match &result {
        Ok(()) => startup::log_bootstrap("PROCESS_EXIT", "Tauri event loop finished"),
        Err(error) => startup::log_bootstrap("TAURI_ERROR", &error.to_string()),
    }
    result.expect("error while running FRE LAUNCH");
}
