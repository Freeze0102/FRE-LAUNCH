use futures::stream::{self, StreamExt};
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::{collections::HashSet, path::{Path, PathBuf}, process::Stdio, sync::atomic::{AtomicBool, Ordering}, time::Duration};
use tauri::{AppHandle, Emitter, Manager};
use tokio::{fs, io::{AsyncBufReadExt, BufReader}, process::Command};
use uuid::Uuid;

static RUNNING: AtomicBool = AtomicBool::new(false);
const MANIFEST: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress { stage: String, current: usize, total: usize, message: String }

fn progress(app: &AppHandle, stage: &str, current: usize, total: usize, message: impl Into<String>) {
    let _ = app.emit("install-progress", Progress { stage: stage.into(), current, total, message: message.into() });
}

pub(super) fn validate_version(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 100 || !id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-')) || id == "." || id == ".." {
        return Err("Invalid Minecraft version identifier".into());
    }
    Ok(())
}

fn root(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app.path().app_data_dir().map_err(|e| e.to_string())?.join("minecraft"))
}

fn client() -> Result<Client, String> {
    super::downloads::client()
}

fn allowed_url(url: &str) -> Result<(), String> {
    super::downloads::trusted_url(url)
}

async fn fetch_json(client: &Client, url: &str) -> Result<Value, String> {
    allowed_url(url)?;
    client.get(url).send().await.map_err(|e| e.to_string())?
        .error_for_status().map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

fn matches_sha(bytes: &[u8], expected: &str) -> bool {
    format!("{:x}", Sha1::digest(bytes)).eq_ignore_ascii_case(expected)
}

async fn download_one(client: &Client, url: &str, path: &Path, sha1: Option<&str>) -> Result<(), String> {
    super::downloads::download_one(client, url, path, sha1).await
}

fn os_rules(item: &Value) -> bool {
    let Some(rules) = item.get("rules").and_then(Value::as_array) else { return true; };
    let mut accepted = false;
    for rule in rules {
        if rule.get("features").is_some() { continue; }
        let os = rule.get("os");
        let name_match = os.and_then(|v| v.get("name")).and_then(Value::as_str)
            .map(|name| name == "windows").unwrap_or(true);
        let arch_match = os.and_then(|v| v.get("arch")).and_then(Value::as_str)
            .map(|arch| match arch { "x86" => cfg!(target_arch = "x86"), "x86_64" => cfg!(target_arch = "x86_64"), _ => false })
            .unwrap_or(true);
        if name_match && arch_match {
            accepted = rule.get("action").and_then(Value::as_str) == Some("allow");
        }
    }
    accepted
}

fn sha_from(item: &Value) -> Option<&str> { item.get("sha1").and_then(Value::as_str) }

fn safe_relative(s: &str) -> Result<PathBuf, String> {
    let path = Path::new(s);
    if path.components().any(|c| !matches!(c, std::path::Component::Normal(_))) {
        return Err("Invalid relative download path".into());
    }
    Ok(path.to_path_buf())
}

async fn version_meta(client: &Client, version: &str) -> Result<Value, String> {
    validate_version(version)?;
    let manifest = fetch_json(client, MANIFEST).await?;
    let entry = manifest["versions"].as_array().ok_or("Missing versions array")?
        .iter().find(|v| v["id"].as_str() == Some(version)).ok_or("Minecraft version not found")?;
    let url = value_str(entry, "url")?;
    let details = fetch_json(client, url).await?;
    if details.get("inheritsFrom").is_some() {
        return Err("Inherited versions are not supported by the vanilla installer yet".into());
    }
    Ok(details)
}

/// Legacy vanilla and Fabric libraries may use Maven coordinates instead of
/// a modern "downloads.artifact" block.
fn maven_library(lib: &Value, base: &Path) -> Result<Option<(String, PathBuf, Option<String>, bool)>, String> {
    let Some(name) = lib["name"].as_str() else { return Ok(None); };
    let coords: Vec<&str> = name.split(':').collect();
    if coords.len() < 3 || coords.len() > 4 || coords.iter().any(|part|
        part.is_empty() || !part.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c,b'.'|b'_'|b'-'|b'+'))
    ) {return Err("Invalid Maven library coordinates".into());}
    let group = coords[0].replace('.', "/");
    let artifact = coords[1];
    let version = coords[2];
    let classifier = if coords.len() == 4 { format!("-{}", coords[3]) } else { String::new() };
    let relative = format!("{group}/{artifact}/{version}/{artifact}-{version}{classifier}.jar");
    let relative = safe_relative(&relative)?;
    let repository = lib["url"].as_str().unwrap_or("https://libraries.minecraft.net/");
    let url = format!("{}/{}",repository.trim_end_matches('/'),relative.to_string_lossy().replace('\\',"/"));
    allowed_url(&url)?;
    Ok(Some((url,base.join("libraries").join(relative),None,false)))
}

fn libraries(metadata: &Value, base: &Path) -> Result<Vec<(String, PathBuf, Option<String>, bool)>, String> {
    let mut result = Vec::new();
    for lib in metadata["libraries"].as_array().ok_or("Missing libraries array")? {
        if !os_rules(lib) { continue; }
        let Some(downloads) = lib.get("downloads") else {
            if let Some(artifact) = maven_library(lib, base)? {result.push(artifact);}
            continue;
        };
        if let Some(artifact) = downloads.get("artifact") {
            let path = safe_relative(value_str(artifact, "path")?)?;
            result.push((value_str(artifact, "url")?.to_string(), base.join("libraries").join(path), sha_from(artifact).map(str::to_string), false));
        }
        if let Some(classifiers) = downloads.get("classifiers") {
            let key = lib["natives"]["windows"].as_str().unwrap_or("");
            let key = key.replace("${arch}", if cfg!(target_arch = "x86_64") { "64" } else { "32" });
            if let Some(archive) = classifiers.get(&key) {
                let path = safe_relative(value_str(archive, "path")?)?;
                result.push((value_str(archive, "url")?.to_string(), base.join("libraries").join(path), sha_from(archive).map(str::to_string), true));
            }
        }
    }
    Ok(result)
}

fn extract_natives(archives: &[PathBuf], dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for archive in archives {
        let file = std::fs::File::open(archive).map_err(|e| e.to_string())?;
        let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
            let Some(relative) = entry.enclosed_name() else { continue; };
            if relative.components().any(|c| c.as_os_str().to_string_lossy().eq_ignore_ascii_case("META-INF")) { continue; }
            let target = dest.join(relative);
            if entry.is_dir() { std::fs::create_dir_all(&target).map_err(|e| e.to_string())?; continue; }
            if let Some(parent) = target.parent() { std::fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
            let mut output = std::fs::File::create(target).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut output).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

// Ancient Minecraft versions require virtual or legacy resource directories.
// Asset object names are untrusted metadata: reject parent components.
async fn materialize_legacy_assets(base: &Path, index_name: &str, index_data: &Value, game_dir: Option<&Path>) -> Result<(), String> {
    validate_version(index_name)?;
    let virtual_assets = index_data["virtual"].as_bool().unwrap_or(false);
    let map_resources = index_data["map_to_resources"].as_bool().unwrap_or(false);
    if !virtual_assets && !(map_resources && game_dir.is_some()) {return Ok(());}
    let objects = index_data["objects"].as_object().ok_or("Missing legacy asset objects")?;
    for (logical_name, details) in objects {
        let relative = safe_relative(logical_name)?;
        let hash = details["hash"].as_str().ok_or("Missing asset SHA-1")?;
        if hash.len()!=40 || !hash.bytes().all(|byte|byte.is_ascii_hexdigit()) {
            return Err("Invalid legacy asset hash".into());
        }
        let object = base.join("assets").join("objects").join(&hash[..2]).join(hash);
        let virtual_dir = base.join("assets").join("virtual").join(index_name);
        for target_base in [virtual_assets.then_some(virtual_dir), if map_resources {game_dir.map(|dir|dir.join("resources"))}else{None}]
            .into_iter().flatten() {
            let target = target_base.join(&relative);
            if let Some(parent) = target.parent(){fs::create_dir_all(parent).await.map_err(|e|e.to_string())?;}
            if fs::try_exists(&target).await.map_err(|e|e.to_string())? {
                if super::downloads::verify_file(&target,hash).await? {continue;}
            }
            fs::copy(&object,&target).await.map_err(|e|format!("Legacy resource {}: {e}",logical_name))?;
        }
    }
    Ok(())
}

async fn install(app: &AppHandle, version: &str) -> Result<Value, String> {
    let http = client()?;
    let base = root(app)?;
    progress(app, "metadata", 0, 1, "Отримання метаданих Mojang");
    let meta = version_meta(&http, version).await?;
    let versions = base.join("versions").join(version);
    fs::create_dir_all(&versions).await.map_err(|e| e.to_string())?;
    let json_path = versions.join(format!("{version}.json"));
    fs::write(&json_path, serde_json::to_vec_pretty(&meta).map_err(|e| e.to_string())?)
        .await.map_err(|e| e.to_string())?;

    progress(app, "client", 0, 1, "Завантаження Minecraft client");
    let jar = meta["downloads"]["client"].as_object().ok_or("Client download not found")?;
    let client_url = jar.get("url").and_then(Value::as_str).ok_or("Missing client download URL")?;
    let client_hash = jar.get("sha1").and_then(Value::as_str);
    download_one(&http, client_url, &versions.join(format!("{version}.jar")), client_hash).await?;

    let libs = libraries(&meta, &base)?;
    let native_archives: Vec<PathBuf> = libs.iter().filter(|(_, _, _, native)| *native).map(|(_,path,_,_)|path.clone()).collect();
    let total_libs = libs.len();
    for (index, (url, path, sha, _)) in libs.iter().enumerate() {
        download_one(&http, url, path, sha.as_deref()).await?;
        progress(app, "libraries", index + 1, total_libs, format!("Бібліотеки: {}/{}", index+1,total_libs));
    }

    let asset_index = &meta["assetIndex"];
    if let (Some(id), Some(url)) = (asset_index["id"].as_str(),asset_index["url"].as_str()) {
        validate_version(id)?;
        let path = base.join("assets").join("indexes").join(format!("{id}.json"));
        download_one(&http, url, &path, sha_from(asset_index)).await?;
        let index_data: Value = serde_json::from_slice(&fs::read(&path).await.map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let objects = index_data["objects"].as_object().ok_or("Invalid assets index")?;
        let mut seen = HashSet::new();
        let mut files = Vec::new();
        for asset in objects.values() {
            let Some(hash) = asset["hash"].as_str() else { continue; };
            if hash.len() != 40 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) { return Err("Invalid asset hash".into()); }
            if !seen.insert(hash.to_string()) { continue; }
            let dest = base.join("assets").join("objects").join(&hash[..2]).join(hash);
            let url = format!("https://resources.download.minecraft.net/{}/{}", &hash[..2], hash);
            files.push((url, dest, hash.to_string()));
        }
        let count = files.len();
        let mut downloads = stream::iter(files.into_iter().map(|(url,dest,hash)| {
            let http = http.clone();
            async move { download_one(&http, &url, &dest, Some(&hash)).await }
        })).buffer_unordered(12);
        let mut finished = 0usize;
        while let Some(result) = downloads.next().await {
            result?;
            finished += 1;
            if finished % 25 == 0 || finished == count {
                progress(app,"assets",finished,count,format!("Ресурси: {finished}/{count}"));
            }
        }
        materialize_legacy_assets(&base, id, &index_data, None).await?;
    }

    let natives_dest = base.join("instances").join(version).join("natives");
    tokio::task::spawn_blocking(move || extract_natives(&native_archives, &natives_dest))
        .await.map_err(|e| e.to_string())??;
    fs::create_dir_all(base.join("instances").join(version)).await.map_err(|e| e.to_string())?;
    fs::write(versions.join(".installed"), b"ok").await.map_err(|e| e.to_string())?;
    progress(app, "complete", 1, 1, "Minecraft встановлено");
    Ok(meta)
}

#[tauri::command]
pub async fn install_version(app: AppHandle, version: String) -> Result<String, String> {
    validate_version(&version)?;
    install(&app, &version).await?;
    Ok(format!("Minecraft {version} installed successfully"))
}

#[tauri::command]
pub async fn is_installed(app: AppHandle, version: String) -> Result<bool, String> {
    validate_version(&version)?;
    Ok(fs::try_exists(root(&app)?.join("versions").join(&version).join(".installed"))
        .await.map_err(|e| e.to_string())?)
}

#[tauri::command]
pub fn java_info() -> Result<String, String> {
    let output = std::process::Command::new("java").arg("-version").output()
        .map_err(|e| format!("Java не знайдено в PATH: {e}"))?;
    if !output.status.success() { return Err("Java завершила перевірку з помилкою".into()); }
    let text = String::from_utf8_lossy(&output.stderr);
    Ok(text.lines().next().unwrap_or("Java знайдено").to_string())
}

fn java_major(raw: &str) -> Option<u32> {
    let quoted = raw.split('"').nth(1)?;
    let nums: Vec<&str> = quoted.split('.').collect();
    if nums.first() == Some(&"1") { nums.get(1)?.parse().ok() }
    else { nums.first()?.parse().ok() }
}

fn jvm_arg_replace(s: &str, base: &Path, natives: &Path, classpath: &str) -> String {
    s.replace("${natives_directory}", &natives.to_string_lossy())
     .replace("${launcher_name}", "FRE_LAUNCH")
     .replace("${launcher_version}", env!("CARGO_PKG_VERSION"))
     .replace("${classpath}", classpath)
     .replace("${library_directory}", &base.join("libraries").to_string_lossy())
     .replace("${classpath_separator}", if cfg!(windows) { ";" } else { ":" })
}

fn game_arg_replace(s: &str, base: &Path, version: &str, dir: &Path, name: &str, uuid: &str, meta: &Value) -> String {
    let assets = base.join("assets");
    s.replace("${auth_player_name}", name)
     .replace("${version_name}", version)
     .replace("${game_directory}", &dir.to_string_lossy())
     .replace("${assets_root}", &assets.to_string_lossy())
     .replace("${game_assets}", &assets.to_string_lossy())
     .replace("${assets_index_name}", meta["assets"].as_str().unwrap_or("legacy"))
     .replace("${auth_uuid}", uuid)
     .replace("${auth_access_token}", "0")
     .replace("${auth_session}", "0")
     .replace("${user_type}", "legacy")
     .replace("${version_type}", meta["type"].as_str().unwrap_or("release"))
     .replace("${user_properties}", "{}")
     .replace("${auth_xuid}", "0")
     .replace("${clientid}", "")
     .replace("${resolution_width}", "1280")
     .replace("${resolution_height}", "720")
}

fn parse_arguments(arr: &Value) -> Vec<String> {
    let mut result = Vec::new();
    if let Some(items) = arr.as_array() {
        for item in items {
            if let Some(s) = item.as_str() { result.push(s.into()); }
            else if os_rules(item) {
                if let Some(s) = item["value"].as_str() { result.push(s.into()); }
                else if let Some(vals) = item["value"].as_array() {
                    result.extend(vals.iter().filter_map(Value::as_str).map(str::to_string));
                }
            }
        }
    }
    result
}

fn stable_uuid(name: &str) -> String {
    let hash = md5::compute(format!("OfflinePlayer:{name}"));
    let mut bytes = hash.0;
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes).to_string()
}

#[tauri::command]
pub async fn launch_game(app: AppHandle, version: String, nickname: String, memory_mb: u32, instance_id: Option<String>) -> Result<String, String> {
    validate_version(&version)?;
    if nickname.is_empty() || nickname.len()>16 || !nickname.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        return Err("Нікнейм: 1–16 латинських літер, цифр або _".into());
    }
    if !(512..=32768).contains(&memory_mb) { return Err("RAM must be between 512 and 32768 MB".into()); }
    if RUNNING.swap(true, Ordering::SeqCst) { return Err("Minecraft уже запускається або працює".into()); }
    let result = launch_inner(app.clone(), version, nickname, memory_mb, instance_id).await;
    if result.is_err() { RUNNING.store(false, Ordering::SeqCst); }
    result
}

async fn launch_inner(app: AppHandle, version: String, nickname: String, memory_mb: u32, instance_id: Option<String>) -> Result<String, String> {
    let base = root(&app)?;
    let cached_metadata = base.join("versions").join(&version).join(format!("{version}.json"));
    // Installed games must remain launchable without contacting Mojang.
    // A manual reinstall explicitly rechecks the downloaded files.
    let mut meta = if fs::try_exists(base.join("versions").join(&version).join(".installed"))
        .await.map_err(|e| e.to_string())? && fs::try_exists(&cached_metadata).await.map_err(|e| e.to_string())? {
        let bytes = fs::read(&cached_metadata).await.map_err(|e| e.to_string())?;
        serde_json::from_slice::<Value>(&bytes).map_err(|e| e.to_string())?
    } else {
        install(&app, &version).await?
    };
    let profile = if let Some(id) = instance_id.as_deref() { Some(super::instances::read(&app, id)?) } else { None };
    if let Some(profile) = &profile {
        if profile.version != version { return Err("Instance version mismatch".into()); }
    }
    if let Some(instance) = &profile {
        if instance.loader == "fabric" {
            let loader_version = instance.loader_version.as_deref()
                .ok_or("Fabric instance has no loader version")?;
            progress(&app, "fabric", 0, 1, format!("Preparing Fabric {loader_version}"));
            let fabric_profile = super::fabric::load_profile(&base, &version, loader_version).await?;
            super::fabric::merge_profile(&mut meta, &fabric_profile)?;
            // Fabric libraries come from the official Maven repository and
            // are cached alongside Mojang dependencies. No installer JAR is executed.
            let http = client()?;
            let fabric_libraries = libraries(&fabric_profile, &base)?;
            let total = fabric_libraries.len();
            for (index, (url, dest, hash, _)) in fabric_libraries.iter().enumerate() {
                download_one(&http, url, dest, hash.as_deref()).await?;
                progress(&app, "fabric", index + 1, total,
                    format!("Fabric libraries: {}/{}", index + 1, total));
            }
        } else if instance.loader != "vanilla" {
            return Err("Selected modloader is not yet supported".into());
        }
    }

    // Prefer an explicit instance override. Otherwise select a compatible
    // Java runtime from already installed JDKs and Minecraft-managed runtimes.
    let chosen_runtime = if profile.as_ref().and_then(|p| p.java_path.as_ref()).is_some() {
        None
    } else {
        let required = meta["javaVersion"]["majorVersion"].as_u64().unwrap_or(8) as u32;
        super::java::list_java_runtimes().into_iter()
            .find(|runtime| runtime.major == required)
            .map(|runtime| runtime.path)
    };
    let java_bin = profile.as_ref().and_then(|p| p.java_path.as_deref())
        .or(chosen_runtime.as_deref()).unwrap_or("java");
    let java_output = std::process::Command::new(java_bin).arg("-version").output()
        .map_err(|e| format!("Java executable not found: {e}"))?;
    if !java_output.status.success() { return Err("Java version check failed".into()); }
    let java = String::from_utf8_lossy(&java_output.stderr).lines().next().unwrap_or("").to_owned();
    if let Some(required) = meta["javaVersion"]["majorVersion"].as_u64() {
        let actual = java_major(&java).ok_or("Не вдалося визначити версію Java")?;
        if actual != required as u32 {
            return Err(format!("Minecraft {version} потребує Java {required}, але {java_bin} має Java {actual}. Обери правильний шлях у FRE JAVA MANAGER."));
        }
    }
    let dir = if let Some(id) = instance_id.as_deref() {
        super::instances::game_directory(&app, id, &version)?
    } else { base.join("instances").join(&version) };
    fs::create_dir_all(&dir).await.map_err(|e| e.to_string())?;
    if let (Some(index_name), Some(_)) = (meta["assetIndex"]["id"].as_str(),meta["assetIndex"]["url"].as_str()) {
        validate_version(index_name)?;
        let asset_index = base.join("assets").join("indexes").join(format!("{index_name}.json"));
        if fs::try_exists(&asset_index).await.map_err(|e|e.to_string())? {
            let bytes = fs::read(asset_index).await.map_err(|e|e.to_string())?;
            let index_data: Value = serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;
            materialize_legacy_assets(&base,index_name,&index_data,Some(&dir)).await?;
        }
    }
    let mut cp = Vec::new();
    for (_, path, _, native) in libraries(&meta, &base)? {
        if !native { cp.push(path.to_string_lossy().to_string()); }
    }
    cp.push(base.join("versions").join(&version).join(format!("{version}.jar")).to_string_lossy().to_string());
    let classpath = cp.join(if cfg!(windows) { ";" } else { ":" });
    // The launcher passes a compact 32-digit Mojang-style UUID argument.
    let uuid = stable_uuid(&nickname).replace('-', "");
    let natives = dir.join("natives");
    let native_libraries: Vec<PathBuf> = libraries(&meta, &base)?
        .into_iter().filter(|(_, _, _, native)| *native)
        .map(|(_, path, _, _)| path).collect();
    let natives_for_extract = natives.clone();
    tokio::task::spawn_blocking(move || extract_natives(&native_libraries, &natives_for_extract))
        .await.map_err(|e| format!("Native extraction failed: {e}"))??;
    let mut jvm = parse_arguments(&meta["arguments"]["jvm"]);
    if jvm.is_empty() {
        jvm = vec!["-Djava.library.path=${natives_directory}".into(), "-cp".into(), "${classpath}".into()];
    }
    let mut args = vec![format!("-Xmx{memory_mb}M"), "-Xms512M".into()];
    args.extend(jvm.iter().map(|s| jvm_arg_replace(s, &base, &natives, &classpath)));
    args.push(value_str(&meta,"mainClass")?.into());

    let game_args = if let Some(legacy) = meta["minecraftArguments"].as_str() {
        legacy.split_whitespace().map(str::to_string).collect::<Vec<String>>()
    } else { parse_arguments(&meta["arguments"]["game"]) };
    args.extend(game_args.iter().map(|s| game_arg_replace(s, &base, &version, &dir, &nickname, &uuid, &meta)));

    let mut command = Command::new(java_bin);
    command.args(&args).current_dir(&dir).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true);
    let mut child = command.spawn().map_err(|e| format!("Не вдалося запустити Java: {e}"))?;
    let pid = child.id().unwrap_or(0);
    let _ = app.emit("game-status", "running");
    if let Some(stdout) = child.stdout.take() {
        let output_app = app.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await { let _ = output_app.emit("game-log", line); }
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let output_app = app.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await { let _ = output_app.emit("game-log", line); }
        });
    }
    tokio::spawn(async move {
        let result = child.wait().await;
        let message = match result {
            Ok(exit) => format!("Minecraft exited with code {:?}", exit.code()),
            Err(e) => format!("Minecraft process error: {e}"),
        };
        let _ = app.emit("game-log", message.clone());
        let _ = app.emit("game-status", message);
        RUNNING.store(false, Ordering::SeqCst);
    });
    Ok(format!("Minecraft {version} launched (PID {pid})"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn version_path_is_safe() {
        assert!(validate_version("1.21.4").is_ok());
        assert!(validate_version("../evil").is_err());
        assert!(validate_version("..").is_err());
    }
    #[test]
    fn offline_uuid_is_stable() {
        assert_eq!(stable_uuid("Player"), stable_uuid("Player"));
        assert_ne!(stable_uuid("Player"), stable_uuid("Other"));
    }
    #[test]
    fn sha1_check_detects_tampering() {
        assert!(matches_sha(b"hello", "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"));
        assert!(!matches_sha(b"world", "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"));
    }
    #[test]
    fn java_version_detection() {
        assert_eq!(java_major("openjdk version \"21.0.2\""), Some(21));
        assert_eq!(java_major("java version \"1.8.0_421\""), Some(8));
    }
}
