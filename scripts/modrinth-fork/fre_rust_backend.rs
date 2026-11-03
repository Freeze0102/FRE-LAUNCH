//! FRE LAUNCH built-in Rust Minecraft profile manager.
//! No Horizon Launcher process, filesystem dependency or API. The native
//! client is bundled from openly licensed Rust sources at local build time.
//! Private Minecraft game resources are supplied only from the owner's copy.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{Emitter, Manager};
use tokio::io::AsyncWriteExt;

const MAX_NAME: usize = 64;
const MAX_METADATA: u64 = 8192;
const MAX_ICON_BYTES: usize = 2 * 1024 * 1024;
// Creation must be serialized: two simultaneous requests cannot create
// duplicate visible profile names or compete for the same metadata.
static PROFILE_CREATION: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
static RUNNING_PROFILES: tokio::sync::Mutex<Vec<String>> = tokio::sync::Mutex::const_new(Vec::new());
static RESOURCE_IMPORT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RustProfile {
    id: String,
    name: String,
    created: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    icon_path: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RustRuntime {
    installed: bool,
    prepared: bool,
    version: &'static str,
    reason: Option<String>,
}

// Only filenames and basic metadata, never arbitrary file contents.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RustGameFile {
    name: String,
    bytes: u64,
    is_directory: bool,
}

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("fre-rust")
        .invoke_handler(tauri::generate_handler![
            rust_profiles,
            rust_create_profile,
            rust_rename_profile,
            rust_delete_profile,
            rust_runtime,
            rust_import_resources,
            rust_mods,
            rust_launch,
            rust_logs,
            rust_open_folder,
            rust_list_files,
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
    // If a settings write was interrupted between moving the original
    // metadata into its backup and installing the replacement, keep the
    // original profile visible until the user can retry recovery.
    let current = dir.join("profile.json");
    let metadata = if current.exists() { current } else { dir.join("profile.json.backup") };
    if metadata.symlink_metadata().ok()?.file_type().is_symlink() {
        return None;
    }
    if metadata.metadata().ok()?.len() > MAX_METADATA {
        return None;
    }
    let mut profile: RustProfile = serde_json::from_slice(&fs::read(metadata).ok()?).ok()?;
    if profile.id != id || !name_valid(&profile.name) { return None; }
    // Never trust a path supplied through editable profile.json metadata.
    // A profile icon may only be an actual ordinary file INSIDE this profile.
    profile.icon_path = ["icon.png", "icon.jpg", "icon.webp"].iter()
        .map(|name| dir.join(name))
        .find(|file| file.symlink_metadata().is_ok_and(|m| m.file_type().is_file()))
        .map(|file| file.to_string_lossy().into_owned());
    Some(profile)
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

// Only three inert raster formats are accepted. Never persist raw SVG, HTML
// or an arbitrary selected file as executable content.
fn valid_icon(bytes: &[u8]) -> Result<&'static str, String> {
    if bytes.len() < 12 || bytes.len() > MAX_ICON_BYTES {
        return Err("The profile icon must be a PNG, JPEG or WebP smaller than 2 MiB.".into());
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") { return Ok("png"); }
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) { return Ok("jpg"); }
    if bytes.starts_with(b"RIFF") && bytes.get(8..12).is_some_and(|tag| tag == &b"WEBP"[..]) {
        return Ok("webp");
    }
    Err("Profile icon must use PNG, JPEG or WebP; SVG and other formats are not allowed.".into())
}

fn prepare_icon(icon_path: Option<&str>, icon_bytes: Option<Vec<u8>>)
    -> Result<Option<(Vec<u8>, &'static str)>, String> {
    let bytes = if let Some(bytes) = icon_bytes {
        bytes
    } else if let Some(path) = icon_path.filter(|path| !path.trim().is_empty()) {
        let path = Path::new(path);
        let metadata = fs::symlink_metadata(path).map_err(|e| format!("Cannot read profile icon: {e}"))?;
        if !metadata.file_type().is_file() || metadata.len() > MAX_ICON_BYTES as u64 {
            return Err("Profile icon is not a regular image file smaller than 2 MiB.".into());
        }
        fs::read(path).map_err(|e| format!("Cannot read profile icon: {e}"))?
    } else {
        return Ok(None);
    };
    let extension = valid_icon(&bytes)?;
    Ok(Some((bytes, extension)))
}

#[tauri::command]
async fn rust_create_profile<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    name: String,
    icon_path: Option<String>,
    icon_bytes: Option<Vec<u8>>,
) -> Result<RustProfile, String> {
    let _guard = PROFILE_CREATION.lock().await;
    let name = name.trim().to_owned();
    if !name_valid(&name) {
        return Err("Name must contain 1–64 characters without control characters.".into());
    }
    let icon = prepare_icon(icon_path.as_deref(), icon_bytes)?;
    let root = base(&app)?;
    if list(&root)?.iter().any(|p| p.name.eq_ignore_ascii_case(&name)) {
        return Err("A Rust profile with this name already exists.".into());
    }
    let profiles_dir = root.join("profiles");
    fs::create_dir_all(&profiles_dir).map_err(|e| e.to_string())?;
    if fs::symlink_metadata(&profiles_dir).map_err(|e| e.to_string())?.file_type().is_symlink() {
        return Err("Refusing to create a Minecraft profile through a symbolic link.".into());
    }
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
                let saved_icon = if let Some((bytes, extension)) = icon.as_ref() {
                    let target = dir.join(format!("icon.{extension}"));
                    if let Err(err) = fs::OpenOptions::new().write(true).create_new(true)
                        .open(&target).and_then(|mut file| std::io::Write::write_all(&mut file, bytes)) {
                        let _ = fs::remove_dir_all(&dir); // Only this fresh profile.
                        return Err(format!("Could not save Rust profile icon: {err}"));
                    }
                    Some(target.to_string_lossy().into_owned())
                } else { None };
                let profile = RustProfile { id, name, created, icon_path: saved_icon };
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
                // A real independent game home, not a shortcut to a foreign
                // launcher. Each instance has its own worlds and native mods.
                for folder in ["saves", "horizon-mods", "resourcepacks", "shaderpacks", "screenshots", "logs"] {
                    if let Err(error) = fs::create_dir(dir.join(folder)) {
                        let _ = fs::remove_dir_all(&dir); // Freshly created, never user data.
                        return Err(format!("Could not initialize Rust game folders: {error}"));
                    }
                }
                return Ok(profile);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("Cannot create Rust profile folder: {error}")),
        }
    }
    Err("Could not allocate a new Rust profile identifier.".into())
}

// The library's normal Rename and Delete actions operate ONLY on the
// selected native profile. Never feed a Rust ID to the Java profile manager.
#[tauri::command]
async fn rust_rename_profile<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    profile_id: String,
    name: String,
) -> Result<RustProfile, String> {
    let _guard = PROFILE_CREATION.lock().await;
    let name = name.trim().to_owned();
    if !name_valid(&name) {
        return Err("Profile name must contain 1–64 characters without control characters.".into());
    }
    if RUNNING_PROFILES.lock().await.contains(&profile_id) {
        return Err("Stop this Rust game before changing its settings.".into());
    }
    let root = base(&app)?;
    let dir = profile_dir(&root, &profile_id)?;
    let mut profile = read_profile(&dir, &profile_id)
        .ok_or("Rust profile no longer exists.")?;
    if list(&root)?.iter().any(|p| p.id != profile_id && p.name.eq_ignore_ascii_case(&name)) {
        return Err("Another Rust profile already uses this name.".into());
    }
    if profile.name == name {
        return Ok(profile);
    }
    profile.name = name;
    // Preserve the old metadata across an interrupted write. Only the
    // selected profile's own JSON is replaced; worlds and mods are untouched.
    let target = dir.join("profile.json");
    let staged = dir.join("profile.json.pending");
    let backup = dir.join("profile.json.backup");
    if staged.exists() || backup.exists() {
        return Err("A previous profile settings update needs recovery. Do not overwrite its backup.".into());
    }
    let content = serde_json::to_vec_pretty(&profile).map_err(|e| e.to_string())?;
    fs::OpenOptions::new().create_new(true).write(true).open(&staged)
        .and_then(|mut file| {
            use std::io::Write;
            file.write_all(&content)?;
            file.sync_all()
        })
        .map_err(|e| format!("Could not stage Rust profile settings: {e}"))?;
    if let Err(err) = fs::rename(&target, &backup) {
        let _ = fs::remove_file(&staged);
        return Err(format!("Could not preserve old Rust profile settings: {err}"));
    }
    if let Err(err) = fs::rename(&staged, &target) {
        let _ = fs::rename(&backup, &target);
        return Err(format!("Could not save Rust profile settings: {err}"));
    }
    let _ = fs::remove_file(&backup);
    Ok(profile)
}

#[tauri::command]
async fn rust_delete_profile<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    profile_id: String,
) -> Result<(), String> {
    let _guard = PROFILE_CREATION.lock().await;
    if RUNNING_PROFILES.lock().await.contains(&profile_id) {
        return Err("Stop this Rust game before deleting its profile and worlds.".into());
    }
    let root = base(&app)?;
    let dir = profile_dir(&root, &profile_id)?;
    read_profile(&dir, &profile_id).ok_or("The Rust profile no longer exists.")?;
    let profiles = root.join("profiles").canonicalize()
        .map_err(|e| format!("Could not resolve the Rust profile library: {e}"))?;
    let owned = dir.canonicalize()
        .map_err(|e| format!("Could not resolve the selected Rust profile: {e}"))?;
    if owned.parent() != Some(profiles.as_path()) || !owned.is_dir() {
        return Err("Refusing to remove a folder outside the Rust profile library.".into());
    }
    if fs::symlink_metadata(&dir).map_err(|e| e.to_string())?.file_type().is_symlink() {
        return Err("Refusing to delete a Rust profile through a symbolic link.".into());
    }
    // This operation follows the same explicit irreversible confirmation as
    // Java's ConfirmDeleteInstanceModal. It never removes shared game resources.
    fs::remove_dir_all(&dir).map_err(|e| format!("Could not delete Rust profile: {e}"))
}

fn runtime_files(root: &Path) -> (PathBuf, PathBuf) {
    (root.join("fre-rust-client.exe"), root.join("resources"))
}

#[derive(Deserialize)]
struct GameBuildInfo {
    engine_license: String,
    source_commit: String,
    game_exe_sha256: String,
}

/// Validate the local, pinned open-source engine before it ever starts.
/// Private Minecraft assets remain in their own read-only package folder.
fn verify_native_client(root: &Path) -> Result<(), String> {
    use std::io::Read;
    let info_path = root.join("BUILD-INFO.json");
    let bytes = fs::read(info_path).map_err(|e| format!("Rust build manifest missing: {e}"))?;
    if bytes.len() > 8192 {
        return Err("Rust build manifest is unexpectedly large.".into());
    }
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&bytes);
    let info: GameBuildInfo = serde_json::from_slice(bytes)
        .map_err(|e| format!("Invalid Rust build manifest: {e}"))?;
    if info.engine_license != "MIT"
        || info.source_commit != "975c07d9151847e6af13afc4b5ab37b6bbe78ef5"
        || info.game_exe_sha256.len() != 64
        || !info.game_exe_sha256.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err("Rust engine build provenance is invalid.".into());
    }
    let binary = root.join("fre-rust-client.exe");
    let mut file = fs::File::open(binary).map_err(|e| format!("Rust game executable missing: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 1024 * 1024];
    loop {
        let count = file.read(&mut buf).map_err(|e| e.to_string())?;
        if count == 0 { break; }
        hasher.update(&buf[..count]);
    }
    let digest = format!("{:x}", hasher.finalize());
    if !digest.eq_ignore_ascii_case(&info.game_exe_sha256) {
        return Err("Rust game executable checksum mismatch; reinstall a trusted FRE LAUNCH build.".into());
    }
    Ok(())
}

/// Quick integrity check for a completed Horizon preparation. A filename
/// alone is not proof of a usable cache: partial copies and stale manifests
/// must not make the library's Play button report Ready.
fn resources_valid(root: &Path) -> bool {
    use std::io::Read;
    let manifest = root.join("horizon-resources.json");
    let Ok(meta) = fs::symlink_metadata(&manifest) else { return false };
    if !meta.file_type().is_file() || meta.len() == 0 || meta.len() > 2 * 1024 * 1024 {
        return false;
    }
    let Ok(bytes) = fs::read(&manifest) else { return false };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else { return false };
    if value.get("minecraft").and_then(|v| v.as_str()) != Some("26.3")
        || !value.get("schema").and_then(|v| v.as_i64()).is_some_and(|v| v > 0) {
        return false;
    }
    // These hashes are recorded by Horizon's preparer and cover essential
    // output. The official preparer verifies the rest at download/prep time.
    for (field, relative) in [
        ("facts_sha256", "facts.json"),
        ("block_tags_sha256", "block-tags.json"),
        ("collision_sha256", "cached-collision.nbt"),
    ] {
        let Some(expected) = value.get(field).and_then(|v| v.as_str()) else { return false };
        if expected.len() != 64 || !expected.bytes().all(|b| b.is_ascii_hexdigit()) {
            return false;
        }
        let path = root.join(relative);
        let Ok(metadata) = fs::symlink_metadata(&path) else { return false };
        if !metadata.file_type().is_file() { return false; }
        let Ok(mut file) = fs::File::open(&path) else { return false };
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            let Ok(size) = file.read(&mut buffer) else { return false };
            if size == 0 { break; }
            hasher.update(&buffer[..size]);
        }
        if !format!("{:x}", hasher.finalize()).eq_ignore_ascii_case(expected) {
            return false;
        }
    }
    true
}

fn local_resources(data_root: &Path, bundle_root: &Path) -> PathBuf {
    let local = data_root.join("prepared");
    if resources_valid(&local) {
        local
    } else {
        bundle_root.join("resources") // backwards-compatible personal installer
    }
}

fn ready(root: &Path, data_root: &Path) -> RustRuntime {
    let (client, _) = runtime_files(root);
    let installed = client.is_file() && root.join("BUILD-INFO.json").is_file()
        && root.join("research-lock.json").is_file();
    let prepared = resources_valid(&local_resources(data_root, root));
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
    Ok(ready(&bundle(&app)?, &base(&app)?))
}

// Import only the owner's already-prepared game files from a local folder.
// The game engine is open source and bundled, but Mojang resources must
// never be copied into a PUBLIC installer or downloaded from an unknown host.
fn copy_prepared(source: &Path, target: &Path) -> Result<u64, String> {
    if !resources_valid(source) {
        return Err("Selected folder is not a prepared Minecraft Rust resource directory.".into());
    }
    if target.exists() {
        return Err("Resources are already installed. Back them up before replacing.".into());
    }
    let source = source.canonicalize().map_err(|e| e.to_string())?;
    let target_parent = target.parent().ok_or("Invalid resource destination")?;
    fs::create_dir_all(target_parent).map_err(|e| e.to_string())?;
    let pending = target_parent.join("prepared.pending");
    if pending.exists() {
        return Err("Previous resource import was interrupted. Inspect prepared.pending before retrying.".into());
    }
    let mut queue = vec![(source.clone(), pending.clone(), 0usize)];
    let mut total = 0u64;
    let mut count = 0usize;
    let result = (|| -> Result<u64, String> {
        while let Some((from, to, depth)) = queue.pop() {
            if depth > 20 { return Err("Too many nested folders in Minecraft resources.".into()); }
            if fs::symlink_metadata(&from).map_err(|e| e.to_string())?.file_type().is_symlink() {
                return Err("Symbolic links are not permitted inside imported Minecraft resources.".into());
            }
            fs::create_dir(&to).map_err(|e| format!("Could not create resource folder: {e}"))?;
            for entry in fs::read_dir(&from).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                count += 1;
                if count > 100_000 { return Err("Minecraft resource set has too many files.".into()); }
                let child = entry.path();
                let kind = entry.file_type().map_err(|e| e.to_string())?;
                if kind.is_symlink() { return Err("Resource symlinks are not permitted.".into()); }
                let dest = to.join(entry.file_name());
                if kind.is_dir() {
                    queue.push((child, dest, depth + 1));
                } else if kind.is_file() {
                    total = total.checked_add(entry.metadata().map_err(|e| e.to_string())?.len())
                        .ok_or("Resource size overflow")?;
                    if total > 12 * 1024 * 1024 * 1024 {
                        return Err("Minecraft resource set exceeds the 12 GiB safety limit.".into());
                    }
                    fs::copy(child, dest).map_err(|e| format!("Could not copy Minecraft resources: {e}"))?;
                } else {
                    return Err("Unexpected special file inside Minecraft resources.".into());
                }
            }
        }
        if !resources_valid(&pending) {
            return Err("Prepared resources manifest was not copied.".into());
        }
        fs::rename(&pending, target).map_err(|e| format!("Unable to finish resource import: {e}"))?;
        Ok(total)
    })();
    if result.is_err() && pending.exists() {
        // This is only our new empty/fresh import staging folder; leave the
        // user's source, existing game saves and finished installations intact.
        let _ = fs::remove_dir_all(&pending);
    }
    result
}

fn find_legacy_resources() -> Result<PathBuf, String> {
    let home = std::env::var_os("APPDATA").map(PathBuf::from)
        .ok_or("Windows application data folder unavailable.")?;
    let versions = home.join("horizon-launcher").join("versions");
    if !versions.is_dir() {
        return Err("Choose the folder containing prepared Minecraft Rust resources.".into());
    }
    let mut entries = fs::read_dir(versions).map_err(|e| e.to_string())?;
    for _ in 0..128 {
        let Some(entry) = entries.next() else { break };
        let entry = entry.map_err(|e| e.to_string())?;
        let folder = entry.path().join(".local").join("horizon-client").join("26.3");
        if resources_valid(&folder) { return Ok(folder); }
    }
    Err("No prepared Minecraft Rust resources were found. Select a folder with horizon-resources.json.".into())
}

/// Never destroy a damaged or interrupted resource cache: rename it
/// atomically so setup can retry with a fresh directory. The backup remains
/// under the FRE Rust data root for manual inspection.
fn preserve_incomplete(path: &Path) -> Result<(), String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(format!("Could not inspect incomplete cache: {e}")),
    };
    if !metadata.file_type().is_dir() {
        return Err("Unsafe Rust cache path: expected an ordinary directory, not a link or file.".into());
    }
    let parent = path.parent().ok_or("Invalid Rust cache parent")?;
    let name = path.file_name().and_then(|s| s.to_str()).ok_or("Invalid cache name")?;
    let time = SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?.as_secs();
    for suffix in 0..32 {
        let backup = parent.join(format!("{name}.incomplete-{time}-{suffix}"));
        if !backup.exists() {
            return fs::rename(path, backup)
                .map_err(|e| format!("Could not preserve incomplete Rust cache: {e}"));
        }
    }
    Err("Too many incomplete Rust cache backups; inspect the data folder.".into())
}

#[tauri::command]
async fn rust_import_resources<R: tauri::Runtime>(
    app: tauri::AppHandle<R>, source: String,
) -> Result<String, String> {
    let _guard = RESOURCE_IMPORT.lock().await;
    let runtime = bundle(&app)?;
    verify_native_client(&runtime)?;
    // Use Theseus's public Minecraft authentication API. Its private
    // State.pool and state::Credentials internals are inaccessible here.
    let credentials = theseus::minecraft_auth::fre_rust_default_credentials().await
        .map_err(|e| e.to_string())?
        .ok_or("Sign in to the Microsoft account that owns Minecraft: Java Edition first.")?;
    if credentials.is_offline_profile() || credentials.online_profile_fresh().await.is_none() {
        return Err("Minecraft Java Edition ownership could not be verified.".into());
    }
    let destination = base(&app)?.join("prepared");
    // Idempotent: prepared assets are shared by all FRE Rust instances. A
    // second launch never downloads or overwrites previously installed files.
    if resources_valid(&destination) {
        return Ok("Already prepared; using your cached Minecraft 26.3 game files.".into());
    }
    if RUNNING_PROFILES.lock().await.len() > 0 {
        return Err("Close every running Rust game before repairing shared resources.".into());
    }
    // Preserve a damaged cache and stale interrupted copy, then safely retry.
    // Do not remove any of the user's original files or saved worlds.
    preserve_incomplete(&destination)?;
    preserve_incomplete(&base(&app)?.join("prepared.pending"))?;
    let selected = if source.trim().is_empty() {
        match find_legacy_resources() {
            Ok(existing) => existing,
            Err(_) => super::horizon::prepare_for_native_rust(app.clone()).await?,
        }
    } else {
        PathBuf::from(source.trim())
    };
    if !selected.is_dir() { return Err("Minecraft resource folder not found.".into()); }
    let _ = app.emit("fre-rust-prepare-stage", "copy");
    let bytes = tokio::task::spawn_blocking(move || copy_prepared(&selected, &destination))
        .await.map_err(|e| e.to_string())??;
    let _ = app.emit("fre-rust-prepare-stage", "ready");
    Ok(format!("Imported {bytes} bytes of your Minecraft resources into FRE LAUNCH."))
}

/// Search the official Horizon registry and install only its native mods.
/// The checked official helper uses the validated FRE profile as HORIZON_HOME.
#[tauri::command]
async fn rust_mods<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    profile_id: String,
    action: String,
    project: Option<String>,
) -> Result<String, String> {
    let dir = profile_dir(&base(&app)?, &profile_id)?;
    read_profile(&dir, &profile_id).ok_or("Rust profile does not exist.")?;
    let dir = dir.canonicalize().map_err(|e| e.to_string())?;
    if !dir.join("profile.json").is_file() && !dir.join("profile.json.backup").is_file() {
        return Err("The Rust profile metadata is missing.".into());
    }
    if action != "search" && action != "list" &&
        RUNNING_PROFILES.lock().await.contains(&profile_id) {
        return Err("Close this Rust game before changing its mods.".into());
    }
    super::horizon::native_mods_for_profile(
        app, &dir, &action, project.as_deref().unwrap_or(""),
    ).await
}

#[tauri::command]
async fn rust_launch<R: tauri::Runtime>(
    app: tauri::AppHandle<R>, profile_id: String,
) -> Result<String, String> {
    // Hold the same profile lifecycle lock as Rename/Delete until this
    // launch has registered its running instance. Otherwise a concurrent
    // confirmed Delete could remove the world during account refresh.
    let _profile_guard = PROFILE_CREATION.lock().await;
    let root = base(&app)?;
    let dir = profile_dir(&root, &profile_id)?;
    let profile = read_profile(&dir, &profile_id)
        .ok_or("Rust profile not found. Refresh the library.")?;
    let runtime_dir = bundle(&app)?;
    let runtime = ready(&runtime_dir, &root);
    if let Some(message) = runtime.reason {
        return Err(message);
    }
    verify_native_client(&runtime_dir)?;
    let (client, _) = runtime_files(&runtime_dir);
    let resources = local_resources(&root, &runtime_dir);
    let client = client.canonicalize().map_err(|e| e.to_string())?;
    let resources = resources.canonicalize().map_err(|e| e.to_string())?;
    // The native client's Launch API requires an authenticated Microsoft
    // owner session (docs/api/launch.md). Reuse the active FRE LAUNCH
    // Minecraft account, never launch an online-mode client using a fake
    // nickname or an expired guest token.
    let credentials = theseus::minecraft_auth::fre_rust_default_credentials().await
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
    // Two simultaneous launches of one game directory can corrupt a saved
    // world. Reject duplicate launches from this FRE LAUNCH process.
    {
        let mut running = RUNNING_PROFILES.lock().await;
        if running.contains(&profile_id) {
            return Err("This Rust Minecraft profile is already running.".into());
        }
        running.push(profile_id.clone());
    }
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(error) => {
            RUNNING_PROFILES.lock().await.retain(|id| id != &profile_id);
            return Err(format!("Could not start Rust Minecraft: {error}"));
        }
    };
    let mut stdin = match child.stdin.take() {
        Some(stdin) => stdin,
        None => {
            let _ = child.kill().await;
            RUNNING_PROFILES.lock().await.retain(|id| id != &profile_id);
            return Err("Could not open the private Minecraft session pipe.".into());
        }
    };
    if let Err(error) = stdin.write_all(&session_bytes).await {
        let _ = child.kill().await;
        RUNNING_PROFILES.lock().await.retain(|id| id != &profile_id);
        return Err(format!("Could not securely hand the Minecraft account to the Rust game: {error}"));
    }
    drop(stdin);
    let pid = child.id().unwrap_or_default();
    let active_id = profile_id.clone();
    let mut task = tokio::spawn(async move {
        let result = child.wait().await;
        RUNNING_PROFILES.lock().await.retain(|id| id != &active_id);
        result
    });
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

// Open a validated game subfolder, just like Java's Files button.
// Never accept arbitrary paths or let a profile folder symlink escape the app.
fn safe_game_folder(dir: &Path, folder: &str) -> Result<PathBuf, String> {
    const ALLOWED: &[&str] = &[
        "root", "saves", "mods", "resourcepacks", "shaderpacks", "screenshots", "logs"
    ];
    if !ALLOWED.contains(&folder) {
        return Err("Unsupported Rust game folder.".into());
    }
    let base = dir.canonicalize().map_err(|e| e.to_string())?;
    let target = if folder == "root" { base.clone() } else if folder == "mods" { base.join("horizon-mods") } else { base.join(folder) };
    // Profiles created before Horizon mod support have only a legacy 'mods'
    // folder. Create the correct empty native folder without moving user data.
    if folder == "mods" && !target.exists() {
        fs::create_dir(&target).map_err(|e| format!("Could not create Horizon mods folder: {e}"))?;
    }
    let resolved = target.canonicalize().map_err(|e| e.to_string())?;
    if !resolved.starts_with(&base) || !resolved.is_dir() {
        return Err("Cannot open a folder outside the Rust game profile.".into());
    }
    Ok(resolved)
}

// Restrict listing to known profile subfolders and never follow links.
// Rust native mods are not Java JARs; this provides file management only.
fn list_game_files(dir: &Path, folder: &str) -> Result<Vec<RustGameFile>, String> {
    let target = safe_game_folder(dir, folder)?;
    let mut entries = Vec::new();
    for entry in fs::read_dir(target).map_err(|e| format!("Cannot read game folder: {e}"))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() || !(kind.is_file() || kind.is_dir()) {
            continue;
        }
        if entries.len() >= 500 {
            return Err("Folder has more than 500 entries. Open it in Windows Explorer.".into());
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let bytes = if kind.is_file() {
            entry.metadata().map_err(|e| e.to_string())?.len()
        } else { 0 };
        entries.push(RustGameFile { name, bytes, is_directory: kind.is_dir() });
    }
    entries.sort_by(|a, b| b.is_directory.cmp(&a.is_directory)
        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(entries)
}

#[tauri::command]
async fn rust_list_files<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    profile_id: String,
    folder: String,
) -> Result<Vec<RustGameFile>, String> {
    let dir = profile_dir(&base(&app)?, &profile_id)?;
    read_profile(&dir, &profile_id).ok_or("Rust profile does not exist.")?;
    list_game_files(&dir, &folder)
}

#[tauri::command]
async fn rust_open_folder<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    profile_id: String,
    folder: String,
) -> Result<(), String> {
    let dir = profile_dir(&base(&app)?, &profile_id)?;
    read_profile(&dir, &profile_id).ok_or("Rust profile does not exist.")?;
    let target = safe_game_folder(&dir, &folder)?;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("explorer.exe")
            .arg(&target)
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| format!("Cannot open Rust game folder: {e}"))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = target;
        Err("Opening a Rust game folder is currently supported on Windows.".into())
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
    fn only_profile_game_folders_are_openable() {
        let root = std::env::temp_dir().join(format!("fre-rust-folders-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("horizon-mods")).unwrap();
        assert!(safe_game_folder(&root, "root").is_ok());
        assert!(safe_game_folder(&root, "mods").is_ok());
        assert!(safe_game_folder(&root, "..").is_err());
        assert!(safe_game_folder(&root, "../saves").is_err());
        assert!(safe_game_folder(&root, "missing").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn lists_only_allowed_local_files_without_following_links() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path();
        fs::create_dir_all(dir.join("horizon-mods")).unwrap();
        fs::write(dir.join("horizon-mods").join("native-mod.bin"), b"test").unwrap();
        fs::create_dir_all(dir.join("horizon-mods").join("subfolder")).unwrap();
        let items = list_game_files(dir, "mods").unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].name, "subfolder");
        assert!(items[0].is_directory);
        assert_eq!(items[1].bytes, 4);
        assert!(list_game_files(dir, "../mods").is_err());
        assert!(list_game_files(dir, "missing").is_err());
    }

    #[test]
    fn rejects_path_traversal_and_invalid_profiles() {
        assert!(!profile_id_valid("../save"));
        assert!(!profile_id_valid("a/123"));
        assert!(profile_id_valid("123456"));
        assert!(!name_valid(""));
        assert!(!name_valid("bad\nname"));
        assert!(name_valid("Мій Minecraft Rust"));
        assert!(!profile_id_valid("123/../../"));
        assert!(!profile_id_valid("123\\345"));
    }
    #[test]
    fn valid_profile_round_trip_and_corrupt_metadata_are_ignored() {
        let root = tempfile::tempdir().unwrap();
        let dir = profile_dir(root.path(), "123456789").unwrap();
        fs::create_dir_all(&dir).unwrap();
        let expected = RustProfile { id: "123456789".into(), name: "Тест Rust".into(), created: 1234, icon_path: None };
        fs::write(dir.join("profile.json"), serde_json::to_vec(&expected).unwrap()).unwrap();
        assert_eq!(read_profile(&dir, "123456789").unwrap().name, "Тест Rust");
        // A read profile must keep the original name and owner-supplied metadata.
        assert_eq!(read_profile(&dir, "123456789").unwrap().created, 1234);
        fs::write(dir.join("profile.json"), b"garbage").unwrap();
        assert!(read_profile(&dir, "123456789").is_none());
    }
    #[test]
    fn rejects_unsafe_or_oversized_rust_icons() {
        assert!(valid_icon(b"<svg onload=alert(1)>").is_err());
        assert!(valid_icon(&vec![0; MAX_ICON_BYTES + 1]).is_err());
        assert_eq!(valid_icon(b"\x89PNG\r\n\x1a\nDATA").unwrap(), "png");
        assert_eq!(valid_icon(b"RIFFSIZEWEBP1234").unwrap(), "webp");
        assert!(prepare_icon(None, None).unwrap().is_none());
    }
    #[test]
    fn refuses_modified_or_unknown_rust_executable() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(directory.path().join("fre-rust-client.exe"), b"MZ trusted build").unwrap();
        let digest = format!("{:x}", Sha256::digest(b"MZ trusted build"));
        fs::write(directory.path().join("BUILD-INFO.json"), format!(
            "{{\"engine_license\":\"MIT\",\"source_commit\":\"975c07d9151847e6af13afc4b5ab37b6bbe78ef5\",\"game_exe_sha256\":\"{digest}\"}}"
        )).unwrap();
        assert!(verify_native_client(directory.path()).is_ok());
        fs::write(directory.path().join("fre-rust-client.exe"), b"MZ tampered").unwrap();
        assert!(verify_native_client(directory.path()).is_err());
    }
    #[test]
    fn cached_rust_resources_require_correct_version_and_file_hashes() {
        let temp = tempfile::tempdir().unwrap();
        let files = [
            ("facts_sha256", "facts.json", &b"facts"[..]),
            ("block_tags_sha256", "block-tags.json", &b"tags"[..]),
            ("collision_sha256", "cached-collision.nbt", &b"collision"[..]),
        ];
        let mut manifest = serde_json::json!({"schema": 7, "minecraft": "26.3"});
        for (key, name, bytes) in files {
            fs::write(temp.path().join(name), bytes).unwrap();
            manifest[key] = serde_json::Value::String(format!("{:x}", Sha256::digest(bytes)));
        }
        fs::write(
            temp.path().join("horizon-resources.json"),
            serde_json::to_vec(&manifest).unwrap(),
        ).unwrap();
        assert!(resources_valid(temp.path()), "a verified resource cache can be reused");
        fs::write(temp.path().join("facts.json"), b"corrupt").unwrap();
        assert!(!resources_valid(temp.path()), "a corrupt file cannot appear playable");
        fs::write(temp.path().join("facts.json"), b"facts").unwrap();
        assert!(resources_valid(temp.path()));
        manifest["minecraft"] = "26.4".into();
        fs::write(temp.path().join("horizon-resources.json"), serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(!resources_valid(temp.path()), "a resource set for another game version is not valid");
    }

    #[test]
    fn incomplete_resources_are_preserved_for_retry() {
        let temp = tempfile::tempdir().unwrap();
        let old = temp.path().join("prepared");
        fs::create_dir(&old).unwrap();
        fs::write(old.join("world.txt"), b"do not delete").unwrap();
        preserve_incomplete(&old).unwrap();
        assert!(!old.exists());
        let backups: Vec<_> = fs::read_dir(temp.path()).unwrap().flatten().collect();
        assert_eq!(backups.len(), 1);
        assert_eq!(fs::read(backups[0].path().join("world.txt")).unwrap(), b"do not delete");
    }

    #[test]
    fn missing_game_is_not_reported_as_ready() {
        let folder = tempfile::tempdir().unwrap();
        let info = ready(folder.path(), folder.path());
        assert!(!info.installed);
        assert!(!info.prepared);
        assert!(info.reason.is_some());
    }
}
