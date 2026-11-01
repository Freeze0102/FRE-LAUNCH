//! Pinned official Windows launcher. No shell, installer, arbitrary URL or archive paths.
use sha2::{Digest, Sha256};
use std::{io::{Cursor, Read, Write}, path::{Path, PathBuf}, time::Duration};
use tauri_plugin_http::reqwest::Client;

pub const VERSION: &str = "0.2.33";
const URL: &str = "https://github.com/horizon-rust/horizon/releases/download/launcher%400.2.33/horizon-launcher-windows-x64.zip";
const ZIP_BYTES: usize = 15_024_354;
const ZIP_SHA256: &str = "590aa11b6d653ac58dcb1bd3149d22d485d2bc215f4eb912b4c79a710ee7f571";
const EXE_BYTES: usize = 42_396_672;
const EXE_SHA256: &str = "a395dd0a75ed7f085838d46d7e847ee57901fd4d1691848c46f82c15ae03833a";

fn digest(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }

pub fn cached(directory: &Path) -> Option<PathBuf> {
    let exe = directory.join("horizon-launcher.exe");
    if std::fs::metadata(&exe).ok()?.len() != EXE_BYTES as u64 { return None; }
    let bytes = std::fs::read(&exe).ok()?;
    (digest(&bytes) == EXE_SHA256).then_some(exe)
}

fn unpack(bytes: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() != ZIP_BYTES || digest(bytes) != ZIP_SHA256 {
        return Err("Horizon download failed its SHA-256 check. Nothing was installed.".into());
    }
    unpack_verified(bytes)
}

fn unpack_verified(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    if archive.len() != 1 { return Err("Unexpected official Horizon archive layout.".into()); }
    let entry = archive.by_index(0).map_err(|e| e.to_string())?;
    if entry.name() != "horizon-launcher.exe" || entry.is_dir() || entry.size() != EXE_BYTES as u64 {
        return Err("Unexpected official Horizon executable.".into());
    }
    let mut exe = Vec::with_capacity(EXE_BYTES);
    entry.take(EXE_BYTES as u64 + 1).read_to_end(&mut exe).map_err(|e| e.to_string())?;
    if exe.len() != EXE_BYTES || !exe.starts_with(b"MZ") || digest(&exe) != EXE_SHA256 {
        return Err("Horizon executable failed its integrity check.".into());
    }
    Ok(exe)
}

pub async fn install(directory: &Path, mut progress: impl FnMut(u8) + Send) -> Result<PathBuf, String> {
    if let Some(exe) = cached(directory) { progress(100); return Ok(exe); }
    let target = directory.join("horizon-launcher.exe");
    if target.exists() {
        return Err("The cached Horizon launcher changed. Select that existing EXE manually, or choose the official download instructions. It was not overwritten.".into());
    }
    let client = Client::builder().user_agent("FRE-LAUNCH Horizon setup")
        .connect_timeout(Duration::from_secs(20)).timeout(Duration::from_secs(300))
        .build().map_err(|e| e.to_string())?;
    let mut response = client.get(URL).send().await.map_err(|e| e.to_string())?
        .error_for_status().map_err(|e| e.to_string())?;
    if response.content_length().is_some_and(|size| size != ZIP_BYTES as u64) {
        return Err("Unexpected Horizon download size.".into());
    }
    let mut bytes = Vec::with_capacity(ZIP_BYTES);
    let mut last = 0;
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if bytes.len().saturating_add(chunk.len()) > ZIP_BYTES { return Err("Horizon download is too large.".into()); }
        bytes.extend_from_slice(&chunk);
        let percent = (bytes.len() * 90 / ZIP_BYTES) as u8;
        if percent != last { progress(percent); last = percent; }
    }
    let exe = tokio::task::spawn_blocking(move || unpack(&bytes)).await.map_err(|e| e.to_string())??;
    progress(95);
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let mut staged = tempfile::NamedTempFile::new_in(directory).map_err(|e| e.to_string())?;
    staged.write_all(&exe).map_err(|e| e.to_string())?;
    staged.flush().map_err(|e| e.to_string())?;
    staged.persist_noclobber(&target).map_err(|e| e.to_string())?;
    progress(100);
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_corrupt_or_unexpected_download_without_writing() {
        assert!(unpack(b"MZ not a verified official ZIP").is_err());
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("horizon-launcher.exe"), b"unrelated file").unwrap();
        assert!(cached(directory.path()).is_none());
    }
    #[test]
    fn archive_paths_are_never_extracted() {
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut bytes);
            zip.start_file("../horizon-launcher.exe", zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(b"MZ").unwrap();
            zip.finish().unwrap();
        }
        assert!(unpack_verified(bytes.get_ref()).is_err());
    }
    #[tokio::test]
    async fn changed_cache_is_never_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("horizon-launcher.exe");
        std::fs::write(&target, b"keep this file").unwrap();
        assert!(install(directory.path(), |_| {}).await.is_err());
        assert_eq!(std::fs::read(target).unwrap(), b"keep this file");
    }
}
