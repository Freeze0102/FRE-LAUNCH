//! FRE LAUNCH's own Minecraft 26.3 asset preparation path.
//! Reuses the MIT-licensed native preparer BUNDLED WITH FRE LAUNCH.
//! No Horizon Launcher, Horizon account, external launcher CLI or
//! proprietary Minecraft data is bundled or downloaded from third parties.
//! The caller must first verify ownership using FRE's Microsoft session.
use std::{fs, io::Read, path::{Component, Path, PathBuf}, time::Duration};
use reqwest::Client;
use serde_json::Value;
use sha1::{Digest, Sha1};
use sha2::Sha256;
use tauri::Emitter;
use tokio::{io::AsyncWriteExt, process::Command, task::JoinSet};

const GAME_VERSION: &str = "26.3";
const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const MAX_JSON: usize = 12 * 1024 * 1024;
const MAX_GAME_FILE: u64 = 200 * 1024 * 1024;
const MAX_ASSET: u64 = 32 * 1024 * 1024;
const MAX_SOURCE_FILES: usize = 512;

fn sha1_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha1::digest(bytes))
}
fn valid_sha1(hash: &str) -> bool {
    hash.len() == 40 && hash.bytes().all(|b| b.is_ascii_hexdigit())
}
fn verified_file(path: &Path, sha1: &str) -> bool {
    if !valid_sha1(sha1) || !path.symlink_metadata().is_ok_and(|m| m.file_type().is_file()) {
        return false;
    }
    let Ok(mut file) = fs::File::open(path) else { return false };
    let mut hasher = Sha1::new();
    let mut buffer = [0u8; 65536];
    loop {
        let Ok(n) = file.read(&mut buffer) else { return false };
        if n == 0 { break; }
        hasher.update(&buffer[..n]);
    }
    format!("{:x}", hasher.finalize()).eq_ignore_ascii_case(sha1)
}
fn trusted_mojang_url(url: &str) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|_| "Invalid Mojang download URL.")?;
    if parsed.scheme() != "https" || parsed.username() != "" || parsed.password().is_some()
        || !matches!(parsed.host_str(),
            Some("piston-meta.mojang.com" | "piston-data.mojang.com" |
                 "resources.download.minecraft.net" | "libraries.minecraft.net"))
        || parsed.port().is_some() || parsed.fragment().is_some()
    {
        return Err("Refusing a game asset URL outside Mojang's HTTPS servers.".into());
    }
    Ok(())
}
fn safe_relative(path: &str) -> Result<PathBuf, String> {
    if path.len() > 320 || path.contains('\\') || path.contains(':') || path.is_empty() {
        return Err("Invalid path in official Minecraft manifest.".into());
    }
    let p = Path::new(path);
    if !p.components().all(|part| matches!(part, Component::Normal(_))) {
        return Err("Unexpected relative game asset path.".into());
    }
    Ok(p.to_path_buf())
}
fn safe_name(name: &str) -> bool {
    !name.is_empty() && name.len() <= 50
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        && name != "." && name != ".."
}
fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value.get(key).and_then(Value::as_str)
        .ok_or_else(|| format!("Mojang metadata is missing {key}."))
}
fn allowed_hash(lock: &Value, name: &str, alternative: &str, hash: &str) -> bool {
    let game = &lock["minecraft"];
    game[name].as_str() == Some(hash)
        || game[alternative].as_array().is_some_and(|accepted|
            accepted.iter().any(|v| v.as_str() == Some(hash)))
}

async fn json(client: &Client, url: &str) -> Result<(Vec<u8>, Value), String> {
    trusted_mojang_url(url)?;
    let mut response = client.get(url).send().await.map_err(|e| format!("Mojang download failed: {e}"))?
        .error_for_status().map_err(|e| format!("Mojang returned an HTTP error: {e}"))?;
    if response.content_length().is_some_and(|size| size > MAX_JSON as u64) {
        return Err("Mojang metadata exceeds the allowed size.".into());
    }
    // A missing/misleading HTTP Content-Length must never allow unlimited
    // allocation before checking the real metadata size.
    let mut bytes = Vec::<u8>::new();
    while let Some(chunk) = response.chunk().await
        .map_err(|e| format!("Cannot read Mojang metadata: {e}"))? {
        if bytes.len().checked_add(chunk.len()).is_none_or(|size| size > MAX_JSON) {
            return Err("Mojang metadata is too large.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|e| format!("Invalid Mojang version data: {e}"))?;
    Ok((bytes, value))
}

/// Download into FRE-owned cache only; never overwrite a user's existing
/// Minecraft launcher, world, mod, account or configuration files.
async fn download_to(client: &Client, url: &str, dest: &Path, expected: &str, limit: u64)
    -> Result<(), String>
{
    trusted_mojang_url(url)?;
    if !valid_sha1(expected) { return Err("Invalid SHA-1 in Mojang version metadata.".into()); }
    if verified_file(dest, expected) { return Ok(()); }
    let parent = dest.parent().ok_or("Cannot locate Minecraft cache parent.")?;
    fs::create_dir_all(parent).map_err(|e| format!("Cannot create FRE Minecraft cache: {e}"))?;
    let tmp = dest.with_extension("fre-download");
    if tmp.exists() { fs::remove_file(&tmp).map_err(|e| format!("Cannot restart incomplete download: {e}"))?; }
    let response = client.get(url).send().await
        .map_err(|e| format!("Minecraft download failed: {e}"))?
        .error_for_status().map_err(|e| format!("Minecraft CDN returned an error: {e}"))?;
    if response.content_length().is_some_and(|size| size > limit) {
        return Err("Minecraft download exceeded its expected size limit.".into());
    }
    let mut body = response;
    let mut file = tokio::fs::File::create(&tmp).await.map_err(|e| e.to_string())?;
    let mut hasher = Sha1::new();
    let mut count: u64 = 0;
    let result: Result<(), String> = async {
        while let Some(chunk) = body.chunk().await.map_err(|e| format!("Minecraft download interrupted: {e}"))? {
            count = count.checked_add(chunk.len() as u64).ok_or("Minecraft download length overflow.")?;
            if count > limit { return Err("Minecraft download exceeded its size limit.".into()); }
            hasher.update(&chunk);
            file.write_all(&chunk).await.map_err(|e| format!("Cannot write game cache: {e}"))?;
        }
        file.flush().await.map_err(|e| e.to_string())?;
        if !format!("{:x}", hasher.finalize()).eq_ignore_ascii_case(expected) {
            return Err("Minecraft download checksum mismatch.".into());
        }
        Ok(())
    }.await;
    drop(file);
    if let Err(error) = result {
        let _ = fs::remove_file(&tmp);
        return Err(error);
    }
    if dest.exists() {
        // Cache files only: never the player's installed Minecraft directory.
        let meta = fs::symlink_metadata(dest).map_err(|e| e.to_string())?;
        if !meta.file_type().is_file() { return Err("Unsafe cached asset path.".into()); }
        fs::remove_file(dest).map_err(|e| e.to_string())?;
    }
    fs::rename(&tmp, dest).map_err(|e| format!("Cannot finish game asset download: {e}"))
}

fn bounded_copy_tree(source: &Path, target: &Path) -> Result<(), String> {
    if !source.is_dir() { return Err("Bundled Rust exporter sources are missing.".into()); }
    let mut pending = vec![(source.to_path_buf(), target.to_path_buf())];
    let mut total = 0usize;
    while let Some((from, to)) = pending.pop() {
        fs::create_dir_all(&to).map_err(|e| e.to_string())?;
        for entry in fs::read_dir(from).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let filetype = entry.file_type().map_err(|e| e.to_string())?;
            if filetype.is_symlink() { return Err("Unsafe link in bundled Rust exporter.".into()); }
            let path = entry.path();
            let dst = to.join(entry.file_name());
            if filetype.is_dir() {
                pending.push((path, dst));
            } else if filetype.is_file() {
                total += 1;
                if total > MAX_SOURCE_FILES || path.metadata().map_err(|e| e.to_string())?.len() > 1024 * 1024 {
                    return Err("Unexpectedly large bundled Rust exporter.".into());
                }
                if path.extension().is_none_or(|ext| ext != "java") {
                    return Err("Unexpected non-Java file in Rust exporter source.".into());
                }
                if dst.exists() && !dst.symlink_metadata().is_ok_and(|m| m.file_type().is_file()) {
                    return Err("Unsafe existing FRE Rust exporter cache.".into());
                }
                fs::copy(&path, &dst).map_err(|e| format!("Could not stage Rust exporter: {e}"))?;
            } else {
                return Err("Unexpected bundled Rust exporter file type.".into());
            }
        }
    }
    Ok(())
}

fn prepare_root(bundle: &Path, data: &Path) -> Result<PathBuf, String> {
    let root = data.join("native-preparer");
    if root.exists() && !root.symlink_metadata().is_ok_and(|m| m.file_type().is_dir()) {
        return Err("Unsafe native resource preparer directory.".into());
    }
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    for relative in [
        "research-lock.json",
        "crates/horizon-prepare/Cargo.toml",
        "crates/horizon-prepare/build.rs",
    ] {
        let input = bundle.join(relative);
        let output = root.join(relative);
        let m = input.symlink_metadata().map_err(|e| format!("Missing bundled preparer source: {e}"))?;
        if !m.file_type().is_file() || m.len() > 1024 * 1024 {
            return Err("Invalid bundled Rust preparer input.".into());
        }
        if let Some(parent) = output.parent() { fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
        if output.exists() && !output.symlink_metadata().is_ok_and(|m| m.file_type().is_file()) {
            return Err("Unsafe FRE Rust preparer cache file.".into());
        }
        fs::copy(&input, &output).map_err(|e| format!("Cannot stage Rust preparer: {e}"))?;
    }
    // Match the pinned preparer's source checksum: Cargo.toml + build.rs
    // + all Rust src/*.rs, not merely the exporter implementation.
    let crate_src = bundle.join("crates/horizon-prepare/src");
    let target_src = root.join("crates/horizon-prepare/src");
    let crate_meta = crate_src.symlink_metadata().map_err(|e| format!("Missing native source snapshot: {e}"))?;
    if !crate_meta.file_type().is_dir() {
        return Err("Native preparer source snapshot is not an ordinary directory.".into());
    }
    fs::create_dir_all(&target_src).map_err(|e| e.to_string())?;
    let mut rust_sources = 0usize;
    for entry in fs::read_dir(&crate_src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if !meta.file_type().is_file() || path.extension().is_none_or(|ext| ext != "rs")
            || meta.len() > 1024 * 1024 {
            return Err("Invalid file in bundled Rust preparer sources.".into());
        }
        let dst = target_src.join(entry.file_name());
        if dst.exists() && !dst.symlink_metadata().is_ok_and(|m| m.file_type().is_file()) {
            return Err("Unsafe native Rust source cache path.".into());
        }
        fs::copy(path, dst).map_err(|e| format!("Cannot stage Rust source: {e}"))?;
        rust_sources += 1;
        if rust_sources > 40 { return Err("Oversized native Rust source snapshot.".into()); }
    }
    if rust_sources < 10 { return Err("Native Rust source snapshot is incomplete.".into()); }
    let parity = "mods/horizon/src/test/java/dev/horizonlab/parity";
    bounded_copy_tree(&bundle.join(parity), &root.join(parity))?;
    Ok(root)
}
fn verified_preparer(bundle: &Path) -> Result<PathBuf, String> {
    let data = fs::read(bundle.join("BUILD-INFO.json")).map_err(|e| e.to_string())?;
    if data.len() > 8192 { return Err("Rust build manifest is oversized.".into()); }
    let json: Value = serde_json::from_slice(&data).or_else(|_|
        serde_json::from_slice(data.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&data)))
        .map_err(|e| format!("Invalid build provenance: {e}"))?;
    let hash = required_string(&json, "prepare_exe_sha256")?;
    if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid native preparer digest.".into());
    }
    let exe = bundle.join("fre-rust-prepare.exe");
    let m = exe.symlink_metadata().map_err(|_| "FRE LAUNCH build does not contain the native resource preparer.")?;
    if !m.file_type().is_file() { return Err("Unsafe native preparer executable.".into()); }
    let mut hasher = Sha256::new();
    let mut file = fs::File::open(&exe).map_err(|e| e.to_string())?;
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 { break; }
        hasher.update(&buffer[..n]);
    }
    if !format!("{:x}", hasher.finalize()).eq_ignore_ascii_case(hash) {
        return Err("Bundled native preparer integrity check failed.".into());
    }
    Ok(exe)
}

async fn acquire_game(client: &Client, data: &Path, lock: &Value) -> Result<PathBuf, String> {
    let minecraft = data.join("minecraft-26.3-cache");
    fs::create_dir_all(&minecraft).map_err(|e| e.to_string())?;

    // The official version index only supplies an address and expected SHA-1.
    // The pinned research lock independently restricts which game version
    // manifest and Minecraft client/server binaries can be accepted.
    let (_, version_index) = json(client, MANIFEST_URL).await?;
    let versions = version_index["versions"].as_array().ok_or("Mojang versions index is missing.")?;
    let version = versions.iter().find(|v| v["id"] == GAME_VERSION)
        .ok_or("Minecraft 26.3 was not found in Mojang's version index.")?;
    let version_url = required_string(version, "url")?;
    let version_sha = required_string(version, "sha1")?;
    if !allowed_hash(lock, "manifest_sha1", "accepted_manifest_sha1", version_sha) {
        return Err("Mojang's Minecraft 26.3 version JSON is not pinned by this FRE Rust build.".into());
    }
    let version_path = minecraft.join("versions").join(GAME_VERSION).join("26.3.json");
    download_to(client, version_url, &version_path, version_sha, MAX_JSON as u64).await?;
    let raw = fs::read(&version_path).map_err(|e| e.to_string())?;
    let game: Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    if game["id"] != GAME_VERSION { return Err("Wrong Minecraft version manifest.".into()); }

    for (part, pinned) in [("client", "client_sha1"), ("server", "server_sha1")] {
        let descriptor = &game["downloads"][part];
        let hash = required_string(descriptor, "sha1")?;
        if lock["minecraft"][pinned].as_str() != Some(hash) {
            return Err(format!("Wrong pinned Minecraft {part} binary."));
        }
        let url = required_string(descriptor, "url")?;
        let destination = if part == "client" {
            minecraft.join("versions/26.3/26.3.jar")
        } else {
            minecraft.join("server-26.3.jar")
        };
        download_to(client, url, &destination, hash, MAX_GAME_FILE).await?;
    }

    let libraries = game["libraries"].as_array().ok_or("Mojang game libraries are missing.")?;
    for library in libraries {
        let downloads = &library["downloads"];
        let artifact = &downloads["artifact"];
        if let (Some(path), Some(url), Some(hash)) = (
            artifact["path"].as_str(), artifact["url"].as_str(), artifact["sha1"].as_str()
        ) {
            let destination = minecraft.join("libraries").join(safe_relative(path)?);
            download_to(client, url, &destination, hash, MAX_GAME_FILE).await?;
        }
    }

    let index = &game["assetIndex"];
    let id = required_string(index, "id")?;
    if !safe_name(id) { return Err("Unsafe Mojang asset index name.".into()); }
    let index_hash = required_string(index, "sha1")?;
    if !allowed_hash(lock, "asset_index_sha1", "accepted_asset_index_sha1", index_hash) {
        return Err("Minecraft asset index does not match the verified 26.3 pins.".into());
    }
    let index_url = required_string(index, "url")?;
    let index_path = minecraft.join("assets/indexes").join(format!("{id}.json"));
    download_to(client, index_url, &index_path, index_hash, MAX_JSON as u64).await?;
    let raw = fs::read(&index_path).map_err(|e| e.to_string())?;
    let assets: Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    let objects = assets["objects"].as_object().ok_or("Minecraft asset index has no objects.")?;

    // Limit network parallelism and reject unsafe entries. Each object URL is
    // assembled from its signed hash, not from user-controlled registry hosts.
    let mut downloads = JoinSet::new();
    let mut unique = std::collections::HashSet::<String>::new();
    for object in objects.values() {
        let hash = required_string(object, "hash")?;
        if !valid_sha1(hash) { return Err("Invalid object hash in Minecraft assets.".into()); }
        if !unique.insert(hash.to_owned()) { continue; }
        let prefix = &hash[..2];
        let file = minecraft.join("assets/objects").join(prefix).join(hash);
        if verified_file(&file, hash) { continue; }
        let url = format!("https://resources.download.minecraft.net/{prefix}/{hash}");
        let http = client.clone();
        let expected = hash.to_owned();
        downloads.spawn(async move { download_to(&http, &url, &file, &expected, MAX_ASSET).await });
        if downloads.len() >= 12 {
            downloads.join_next().await.ok_or("Missing Minecraft asset task.")?
                .map_err(|e| format!("Minecraft asset task failed: {e}"))??;
        }
    }
    while let Some(result) = downloads.join_next().await {
        result.map_err(|e| format!("Minecraft asset task failed: {e}"))??;
    }
    Ok(minecraft)
}

/// The owned game's resources are generated by FRE LAUNCH's own *bundled*
/// MIT helper. The only account used is the one FRE already authenticated.
/// The preparer may require an installed Java 25 JDK for its vanilla exporters.
pub async fn prepare<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    bundle: &Path,
    data: &Path,
) -> Result<PathBuf, String> {
    let exe = verified_preparer(bundle)?;
    let root = prepare_root(bundle, data)?;
    let lock: Value = serde_json::from_slice(&fs::read(root.join("research-lock.json"))
        .map_err(|e| e.to_string())?).map_err(|e| format!("Invalid Minecraft lock: {e}"))?;
    let _ = app.emit("fre-rust-prepare-stage", "local");
    let client = Client::builder()
        .user_agent("FRE-LAUNCH/0.5.10 native-minecraft-setup")
        .timeout(Duration::from_secs(120))
        .redirect(reqwest::redirect::Policy::none())
        .build().map_err(|e| e.to_string())?;
    let _ = app.emit("fre-rust-prepare-stage", "download");
    let minecraft = acquire_game(&client, data, &lock).await?;
    let output = root.join(".local/horizon-client/26.3");
    let _ = app.emit("fre-rust-prepare-stage", "prepare");
    let mut command = Command::new(exe);
    command
        .arg("--root").arg(&root)
        .arg("--output").arg(&output)
        .arg("--minecraft-dir").arg(&minecraft)
        .arg("--server-jar").arg(minecraft.join("server-26.3.jar"))
        .arg("--prepare")
        .arg("--complete")
        .arg("--progress").arg("json")
        .current_dir(&root)
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let output_status = tokio::time::timeout(Duration::from_secs(90 * 60), command.output())
        .await.map_err(|_| "Native Minecraft resource preparation timed out.")?
        .map_err(|e| format!("Could not run FRE's bundled native preparer: {e}"))?;
    if !output_status.status.success() {
        let stderr = String::from_utf8_lossy(&output_status.stderr);
        let stdout = String::from_utf8_lossy(&output_status.stdout);
        let tail = format!("{stdout}\n{stderr}").chars().rev().take(1800).collect::<String>()
            .chars().rev().collect::<String>();
        return Err(format!("Native Minecraft preparation failed: {tail}"));
    }
    if !super::resources_valid(&output) {
        return Err("Bundled FRE Rust preparer did not produce a verified 26.3 resource cache.".into());
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn refuses_external_minecraft_asset_urls_and_traversal() {
        assert!(trusted_mojang_url("https://piston-data.mojang.com/test.jar").is_ok());
        assert!(trusted_mojang_url("https://example.com/evil").is_err());
        assert!(trusted_mojang_url("http://piston-data.mojang.com/test").is_err());
        assert!(safe_relative("net/minecraft/client/26.3/client.jar").is_ok());
        assert!(safe_relative("../world/saves").is_err());
        assert!(safe_relative("C:\\temp\\game.jar").is_err());
        assert!(!valid_sha1("not-a-hash"));
    }
}