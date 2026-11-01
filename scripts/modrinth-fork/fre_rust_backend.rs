//! FRE LAUNCH built-in Rust Minecraft profile manager.
//! No Horizon Launcher process, filesystem dependency or API. The native
//! client is bundled from openly licensed Rust sources at local build time.
//! Private Minecraft game resources are supplied only from the owner's copy.
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::Manager;
use tokio::io::AsyncWriteExt;
use theseus::{State, state::Credentials};

const MAX_NAME: usize = 64;
const MAX_METADATA: u64 = 8192;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RustProfile {
    id: String,
    name: String,
    created: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RustRuntime {
    installed: bool,
    prepared: bool,
    version: &'static str,
    reason: Option<String>,
}

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("fre-rust")
        .invoke_handler(tauri::generate_handler![
            rust_profiles,
            rust_create_profile,
            rust_runtime,
            rust_launch,
            rust_logs,
        ])
        .build()
}

fn base<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|p| p.join("fre-rust-minecraft"))
        .map_err(|e| format!("Cannot resolve FRE LAUNCH data directory: {e}"))
}

fn bundle<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    app.path()
        .resource_dir()
        .map(|p| p.join("fre-rust"))
        .map_err(|e| format!("Cannot resolve FRE LAUNCH bundled files: {e}"))
}

fn profile_id_valid(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 40
        && id.bytes().all(|c| c.is_ascii_digit())
}
fn name_valid(name: &str) -> bool {
    !name.trim().is_empty()
        && name.chars().count() <= MAX_NAME
        && !name.chars().any(char::is_control)
}

fn profile_dir(root: &Path, id: &str) -> Result<PathBuf, String> {
    if !profile_id_valid(id) {
        return Err("Invalid Rust Minecraft profile identifier.".into());
    }
    Ok(root.join("profiles").join(id))
}

fn read_profile(dir: &Path, id: &str) -> Option<RustProfile> {
    if !profile_id_valid(id)
        || dir.symlink_metadata().ok()?.file_type().is_symlink()
        || !dir.is_dir()
    {
        return None;
    }
    let metadata = dir.join("profile.json");
    if metadata.metadata().ok()?.len() > MAX_METADATA {
        return None;
    }
    let profile: RustProfile = serde_json::from_slice(&fs::read(metadata).ok()?).ok()?;
    (profile.id == id && name_valid(&profile.name)).then_some(profile)
}

fn list(root: &Path) -> Result<Vec<RustProfile>, String> {
    let mut profiles = Vec::new();
    let dir = root.join("profiles");
    if !dir.exists() {
        return Ok(profiles);
    }
    for entry in fs::read_dir(&dir).map_err(|e| format!("Cannot read Rust library: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let id = entry.file_name().to_string_lossy().into_owned();
        if let Some(profile) = read_profile(&entry.path(), &id) {
            profiles.push(profile);
        }
    }
    profiles.sort_by(|a, b| b.created.cmp(&a.created).then_with(|| a.name.cmp(&b.name)));
    Ok(profiles)
}

#[tauri::command]
async fn rust_profiles<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Vec<RustProfile>, String> {
    list(&base(&app)?)
}

#[tauri::command]
async fn rust_create_profile<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    name: String,
) -> Result<RustProfile, String> {
    let name = name.trim().to_owned();
    if !name_valid(&name) {
        return Err("Name must contain 1–64 characters without control characters.".into());
    }
    let root = base(&app)?;
    if list(&root)?.iter().any(|p| p.name.eq_ignore_ascii_case(&name)) {
        return Err("A Rust profile with this name already exists.".into());
    }
    let profiles_dir = root.join("profiles");
    fs::create_dir_all(&profiles_dir).map_err(|e| e.to_string())?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?;
    let created = stamp.as_secs();
    // Numeric identifiers are not derived from user input and cannot escape
    // the profile directory. create_dir() atomically prevents collisions.
    for attempt in 0..16u128 {
        let id = (stamp.as_nanos() + attempt).to_string();
        let dir = profile_dir(&root, &id)?;
        match fs::create_dir(&dir) {
            Ok(()) => {
                let profile = RustProfile { id, name, created };
                let metadata = serde_json::to_vec_pretty(&profile).map_err(|e| e.to_string())?;
                let path = dir.join("profile.json");
                let written = fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&path)
                    .and_then(|mut file| std::io::Write::write_all(&mut file, &metadata));
                if let Err(error) = written {
                    let _ = fs::remove_dir_all(&dir); // Only our newly created directory.
                    return Err(format!("Could not write profile metadata: {error}"));
                }
                return Ok(profile);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("Cannot create Rust profile folder: {error}")),
        }
    }
    Err("Could not allocate a new Rust profile identifier.".into())
}

fn runtime_files(root: &Path) -> (PathBuf, PathBuf) {
    (root.join("fre-rust-client.exe"), root.join("resources"))
}

fn ready(root: &Path) -> RustRuntime {
    let (client, resources) = runtime_files(root);
    let installed = client.is_file();
    let prepared = resources.join("horizon-resources.json").is_file();
    let reason = if !installed {
        Some("The FRE LAUNCH Rust game was not included in this installer. Build and bundle the native client first.".into())
    } else if !prepared {
        Some("Minecraft game resources are not prepared for this Rust build. Use your legally owned Minecraft installation.".into())
    } else {
        None
    };
    RustRuntime {
        installed,
        prepared,
        version: "Minecraft 26.3",
        reason,
    }
}

#[tauri::command]
async fn rust_runtime<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<RustRuntime, String> {
    Ok(ready(&bundle(&app)?))
}

#[tauri::command]
async fn rust_launch<R: tauri::Runtime>(
    app: tauri::AppHandle<R>, profile_id: String,
) -> Result<String, String> {
    let root = base(&app)?;
    let dir = profile_dir(&root, &profile_id)?;
    let profile = read_profile(&dir, &profile_id)
        .ok_or("Rust profile not found. Refresh the library.")?;
    let runtime_dir = bundle(&app)?;
    let runtime = ready(&runtime_dir);
    if let Some(message) = runtime.reason {
        return Err(message);
    }
    let (client, resources) = runtime_files(&runtime_dir);
    let client = client.canonicalize().map_err(|e| e.to_string())?;
    let resources = resources.canonicalize().map_err(|e| e.to_string())?;
    // The native client's Launch API requires an authenticated Microsoft
    // owner session (docs/api/launch.md). Reuse the active FRE LAUNCH
    // Minecraft account, never launch an online-mode client using a fake
    // nickname or an expired guest token.
    let state = State::get().await
        .map_err(|e| format!("Minecraft account state unavailable: {e}"))?;
    let credentials = Credentials::get_default_credential(&state.pool).await
        .map_err(|e| format!("Could not refresh Minecraft account: {e}"))?
        .ok_or("Sign in with the Microsoft account that owns Minecraft: Java Edition before playing Rust Minecraft.")?;
    if credentials.access_token.trim().is_empty()
        || credentials.access_token.starts_with("fre-offline:")
        || credentials.offline_profile.id.is_nil()
    {
        return Err("Rust Minecraft requires an authenticated Microsoft Java Edition account.".into());
    }
    let verified = credentials.online_profile_fresh().await
        .ok_or("Minecraft Java Edition ownership could not be verified. Reconnect your Microsoft account and retry.")?;
    let session = serde_json::json!({
        "name": verified.name,
        "uuid": verified.id.simple().to_string(),
        "access_token": credentials.access_token,
        "user_type": "msa",
    });
    let session_bytes = serde_json::to_vec(&session)
        .map_err(|_| "Could not prepare the Minecraft session.".to_owned())?;

    // The engine reads --home, --game-dir, --resources, and
    // --session-stdin directly (Horizon native Launch API 0.1).
    // Account tokens only travel through the child's private stdin pipe,
    // never in command-line arguments, logs or profile metadata.
    let mut cmd = tokio::process::Command::new(client);
    cmd.current_dir(&dir)
        .arg("--game-dir").arg(&dir)
        .arg("--home").arg(&dir)
        .arg("--resources").arg(resources)
        .arg("--session-stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(false);
    #[cfg(windows)]
    cmd.creation_flags(0x08000000);
    let mut child = cmd.spawn().map_err(|e| format!("Could not start Rust Minecraft: {e}"))?;
    let mut stdin = child.stdin.take().ok_or("Could not open the Minecraft session pipe.")?;
    if let Err(err) = stdin.write_all(&session_bytes).await {
        let _ = child.kill().await;
        return Err(format!("Could not securely hand the Minecraft account to the Rust game: {err}"));
    }
    drop(stdin);
    let pid = child.id().unwrap_or_default();
    let mut task = tokio::spawn(async move { child.wait().await });
    match tokio::time::timeout(Duration::from_millis(1200), &mut task).await {
        Ok(result) => {
            let status = result.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
            if !status.success() {
                Err(format!("Rust Minecraft exited immediately ({status}). See the profile log."))
            } else {
                Ok(format!("Rust Minecraft exited normally: {}.", profile.name))
            }
        }
        Err(_) => Ok(format!("Started FRE LAUNCH Rust Minecraft for {} (PID {pid}).", profile.name)),
    }
}

#[tauri::command]
async fn rust_logs<R: tauri::Runtime>(
    app: tauri::AppHandle<R>, profile_id: String,
) -> Result<String, String> {
    let root = base(&app)?;
    let dir = profile_dir(&root, &profile_id)?;
    if read_profile(&dir, &profile_id).is_none() {
        return Err("Rust profile not found.".into());
    }
    let logfile = dir.join("logs").join("latest.log");
    let metadata = logfile.metadata().map_err(|e| format!("No local game log found: {e}"))?;
    if metadata.len() > 32 * 1024 * 1024 {
        return Err("Game log is too large to display. Open the log file locally.".into());
    }
    let content = fs::read_to_string(&logfile).map_err(|e| e.to_string())?;
    Ok(content.chars().rev().take(12_000).collect::<String>().chars().rev().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_path_traversal_and_invalid_profiles() {
        assert!(!profile_id_valid("../save"));
        assert!(!profile_id_valid("a/123"));
        assert!(profile_id_valid("123456"));
        assert!(!name_valid(""));
        assert!(!name_valid("bad\nname"));
        assert!(name_valid("Мій Minecraft Rust"));
    }
    #[test]
    fn missing_game_is_not_reported_as_ready() {
        let folder = tempfile::tempdir().unwrap();
        let info = ready(folder.path());
        assert!(!info.installed);
        assert!(!info.prepared);
        assert!(info.reason.is_some());
    }
}
