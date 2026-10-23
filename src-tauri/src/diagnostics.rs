//! Local-only crash and installation diagnostics. No uploads or telemetry.
use std::{fs::{self, OpenOptions}, io::Write, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};
use tauri::{AppHandle, Manager};
use serde::Serialize;

const MAX_VIEW_BYTES: usize = 120 * 1024;
const MAX_LOG_BYTES: u64 = 4 * 1024 * 1024;

fn file(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_log_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("fre-launch-install.log"))
}

/// Error logging is best effort and never allowed to panic or interrupt an installation.
pub fn write(app: &AppHandle, stage: &str, message: &str) {
    let Ok(path) = file(app) else {return;};
    if fs::metadata(&path).map(|m|m.len()>MAX_LOG_BYTES).unwrap_or(false) {
        // Keep the last diagnostics in a single rotating local file.
        let previous = path.with_extension("previous.log");
        let _ = fs::remove_file(&previous);
        let _ = fs::rename(&path, &previous);
    }
    let since_epoch=SystemTime::now().duration_since(UNIX_EPOCH)
        .map(|t|t.as_secs()).unwrap_or(0);
    let compact = message.replace('\n'," ").replace('\r'," ");
    if let Ok(mut out)=OpenOptions::new().append(true).create(true).open(&path) {
        let _=writeln!(out,"[{since_epoch}] [{stage}] {compact}");
    }
}

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct Diagnostics {
    pub log_path: String,
    pub contents: String,
}

#[tauri::command]
pub fn read_diagnostics(app: AppHandle) -> Result<Diagnostics, String> {
    let path=file(&app)?;
    let bytes=fs::read(&path).unwrap_or_default();
    let tail=if bytes.len()>MAX_VIEW_BYTES {
        &bytes[bytes.len()-MAX_VIEW_BYTES..]
    } else { &bytes[..] };
    Ok(Diagnostics{
        log_path:path.to_string_lossy().to_string(),
        contents:String::from_utf8_lossy(tail).to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn log_size_is_bounded() {assert!(MAX_LOG_BYTES>=MAX_VIEW_BYTES as u64);}
}

/// Persists a previous abnormal exit for later review, without telemetry.
#[tauri::command]
pub fn record_interrupted_install(app: AppHandle, version: String) -> Result<(), String> {
    super::minecraft::validate_version(&version)?;
    write(&app, "INSTALL_INTERRUPTED",
        &format!("Previous session ended unexpectedly while installing Minecraft {version}"));
    Ok(())
}
