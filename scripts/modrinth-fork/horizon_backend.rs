//! FRE LAUNCH delegates Horizon operations to the user-selected official CLI.
use std::{path::{Path, PathBuf}, process::{Output, Stdio}, time::Duration};
use tokio::{io::{AsyncRead, AsyncReadExt}, process::Command};
use tauri::{Emitter, Manager};

#[path = "horizon_bootstrap.rs"]
mod bootstrap;

const OUTPUT_LIMIT: usize = 24_000;
const QUICK_TIMEOUT: Duration = Duration::from_secs(60);
const INSTALL_TIMEOUT: Duration = Duration::from_secs(30 * 60);
static OPERATION: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("fre-horizon")
        .invoke_handler(tauri::generate_handler![
            horizon_instances, horizon_create, horizon_play, horizon_logs,
            horizon_versions, horizon_install_version, horizon_set_version, horizon_prepare,
            horizon_install_launcher, horizon_detect_launcher, horizon_status, horizon_open_launcher,
            horizon_direct_play, horizon_open_folder, horizon_list_files,
        ])
        .build()
}

fn launcher_directory<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> Result<PathBuf, String> {
    Ok(app.path().app_data_dir().map_err(|e| e.to_string())?
        .join("horizon-launcher").join(bootstrap::VERSION))
}

#[tauri::command]
async fn horizon_detect_launcher<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<Option<String>, String> {
    let directory = launcher_directory(&app)?;
    Ok(bootstrap::cached(&directory).map(|p| p.to_string_lossy().into_owned()))
}

#[tauri::command]
async fn horizon_install_launcher<R: tauri::Runtime>(app: tauri::AppHandle<R>) -> Result<String, String> {
    if !cfg!(all(windows, target_arch = "x86_64")) { return Err("Horizon automatic setup supports Windows x64.".into()); }
    let _operation = OPERATION.try_lock().map_err(|_| "Another Horizon operation is still running.")?;
    bootstrap::install(&launcher_directory(&app)?, |percent| {
        let _ = app.emit("fre-horizon-bootstrap-progress", percent);
    }).await.map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
async fn horizon_open_launcher(exe_path: String) -> Result<(), String> {
    let exe = validate_launcher(&exe_path)?;
    let mut child = command(&exe).stdout(Stdio::null()).stderr(Stdio::null())
        .spawn().map_err(|e| e.to_string())?;
    tokio::spawn(async move { let _ = child.wait().await; });
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct HorizonInstance {
    id: String,
    name: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    selected: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct HorizonStatus {
    instances: Vec<HorizonInstance>,
    installed_versions: Vec<String>,
    prepared_versions: Vec<String>,
    selected_version: Option<String>,
}

fn parse_status(instances: &str, versions: &str, detail: &str) -> Result<HorizonStatus, String> {
    let rows: Vec<serde_json::Value> = serde_json::from_str(instances)
        .map_err(|_| "Horizon did not return an instance list. Use launcher 0.2.33 or newer.")?;
    let mut parsed = Vec::new();
    for row in rows {
        // Only the standard and independent Rust profiles have standalone
        // directories. Java and pack/port profiles need different launch paths.
        if !matches!(row.get("kind").and_then(|v| v.as_str()), Some("default" | "own"))
            || !row["java"].is_null() { continue; }
        let instance: HorizonInstance = serde_json::from_value(row).map_err(|e| e.to_string())?;
        validate_name(&instance.id)?;
        parsed.push(instance);
    }
    #[derive(serde::Deserialize)]
    struct Versions { installed: Vec<String>, selected: Option<String> }
    let versions: Versions = serde_json::from_str(versions).map_err(|e| e.to_string())?;
    let prepared = versions.installed.iter().filter(|tag| detail.lines().any(|line| {
        let line = line.trim_start().trim_start_matches('*').trim_start();
        line.strip_prefix(tag.as_str()).is_some_and(|tail| tail.starts_with("  prepared"))
    })).cloned().collect();
    Ok(HorizonStatus { instances: parsed, installed_versions: versions.installed,
        prepared_versions: prepared, selected_version: versions.selected })
}

#[tauri::command]
async fn horizon_status(exe_path: String) -> Result<HorizonStatus, String> {
    let exe = validate_launcher(&exe_path)?;
    let instances = run_cli(&exe, &["instance", "list", "--json"], QUICK_TIMEOUT).await?;
    let versions = run_cli(&exe, &["versions", "--installed", "--json"], QUICK_TIMEOUT).await?;
    let detail = run_cli(&exe, &["versions", "--installed"], QUICK_TIMEOUT).await?;
    parse_status(&instances, &versions, &detail)
}

// Return the selected Horizon instance's real game folder, never a Java
// instance's mods directory. Horizon itself supplies its source of truth.
#[tauri::command]
async fn horizon_open_folder(
    exe_path: String,
    name: String,
    folder: String,
) -> Result<(), String> {
    let exe = validate_launcher(&exe_path)?;
    validate_name(&name)?;
    if !matches!(folder.as_str(), "root" | "saves" | "mods" | "resourcepacks" | "shaderpacks" | "logs") {
        return Err("Unsupported Horizon game folder.".into());
    }
    let text = run_cli(&exe, &["instance", "list", "--json"], QUICK_TIMEOUT).await?;
    let entries: Vec<serde_json::Value> = serde_json::from_str(&text)
        .map_err(|_| "Horizon instance listing is invalid.")?;
    let entry = entries.iter().find(|value| value["id"].as_str() == Some(name.as_str()))
        .ok_or("The Horizon instance no longer exists.")?;
    if !matches!(entry["kind"].as_str(), Some("own" | "default")) || !entry["java"].is_null() {
        return Err("This is not a native Horizon game instance.".into());
    }
    let home = PathBuf::from(entry["home"].as_str().ok_or("Horizon did not return an instance folder.")?);
    let home = home.canonicalize().map_err(|e| e.to_string())?;
    if !home.is_dir() { return Err("Horizon game folder is unavailable.".into()); }
    let child = if folder == "root" { home.clone() } else { home.join(folder) };
    let target = child.canonicalize().map_err(|e| e.to_string())?;
    if !target.starts_with(&home) || !target.is_dir() {
        return Err("The selected folder is outside the Horizon game directory or missing.".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("explorer.exe")
            .arg(&target)
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| format!("Cannot open Horizon folder: {e}"))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = target;
        Err("Opening Horizon folders is currently supported on Windows.".into())
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct HorizonGameFile {
    name: String,
    bytes: u64,
    is_directory: bool,
}

fn read_horizon_files(home: &Path, folder: &str) -> Result<Vec<HorizonGameFile>, String> {
    if !matches!(folder, "root" | "saves" | "mods" | "resourcepacks" | "shaderpacks" | "logs") {
        return Err("Unsupported Horizon game folder.".into());
    }
    let home = home.canonicalize().map_err(|e| e.to_string())?;
    if !home.is_dir() { return Err("Horizon game folder is unavailable.".into()); }
    let target = if folder == "root" { home.clone() } else { home.join(folder) };
    let resolved = target.canonicalize().map_err(|e| e.to_string())?;
    if !resolved.starts_with(&home) || !resolved.is_dir() {
        return Err("The selected folder is outside the Horizon game directory or missing.".into());
    }
    let mut results = Vec::new();
    for entry in std::fs::read_dir(resolved).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() || !(kind.is_file() || kind.is_dir()) { continue; }
        if results.len() >= 500 {
            return Err("Folder has more than 500 entries; open it in Windows Explorer.".into());
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let bytes = if kind.is_file() {
            entry.metadata().map_err(|e| e.to_string())?.len()
        } else { 0 };
        results.push(HorizonGameFile { name, bytes, is_directory: kind.is_dir() });
    }
    results.sort_by(|a,b| b.is_directory.cmp(&a.is_directory)
        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(results)
}

#[tauri::command]
async fn horizon_list_files(exe_path: String, name: String, folder: String)
    -> Result<Vec<HorizonGameFile>, String> {
    let exe = validate_launcher(&exe_path)?;
    validate_name(&name)?;
    let raw = run_cli(&exe, &["instance", "list", "--json"], QUICK_TIMEOUT).await?;
    let entries: Vec<serde_json::Value> = serde_json::from_str(&raw)
        .map_err(|_| "Horizon instance listing is invalid.")?;
    let entry = entries.iter().find(|entry| entry["id"].as_str() == Some(name.as_str()))
        .ok_or("The Horizon instance no longer exists.")?;
    if !matches!(entry["kind"].as_str(), Some("own" | "default")) || !entry["java"].is_null() {
        return Err("This is not a native Horizon game instance.".into());
    }
    let home = PathBuf::from(entry["home"].as_str()
        .ok_or("Horizon did not return an instance folder.")?);
    read_horizon_files(&home, &folder)
}

fn validate_launcher(path: &str) -> Result<PathBuf, String> {
    let path = Path::new(path);
    if !path.is_absolute() {
        return Err("Choose the full path to horizon-launcher.exe.".into());
    }
    let absolute = path.canonicalize().map_err(|e| format!("Horizon executable not found: {e}"))?;
    if !absolute.is_file() || !absolute.file_name().and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("horizon-launcher.exe")) {
        return Err("Choose horizon-launcher.exe from the official Horizon Windows release.".into());
    }
    Ok(absolute)
}

fn validate_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 64 || name.starts_with('-')
        || name.ends_with('.') || name.chars().any(|c| c.is_control() || "<>:\"/\\|?*".contains(c)) {
        return Err("Use a name of 1–64 characters without Windows filename symbols.".into());
    }
    Ok(name)
}

fn validate_version(version: &str) -> Result<&str, String> {
    let version = version.trim();
    if version.is_empty() || version.len() > 80
        || !version.starts_with(|c: char| c.is_ascii_alphanumeric())
        || !version.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')) {
        return Err("Enter a Horizon version tag or 'latest'.".into());
    }
    Ok(version)
}

fn command(path: &Path) -> Command {
    let mut command = Command::new(path);
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    command
}

async fn bounded_output(mut stream: impl AsyncRead + Unpin) -> std::io::Result<Vec<u8>> {
    let mut retained = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        let count = stream.read(&mut buffer).await?;
        if count == 0 { return Ok(retained); }
        retained.extend_from_slice(&buffer[..count]);
        if retained.len() > OUTPUT_LIMIT {
            retained.drain(..retained.len() - OUTPUT_LIMIT);
        }
    }
}

async fn collect(mut child: tokio::process::Child) -> std::io::Result<Output> {
    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    let (status, stdout, stderr) = tokio::try_join!(child.wait(), bounded_output(stdout), bounded_output(stderr))?;
    Ok(Output { status, stdout, stderr })
}

fn output_string(output: Output) -> Result<String, String> {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if output.status.success() {
        return Ok(if stdout.is_empty() { stderr } else { stdout });
    }
    Err(format!("Horizon launcher returned {}: {}", output.status,
        if stderr.is_empty() { stdout } else { stderr }.chars().take(1500).collect::<String>()))
}

async fn run_cli(exe: &Path, args: &[&str], timeout: Duration) -> Result<String, String> {
    let _operation = OPERATION.try_lock().map_err(|_| "Another Horizon operation is still running.")?;
    let child = command(exe).args(args).kill_on_drop(true).spawn()
        .map_err(|e| format!("Could not run the Horizon launcher: {e}"))?;
    let output = tokio::time::timeout(timeout, collect(child)).await
        .map_err(|_| "Horizon operation timed out. Check the official launcher and logs before retrying.")?
        .map_err(|e| format!("Horizon output error: {e}"))?;
    output_string(output)
}


//// Verified Horizon helper: prepare proprietary Minecraft assets only on
//// the signed-in owner's PC. Neither assets nor tokens are shipped by FRE.
pub async fn prepare_for_native_rust<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<PathBuf, String> {
    if !cfg!(all(windows, target_arch = "x86_64")) {
        return Err("Automatic Horizon preparation requires Windows x64.".into());
    }
    let _ = app.emit("fre-rust-prepare-stage", "helper");
    let exe = bootstrap::install(&launcher_directory(&app)?, |_| {}).await?;
    let exe_path = exe.to_string_lossy().into_owned();
    let mut status = horizon_status(exe_path.clone()).await?;
    if status.installed_versions.is_empty() {
        let _ = app.emit("fre-rust-prepare-stage", "version");
        run_cli(&exe, &["install", "latest"], INSTALL_TIMEOUT).await?;
        status = horizon_status(exe_path).await?;
    }
    let version = status.selected_version.as_deref()
        .filter(|tag| status.installed_versions.iter().any(|installed| installed.as_str() == *tag))
        .or_else(|| status.installed_versions.first().map(String::as_str))
        .ok_or("Horizon did not install a Rust game version.")?;
    let version = validate_version(version)?.to_owned();
    if !status.prepared_versions.contains(&version) {
        // Prefer existing installations; only fetch missing resources from
        // Mojang after this attempt fails. Official Horizon verifies hashes,
        // account ownership and downloads, and resumes its own cache.
        let _ = app.emit("fre-rust-prepare-stage", "local");
        if run_cli(&exe, &["prepare", "--version", &version], INSTALL_TIMEOUT).await.is_err() {
            let _ = app.emit("fre-rust-prepare-stage", "download");
            run_cli(&exe, &["prepare", "--download", "--version", &version], INSTALL_TIMEOUT)
                .await.map_err(|error| format!(
                    "Minecraft could not be prepared by Horizon: {error}. Sign in to your Minecraft-owning Microsoft account in Horizon and retry."
                ))?;
        }
    }
    let data = std::env::var_os("HORIZON_LAUNCHER_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join("horizon-launcher")))
        .unwrap_or(app.path().data_dir().map_err(|e| e.to_string())?.join("horizon-launcher"));
    let release = data.join("versions").join(&version);
    let game_version = std::fs::read(release.join("research-lock.json")).ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|json| json.pointer("/minecraft/version").and_then(|value| value.as_str()).map(str::to_owned))
        .unwrap_or_else(|| "26.3".into());
    if game_version != "26.3" {
        return Err(format!("Horizon {version} uses Minecraft {game_version}, while this FRE Rust engine requires 26.3."));
    }
    let resources = release.join(".local").join("horizon-client").join("26.3");
    if !resources.join("horizon-resources.json").is_file() {
        return Err("Horizon finished without a valid Minecraft 26.3 resource manifest.".into());
    }
    Ok(resources)
}

/// Execute the official checked CLI with a native FRE profile as its
/// Horizon home. Native mod downloads and signatures remain Horizon's
/// responsibility; Java JARs are never installed as native mods here.
pub async fn native_mods_for_profile<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    profile_home: &Path,
    action: &str,
    project: &str,
) -> Result<String, String> {
    if !cfg!(all(windows, target_arch = "x86_64")) {
        return Err("Horizon native mod browsing requires Windows x64.".into());
    }
    let exe = bootstrap::install(&launcher_directory(&app)?, |_| {}).await?;
    let args: Vec<&str> = match action {
        "list" => vec!["--json", "mods", "list", "--instance", "horizon"],
        "search" => {
            if project.trim().is_empty() || project.chars().count() > 100 {
                return Err("Enter a search term (up to 100 characters).".into());
            }
            vec!["--json", "mods", "search", project, "--kind", "mod", "--source", "horizon", "--instance", "horizon"]
        }
        "install" | "remove" => {
            if project.is_empty() || project.len() > 128 ||
                !project.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.')) ||
                project == "." || project == ".." {
                return Err("Invalid Horizon mod identifier.".into());
            }
            if action == "install" {
                vec!["mods", "install", project, "--instance", "horizon", "--source", "horizon"]
            } else {
                vec!["mods", "remove", project, "--instance", "horizon"]
            }
        }
        _ => return Err("Unsupported Horizon mod operation.".into()),
    };
    let _operation = OPERATION.try_lock().map_err(|_| "Another Horizon operation is still running.")?;
    let child = command(&exe).args(&args)
        .env("HORIZON_HOME", profile_home)
        .kill_on_drop(true)
        .spawn().map_err(|e| format!("Could not run the Horizon mod manager: {e}"))?;
    let output = tokio::time::timeout(INSTALL_TIMEOUT, collect(child)).await
        .map_err(|_| "Horizon mod operation timed out.")?
        .map_err(|e| format!("Horizon mod operation failed: {e}"))?;
    output_string(output)
}

#[tauri::command]
async fn horizon_instances(exe_path: String) -> Result<String, String> {
    run_cli(&validate_launcher(&exe_path)?, &["instance", "list"], QUICK_TIMEOUT).await
}

#[tauri::command]
async fn horizon_create(exe_path: String, name: String) -> Result<String, String> {
    let name = validate_name(&name)?;
    run_cli(&validate_launcher(&exe_path)?, &["instance", "create", name], QUICK_TIMEOUT).await?;
    Ok(format!("Horizon instance '{name}' created."))
}

#[tauri::command]
async fn horizon_play(exe_path: String, name: String) -> Result<String, String> {
    let name = validate_name(&name)?;
    let _operation = OPERATION.try_lock().map_err(|_| "Another Horizon operation is still running.")?;
    let mut child = command(&validate_launcher(&exe_path)?).args(["play", "--instance", name])
        .stdout(Stdio::null()).stderr(Stdio::null())
        .spawn().map_err(|e| format!("Could not start Horizon: {e}"))?;
    let pid = child.id().unwrap_or_default();
    // No pipes: closing FRE LAUNCH must not break the game's stdout/stderr.
    let mut task = tokio::spawn(async move { child.wait().await });
    match tokio::time::timeout(Duration::from_millis(1000), &mut task).await {
        Ok(result) => {
            let status = result.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
            let details = output_string(Output { status, stdout: Vec::new(),
                stderr: b"Check the official Horizon launcher and logs for details.".to_vec() })?;
            Ok(format!("Horizon launcher finished.\n{details}"))
        }
        Err(_) => Ok(format!("Passed instance '{name}' to Horizon launcher (process {pid}). Check Horizon or its logs for game status.")),
    }
}

#[tauri::command]
async fn horizon_logs(exe_path: String) -> Result<String, String> {
    run_cli(&validate_launcher(&exe_path)?, &["logs", "--lines", "120"], QUICK_TIMEOUT).await
}

#[tauri::command]
async fn horizon_versions(exe_path: String, installed_only: bool) -> Result<String, String> {
    let args: &[&str] = if installed_only { &["versions", "--installed"] } else { &["versions"] };
    run_cli(&validate_launcher(&exe_path)?, args, QUICK_TIMEOUT).await
}

#[tauri::command]
async fn horizon_install_version(exe_path: String, version: String) -> Result<String, String> {
    let version = validate_version(&version)?;
    run_cli(&validate_launcher(&exe_path)?, &["install", version], INSTALL_TIMEOUT).await
}

#[tauri::command]
async fn horizon_set_version(exe_path: String, name: String, version: String) -> Result<String, String> {
    let name = validate_name(&name)?;
    let version = validate_version(&version)?;
    run_cli(&validate_launcher(&exe_path)?, &["instance", "version", name, version], QUICK_TIMEOUT).await?;
    Ok(format!("Set '{name}' to Horizon version '{version}'."))
}

#[tauri::command]
async fn horizon_prepare(exe_path: String, minecraft_path: Option<String>, version: Option<String>) -> Result<String, String> {
    let exe = validate_launcher(&exe_path)?;
    let mut args = vec!["prepare"];
    if let Some(version) = version.as_deref().filter(|v| !v.trim().is_empty() && v.trim() != "latest") {
        args.extend(["--version", validate_version(version)?]);
    }
    if let Some(path) = minecraft_path.as_deref().filter(|p| !p.trim().is_empty()) {
        if !Path::new(path).is_absolute() || !Path::new(path).is_dir() {
            return Err("Choose an existing Minecraft installation folder.".into());
        }
        args.extend(["--minecraft", path]);
    }
    run_cli(&exe, &args, INSTALL_TIMEOUT).await
}

/// FRE LAUNCH owns the Play action. The official Horizon CLI is used only for
/// verified installation and preparing Minecraft resources, not for starting
/// the game. The actual program here is horizon-client.exe.
#[tauri::command]
async fn horizon_direct_play<R: tauri::Runtime>(
    app: tauri::AppHandle<R>, exe_path: String, name: String,
) -> Result<String, String> {
    let exe = validate_launcher(&exe_path)?;
    let name = validate_name(&name)?;
    // Never infer an instance from a folder typed by the caller: trust the
    // official locally installed instance manifest returned by the CLI.
    let state = horizon_status(exe_path.clone()).await?;
    let profile = state.instances.iter().find(|instance| instance.id == name)
        .ok_or("Horizon profile is no longer present; refresh the library.")?;
    if profile.id != "horizon"
        && (!profile.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            || profile.id.starts_with('-')) {
        return Err("Only ordinary, independent Horizon profiles can be launched directly.".into());
    }
    let version = profile.version.as_deref()
        .filter(|tag| *tag != "latest")
        .or(state.selected_version.as_deref())
        .or_else(|| state.installed_versions.first().map(String::as_str))
        .ok_or("Install a Horizon game version before playing.")?;
    let version = validate_version(version)?.to_owned();
    if !state.installed_versions.iter().any(|v| v == &version)
        || !state.prepared_versions.iter().any(|v| v == &version) {
        return Err("Horizon game version or Minecraft resources are not prepared. Open Manage.".into());
    }

    // Horizon itself uses dirs::data_dir()/horizon-launcher and
    // dirs::data_dir()/horizon (APPDATA on Windows). Respect documented
    // overrides so profiles created by the official installer remain visible.
    let data = std::env::var_os("APPDATA").map(PathBuf::from)
        .unwrap_or(app.path().data_dir().map_err(|e| e.to_string())?);
    let launcher_home = std::env::var_os("HORIZON_LAUNCHER_HOME")
        .filter(|v| !v.is_empty()).map(PathBuf::from)
        .unwrap_or_else(|| data.join("horizon-launcher"));
    let game_home = std::env::var_os("HORIZON_HOME")
        .filter(|v| !v.is_empty()).map(PathBuf::from)
        .unwrap_or_else(|| data.join("horizon"));
    let release_dir = launcher_home.join("versions").join(&version);
    let binary = release_dir.join("horizon-client.exe");
    if !release_dir.join(".installed").is_file() || !binary.is_file() {
        return Err(format!("Native Rust game binary missing for {version}. Reinstall the game from Manage."));
    }
    let game_version = std::fs::read(release_dir.join("research-lock.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|v| v.pointer("/minecraft/version").and_then(|x| x.as_str()).map(str::to_string))
        .unwrap_or_else(|| "26.3".to_owned());
    // Prevent resource traversal even if a release's metadata was modified.
    if !game_version.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
        || !game_version.starts_with(|c: char| c.is_ascii_digit()) {
        return Err("Horizon release has an invalid Minecraft resource version.".into());
    }
    let resources = release_dir.join(".local").join("horizon-client").join(&game_version);
    if !resources.join("horizon-resources.json").is_file() {
        return Err("Minecraft game resources were not prepared for this Horizon build.".into());
    }
    let home = if profile.id == "horizon" {
        game_home.clone()
    } else {
        game_home.join("instances").join(&profile.id)
    };
    if !home.is_dir() {
        return Err("Horizon profile folder was removed. Refresh the library.".into());
    }
    // The official client refreshes authenticated sessions through the
    // installed helper only if it needs to. That does not start its GUI.
    let mut child = Command::new(&binary);
    child.current_dir(&home)
        .env("HORIZON_HOME", &home)
        .env("HORIZON_LAUNCHER_EXE", &exe)
        .arg("--resources").arg(&resources)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    child.creation_flags(0x00000008 | 0x00000200); // DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP: no terminal or Ctrl+C inheritance
    let mut child = child.spawn().map_err(|e| format!("Could not start the Rust client: {e}"))?;
    let pid = child.id().unwrap_or_default();
    let mut task = tokio::spawn(async move { child.wait().await });
    match tokio::time::timeout(Duration::from_millis(1000), &mut task).await {
        Ok(result) => {
            let status = result.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
            if !status.success() {
                return Err(format!("Rust client exited immediately ({status}). Open Horizon logs."));
            }
            Ok(format!("Rust client exited normally. Profile: {}.", profile.name))
        }
        Err(_) => Ok(format!("Started horizon-client.exe directly for {} (PID {pid}).", profile.name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    static PROCESS_TEST: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    #[test]
    fn in_app_files_are_scoped_to_horizon_profile() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("mods")).unwrap();
        std::fs::write(root.path().join("mods/example.native"), b"game").unwrap();
        let files = read_horizon_files(root.path(), "mods").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "example.native");
        assert_eq!(files[0].bytes, 4);
        assert!(read_horizon_files(root.path(), "../../").is_err());
        assert!(read_horizon_files(root.path(), "missing").is_err());
    }

    #[test]
    fn status_uses_real_cli_json_and_never_calls_unprepared_ready() {
        let instances = r#"[{"id":"horizon","name":"Horizon","kind":"default","version":null,"java":null,"selected":true},
            {"id":"java","name":"Java","kind":"java","java":{"minecraft":"26.3"}}]"#;
        let status = parse_status(instances,
            r#"{"installed":["nightly-new","nightly-old"],"selected":"nightly-new"}"#,
            "* nightly-new  not prepared\n  nightly-old  prepared, no sounds (prepare again)").unwrap();
        assert_eq!(status.instances.len(), 1);
        assert_eq!(status.prepared_versions, vec!["nightly-old"]);
        assert_eq!(status.selected_version.as_deref(), Some("nightly-new"));
        assert!(parse_status("not JSON", "{}", "").is_err());
    }
    #[cfg(windows)]
    #[tokio::test]
    #[ignore = "requires official launcher download and explicit isolated HOME overrides"]
    async fn official_windows_cli_smoke() {
        let home = PathBuf::from(std::env::var_os("HORIZON_LAUNCHER_HOME").expect("isolated HOME required"));
        assert!(home.is_absolute() && home.components().any(|p| p.as_os_str() == ".audit"));
        let directory = home.parent().unwrap().join("managed-native-smoke");
        let exe = bootstrap::install(&directory, |_| {}).await.unwrap();
        assert!(bootstrap::cached(&directory).is_some());
        let path = exe.to_string_lossy().into_owned();
        let status = horizon_status(path.clone()).await.unwrap();
        assert!(status.instances.iter().any(|i| i.id == "horizon"));
        let name = format!("FRE audit {}", uuid::Uuid::new_v4());
        horizon_create(path.clone(), name.clone()).await.unwrap();
        assert!(horizon_status(path).await.unwrap().instances.iter().any(|i| i.name == name));
    }
    #[test]
    fn validates_instance_name_and_version() {
        assert_eq!(validate_name("  Мій Horizon  ").unwrap(), "Мій Horizon");
        assert!(validate_name(&"ї".repeat(64)).is_ok());
        for name in ["../world", "--config", "", "world."] { assert!(validate_name(name).is_err()); }
        for version in ["latest", "v2026.10.13", "0.2.31"] { assert!(validate_version(version).is_ok()); }
        for version in ["--config", "../../bad", ".", "..", ""] { assert!(validate_version(version).is_err()); }
    }
    #[tokio::test]
    async fn output_is_bounded_and_keeps_the_end() {
        let bytes = vec![b'x'; OUTPUT_LIMIT * 4];
        let result = bounded_output(bytes.as_slice()).await.unwrap();
        assert_eq!(result, bytes[bytes.len() - OUTPUT_LIMIT..]);
    }
    #[tokio::test]
    async fn slow_cli_is_terminated_at_the_deadline() {
        let _test = PROCESS_TEST.lock().await;
        let exe = std::env::current_exe().unwrap();
        let error = run_cli(&exe,
            &["--ignored", "--exact", "api::horizon::tests::sleep_fixture"],
            Duration::from_millis(100)).await.unwrap_err();
        assert!(error.contains("timed out"), "{error}");
    }
    #[test]
    #[ignore = "child process fixture for the timeout regression"]
    fn sleep_fixture() { std::thread::sleep(Duration::from_secs(10)); }
    #[cfg(windows)]
    #[tokio::test]
    async fn captures_real_process_output_and_failure() {
        let _test = PROCESS_TEST.lock().await;
        let exe = PathBuf::from(std::env::var_os("COMSPEC").unwrap());
        assert!(run_cli(&exe, &["/D", "/C", "echo horizon-test"], QUICK_TIMEOUT).await.unwrap().contains("horizon-test"));
        assert!(run_cli(&exe, &["/D", "/C", "exit 7"], QUICK_TIMEOUT).await.is_err());
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn commands_match_official_cli_contract_using_isolated_fixture() {
        let _test = PROCESS_TEST.lock().await;
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("fixture.rs");
        let exe = directory.path().join("horizon-launcher.exe");
        std::fs::write(&source, r#"
            fn main() {
                let args: Vec<String> = std::env::args().skip(1).collect();
                let args: Vec<&str> = args.iter().map(String::as_str).collect();
                match args.as_slice() {
                    ["instance", "list"] => println!("Survival with spaces"),
                    ["instance", "create", "Survival with spaces"] => println!("created"),
                    ["play", "--instance", "Survival with spaces"] => println!("finished"),
                    ["play", "--instance", "Missing"] => {
                        eprintln!("Missing installed version"); std::process::exit(7);
                    }
                    ["logs", "--lines", "120"] => println!("fixture logs"),
                    ["versions", "--installed"] => println!("installed"),
                    ["versions"] => println!("available"),
                    ["install", "latest"] => println!("installed latest"),
                    ["instance", "version", "Survival with spaces", "latest"] => println!("selected"),
                    ["prepare"] | ["prepare", "--minecraft", _]
                        | ["prepare", "--version", "v2026.10.13"] => println!("prepared"),
                    _ => { eprintln!("Invalid CLI arguments: {args:?}"); std::process::exit(2); }
                }
            }
        "#).unwrap();
        let output = std::process::Command::new("rustc").arg(&source).arg("-o").arg(&exe).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let path = exe.to_str().unwrap().to_string();
        assert!(horizon_instances(path.clone()).await.unwrap().contains("Survival with spaces"));
        horizon_create(path.clone(), "Survival with spaces".into()).await.unwrap();
        assert_eq!(horizon_versions(path.clone(), true).await.unwrap(), "installed");
        assert_eq!(horizon_versions(path.clone(), false).await.unwrap(), "available");
        horizon_install_version(path.clone(), "latest".into()).await.unwrap();
        horizon_set_version(path.clone(), "Survival with spaces".into(), "latest".into()).await.unwrap();
        horizon_prepare(path.clone(), None, None).await.unwrap();
        horizon_prepare(path.clone(), None, Some("v2026.10.13".into())).await.unwrap();
        horizon_prepare(path.clone(), Some(directory.path().to_str().unwrap().into()), Some(" latest ".into())).await.unwrap();
        assert_eq!(horizon_logs(path.clone()).await.unwrap(), "fixture logs");
        horizon_play(path.clone(), "Survival with spaces".into()).await.unwrap();
        assert!(horizon_play(path, "Missing".into()).await.unwrap_err().contains('7'));
    }
}
