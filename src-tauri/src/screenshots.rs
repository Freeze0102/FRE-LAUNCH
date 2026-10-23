//! A safe, instance-scoped screenshot gallery. The frontend never receives arbitrary paths.
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;
use std::{fs, path::{Component, Path, PathBuf}, time::UNIX_EPOCH};
use tauri::AppHandle;

const MAX_SCREENSHOT_BYTES: u64 = 24 * 1024 * 1024;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotInfo {
    pub filename: String,
    pub size_bytes: u64,
    pub modified_at: u64,
}

fn screenshot_folder(app: &AppHandle, instance_id: &str) -> Result<PathBuf, String> {
    let instance = super::instances::read(app, instance_id)?;
    Ok(super::instances::game_directory(app, instance_id, &instance.version)?.join("screenshots"))
}

fn check_filename(filename: &str) -> Result<(), String> {
    if filename.is_empty() || filename.len() > 250 || filename.chars().any(char::is_control) {
        return Err("Invalid screenshot filename".into());
    }
    let path = Path::new(filename);
    if path.components().count() != 1 ||
        !matches!(path.components().next(), Some(Component::Normal(_))) ||
        path.extension().and_then(|x| x.to_str()).map(|x| x.eq_ignore_ascii_case("png")) != Some(true)
    {
        return Err("Only a single PNG filename inside this instance is allowed".into());
    }
    Ok(())
}

fn verified_image(path: &Path) -> Result<Vec<u8>, String> {
    let meta = fs::symlink_metadata(path).map_err(|e| format!("Screenshot not accessible: {e}"))?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err("Screenshot must be a regular file".into());
    }
    if meta.len() > MAX_SCREENSHOT_BYTES {
        return Err("Screenshot is too large to preview (24 MB limit)".into());
    }
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    if bytes.len() < 24 || &bytes[0..8] != b"\x89PNG\r\n\x1a\n" {
        return Err("File is not a valid PNG image".into());
    }
    // PNG dimensions live in the IHDR chunk. Bound decompression by the WebView.
    if &bytes[12..16] != b"IHDR" {
        return Err("PNG IHDR metadata is missing".into());
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().map_err(|_| "Invalid PNG width")?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().map_err(|_| "Invalid PNG height")?);
    if width == 0 || height == 0 || width > 8192 || height > 8192 ||
        (width as u64) * (height as u64) > 20_000_000 {
        return Err("Screenshot dimensions are unsafe or unsupported".into());
    }
    Ok(bytes)
}

#[tauri::command]
pub fn list_screenshots(app: AppHandle, instance_id: String) -> Result<Vec<ScreenshotInfo>, String> {
    let dir = screenshot_folder(&app, &instance_id)?;
    if !dir.exists() {return Ok(Vec::new());}
    let mut items = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if check_filename(&name).is_err() {continue;}
        let metadata = fs::symlink_metadata(entry.path()).map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {continue;}
        let modified_at = metadata.modified().ok()
            .and_then(|date| date.duration_since(UNIX_EPOCH).ok()).map(|t|t.as_secs()).unwrap_or(0);
        items.push(ScreenshotInfo{filename:name,size_bytes:metadata.len(),modified_at});
    }
    items.sort_by(|a,b| b.modified_at.cmp(&a.modified_at).then_with(||b.filename.cmp(&a.filename)));
    items.truncate(500);
    Ok(items)
}

#[tauri::command]
pub fn read_screenshot(app: AppHandle, instance_id: String, filename: String) -> Result<String, String> {
    check_filename(&filename)?;
    let path = screenshot_folder(&app, &instance_id)?.join(filename);
    let data = verified_image(&path)?;
    Ok(format!("data:image/png;base64,{}", STANDARD.encode(data)))
}

#[tauri::command]
pub fn delete_screenshot(app: AppHandle, instance_id: String, filename: String) -> Result<(), String> {
    check_filename(&filename)?;
    let path = screenshot_folder(&app, &instance_id)?.join(filename);
    // Refuse to follow symlinks even if the path was changed after listing.
    let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("Refusing to delete a non-regular file".into());
    }
    fs::remove_file(path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn does_not_accept_traversal() {
        assert!(check_filename("2026-10-23_12.00.00.png").is_ok());
        assert!(check_filename("../secret.png").is_err());
        assert!(check_filename(r"C:\secret.png").is_err());
        assert!(check_filename("test.jpg").is_err());
        assert!(check_filename("test.png/foo").is_err());
    }
    #[test]
    fn png_header_is_verified_before_webview_preview() {
        let tmp = std::env::temp_dir().join(format!("fre-test-{}.png",uuid::Uuid::new_v4()));
        fs::write(&tmp, b"not a png").expect("write temporary file");
        assert!(verified_image(&tmp).is_err());
        fs::remove_file(&tmp).expect("remove temporary file");
    }
}
