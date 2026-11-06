//! Native instance data: session accounting, bounded logs and validated quick play.
use super::*;
use std::{
    io::{Read, Seek, SeekFrom},
    time::Instant,
};
pub(super) static SESSIONS: LazyLock<tokio::sync::Mutex<HashMap<String, Session>>> =
    LazyLock::new(|| tokio::sync::Mutex::new(HashMap::new()));
#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Playtime {
    pub seconds: u64,
    pub last_played: Option<u64>,
}
pub(super) fn read_playtime(dir: &Path) -> Result<Playtime, String> {
    let path = dir.join("fre-playtime.json");
    if !path.try_exists().map_err(|e| e.to_string())? {
        return Ok(Playtime::default());
    }
    regular_file(&path, 8192)?;
    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
#[derive(Clone)]
pub(super) struct Session {
    started: Instant,
    base: u64,
    last_played: u64,
}
impl Session {
    pub fn start(dir: &Path) -> Result<Self, String> {
        let stats = read_playtime(dir)?;
        let session = Self {
            started: Instant::now(),
            base: stats.seconds,
            last_played: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };
        session.save(dir)?;
        Ok(session)
    }
    pub fn total(&self) -> u64 {
        self.base.saturating_add(self.started.elapsed().as_secs())
    }
    pub fn save(&self, dir: &Path) -> Result<(), String> {
        let path = dir.join("fre-playtime.json");
        if path.symlink_metadata().is_ok() {
            regular_file(&path, 8192)?;
        }
        let mut pending = tempfile::NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
        use std::io::Write;
        serde_json::to_writer(
            &mut pending,
            &Playtime {
                seconds: self.total(),
                last_played: Some(self.last_played),
            },
        )
        .map_err(|e| e.to_string())?;
        pending
            .flush()
            .and_then(|()| pending.as_file().sync_all())
            .map_err(|e| e.to_string())?;
        pending.persist(path).map_err(|e| e.to_string())?;
        Ok(())
    }
}
fn regular_file(path: &Path, limit: u64) -> Result<(), String> {
    let meta = path.symlink_metadata().map_err(|e| e.to_string())?;
    if !meta.file_type().is_file() || meta.len() > limit {
        return Err("Unsafe or oversized instance file.".into());
    }
    Ok(())
}
fn child_dir(parent: &Path, name: &str) -> Result<PathBuf, String> {
    let mut parts = Path::new(name).components();
    if name.contains(['\\', '/'])
        || !matches!(
            (parts.next(), parts.next()),
            (Some(std::path::Component::Normal(_)), None)
        )
    {
        return Err("Choose one world folder.".into());
    }
    let root = parent.canonicalize().map_err(|e| e.to_string())?;
    let path = root.join(name);
    if !path
        .symlink_metadata()
        .is_ok_and(|m| m.file_type().is_dir())
    {
        return Err("World folder is missing or is a link.".into());
    }
    let real = path.canonicalize().map_err(|e| e.to_string())?;
    if real.parent() != Some(root.as_path()) {
        return Err("World folder leaves this instance.".into());
    }
    Ok(real)
}
pub(super) fn validate_destination(
    dir: &Path,
    world: Option<&str>,
    server: Option<&str>,
) -> Result<(), String> {
    if world.is_some() && server.is_some() {
        return Err("Choose a world or a server.".into());
    }
    if let Some(name) = world {
        regular_file(
            &child_dir(&safe_game_folder(dir, "saves")?, name)?.join("level.dat"),
            32 * 1024 * 1024,
        )?;
    }
    if let Some(address) = server {
        theseus::worlds::fre_validate_native_address(address).map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn log_dir(dir: &Path) -> Result<PathBuf, String> {
    safe_game_folder(dir, "logs")
}
fn log_file(dir: &Path, name: &str) -> Result<PathBuf, String> {
    if name.is_empty()
        || name.len() > 160
        || name.starts_with('.')
        || name.contains(['\\', '/', ':'])
        || !(name.ends_with(".log") || name.ends_with(".log.gz"))
    {
        return Err("Choose a local .log or .log.gz file.".into());
    }
    let path = log_dir(dir)?.join(name);
    regular_file(&path, 256 * 1024 * 1024)?;
    Ok(path)
}
pub(super) fn read_log(dir: &Path, name: &str) -> Result<String, String> {
    if name == "latest.log" && !dir.join("logs/latest.log").exists() {
        return Ok(String::new());
    }
    let path = log_file(dir, name)?;
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    const TAIL: usize = 2 * 1024 * 1024;
    const MAX_EXPANDED: u64 = 64 * 1024 * 1024;
    let (data, trimmed) = if name.ends_with(".gz") {
        // The game rolls latest.log into dated .log.gz files. Keep only a
        // bounded tail while streaming; a compressed bomb cannot allocate
        // the entire expanded archive in the launcher.
        let mut decoder = flate2::read::GzDecoder::new(file);
        let mut tail = std::collections::VecDeque::with_capacity(TAIL);
        let mut total = 0u64;
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let count = decoder.read(&mut buffer).map_err(|e| e.to_string())?;
            if count == 0 {
                break;
            }
            total += count as u64;
            if total > MAX_EXPANDED {
                return Err("Expanded log exceeds 64 MiB; open it locally.".into());
            }
            let overflow = (tail.len() + count).saturating_sub(TAIL);
            if overflow > 0 {
                tail.drain(..overflow);
            }
            tail.extend(buffer[..count].iter().copied());
        }
        (tail.into_iter().collect::<Vec<u8>>(), total > TAIL as u64)
    } else {
        let len = file.metadata().map_err(|e| e.to_string())?.len();
        let offset = len.saturating_sub(TAIL as u64);
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| e.to_string())?;
        let mut data = Vec::new();
        file.take(TAIL as u64)
            .read_to_end(&mut data)
            .map_err(|e| e.to_string())?;
        (data, offset > 0)
    };
    let text = String::from_utf8_lossy(&data).into_owned();
    Ok(if trimmed {
        text.split_once('\n')
            .map(|(_, tail)| tail.to_owned())
            .unwrap_or_default()
    } else {
        text
    })
}
#[tauri::command]
pub(super) async fn rust_log_sources<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    profile_id: String,
) -> Result<Vec<String>, String> {
    let dir = profile_dir(&base(&app)?, &profile_id)?;
    read_profile(&dir, &profile_id).ok_or("Profile not found.")?;
    if !dir.join("logs").exists() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(log_dir(&dir)?).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if log_file(&dir, &name).is_ok() {
            names.push(name);
        }
        if names.len() > 500 {
            break;
        }
    }
    names.sort_by(|a, b| b.cmp(a));
    Ok(names)
}
#[tauri::command]
pub(super) async fn rust_worlds<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    profile_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let dir = profile_dir(&base(&app)?, &profile_id)?;
    read_profile(&dir, &profile_id).ok_or("Profile not found.")?;
    let mut worlds = Vec::new();
    let saves = safe_game_folder(&dir, "saves")?;
    for entry in fs::read_dir(&saves).map_err(|e| e.to_string())?.take(500) {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(path) = child_dir(&saves, &name) else {
            continue;
        };
        if regular_file(&path.join("level.dat"), 32 * 1024 * 1024).is_err() {
            continue;
        }
        match theseus::worlds::fre_read_native_world(path.clone()).await {
            Ok(world)=>{
                let icon=path.join("icon.png");
                if regular_file(&icon,2*1024*1024).is_ok() && icon.canonicalize().ok().as_deref().and_then(|p|p.parent())==Some(path.as_path()) {let _=app.asset_protocol_scope().allow_file(&icon);}
                worlds.push(serde_json::to_value(world).map_err(|e|e.to_string())?);
            },
            Err(_)=>worlds.push(serde_json::json!({"type":"singleplayer","path":name,"name":name,"game_mode":"survival","hardcore":false,"locked":false,"display_status":"normal"})),
        }
    }
    let servers = dir.join("servers.dat");
    if servers.exists() {
        regular_file(&servers, 8 * 1024 * 1024)?;
        worlds.extend(
            theseus::worlds::fre_read_native_servers(&servers)
                .await
                .map_err(|e| e.to_string())?,
        );
    }
    Ok(worlds)
}
#[tauri::command]
pub(super) async fn rust_server_edit<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    profile_id: String,
    action: String,
    name: String,
    address: String,
) -> Result<(), String> {
    let _guard = PROFILE_CREATION.lock().await;
    if RUNNING_PROFILES.lock().await.contains(&profile_id) {
        return Err("Close the game before editing saved servers.".into());
    }
    let dir = profile_dir(&base(&app)?, &profile_id)?;
    read_profile(&dir, &profile_id).ok_or("Profile not found.")?;
    if !matches!(action.as_str(), "add" | "remove") || !name_valid(&name) {
        return Err("Enter a server name.".into());
    }
    theseus::worlds::fre_validate_native_address(&address).map_err(|e| e.to_string())?;
    edit_server_file(&dir, &action, &name, &address).await
}

async fn edit_server_file(
    dir: &Path,
    action: &str,
    name: &str,
    address: &str,
) -> Result<(), String> {
    let source = dir.join("servers.dat");
    if source.symlink_metadata().is_ok() {
        regular_file(&source, 8 * 1024 * 1024)?;
    }
    // Theseus already writes its target atomically. Close the placeholder
    // handle before asking it to replace that path: Windows rejects replacing
    // an open NamedTempFile even though a later persist would close it.
    let pending = tempfile::NamedTempFile::new_in(dir)
        .map_err(|e| e.to_string())?
        .into_temp_path();
    theseus::worlds::fre_edit_native_server(&source, &pending, action, name, address)
        .await
        .map_err(|e| e.to_string())?;
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&pending)
        .and_then(|file| file.sync_all())
        .map_err(|e| e.to_string())?;
    pending.persist(source).map_err(|e| e.to_string())?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn server_edit_uses_a_closed_staging_file_and_preserves_the_saved_list() {
        let dir = tempfile::tempdir().unwrap();
        edit_server_file(dir.path(), "add", "First", "localhost:25565")
            .await
            .unwrap();
        edit_server_file(dir.path(), "add", "Second", "example.org")
            .await
            .unwrap();
        let source = dir.path().join("servers.dat");
        let before = fs::read(&source).unwrap();
        assert!(
            edit_server_file(dir.path(), "add", "Duplicate", "example.org")
                .await
                .is_err()
        );
        assert_eq!(fs::read(&source).unwrap(), before);
        edit_server_file(dir.path(), "remove", "First", "localhost:25565")
            .await
            .unwrap();
        let records = theseus::worlds::fre_read_native_servers(&source)
            .await
            .unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["address"], "example.org");
        assert_eq!(
            fs::read_dir(dir.path()).unwrap().count(),
            1,
            "temporary files must be cleaned up"
        );
    }
    #[tokio::test]
    async fn saved_native_servers_use_the_games_nbt_file_and_preserve_other_servers() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("servers.dat");
        let staged = dir.path().join("pending.dat");
        theseus::worlds::fre_edit_native_server(
            &source,
            &staged,
            "add",
            "Local game",
            "localhost:25565",
        )
        .await
        .unwrap();
        fs::rename(&staged, &source).unwrap();
        theseus::worlds::fre_edit_native_server(
            &source,
            &staged,
            "add",
            "Second server",
            "example.org",
        )
        .await
        .unwrap();
        fs::rename(&staged, &source).unwrap();
        let worlds = theseus::worlds::fre_read_native_servers(&source)
            .await
            .unwrap();
        assert_eq!(worlds.len(), 2);
        assert_eq!(worlds[0]["name"], "Local game");
        assert!(theseus::worlds::fre_edit_native_server(
            &source,
            &staged,
            "add",
            "Duplicate",
            "example.org"
        )
        .await
        .is_err());
        theseus::worlds::fre_edit_native_server(
            &source,
            &staged,
            "remove",
            "Local game",
            "localhost:25565",
        )
        .await
        .unwrap();
        fs::rename(&staged, &source).unwrap();
        let worlds = theseus::worlds::fre_read_native_servers(&source)
            .await
            .unwrap();
        assert_eq!(worlds.len(), 1);
        assert_eq!(worlds[0]["address"], "example.org");
    }
    #[test]
    fn quick_play_rejects_paths_missing_worlds_and_conflicting_targets() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("saves")).unwrap();
        for world in ["../other", "..", "C:\\other", "one/two", "missing"] {
            assert!(validate_destination(dir.path(), Some(world), None).is_err());
        }
        fs::create_dir(dir.path().join("saves/Valid")).unwrap();
        fs::write(dir.path().join("saves/Valid/level.dat"), b"world").unwrap();
        assert!(validate_destination(dir.path(), Some("Valid"), None).is_ok());
        assert!(validate_destination(dir.path(), Some("Valid"), Some("example.org")).is_err());
        assert!(validate_destination(dir.path(), None, Some("bad\n--token")).is_err());
    }
    #[test]
    fn session_stats_preserve_profile_and_world_data() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("profile.json"), b"keep").unwrap();
        let first = Session::start(dir.path()).unwrap();
        let mut later = first.clone();
        later.base = 70;
        later.save(dir.path()).unwrap();
        let read = read_playtime(dir.path()).unwrap();
        assert!(read.seconds >= 70);
        assert!(read.last_played.is_some());
        assert_eq!(fs::read(dir.path().join("profile.json")).unwrap(), b"keep");
    }
    #[test]
    fn reads_the_games_gzip_archives_and_rejects_expansion_bombs() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("logs")).unwrap();
        let archive = dir.path().join("logs/2026-11-05-1.log.gz");
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder
            .write_all(b"[INFO]: yesterday\n[ERROR]: archived failure\n")
            .unwrap();
        fs::write(&archive, encoder.finish().unwrap()).unwrap();
        assert!(read_log(dir.path(), "2026-11-05-1.log.gz")
            .unwrap()
            .contains("archived failure"));
        fs::write(&archive, b"not gzip").unwrap();
        assert!(read_log(dir.path(), "2026-11-05-1.log.gz").is_err());
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        let block = vec![b'x'; 1024 * 1024];
        for _ in 0..65 {
            encoder.write_all(&block).unwrap();
        }
        fs::write(&archive, encoder.finish().unwrap()).unwrap();
        assert!(read_log(dir.path(), "2026-11-05-1.log.gz")
            .unwrap_err()
            .contains("64 MiB"));
    }
    #[test]
    fn logs_are_bounded_and_refuse_traversal() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("logs")).unwrap();
        assert_eq!(read_log(dir.path(), "latest.log").unwrap(), "");
        fs::write(
            dir.path().join("logs/latest.log"),
            b"[INFO]: hello\n[WARN]: warning",
        )
        .unwrap();
        assert!(read_log(dir.path(), "latest.log")
            .unwrap()
            .contains("warning"));
        for name in ["../profile.json", "C:\\latest.log", ".hidden.log"] {
            assert!(read_log(dir.path(), name).is_err());
        }
    }
}
