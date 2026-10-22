use serde::Serialize;
use std::{fs, io::{Read, Write}, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};
use tauri::{AppHandle, Manager};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    pub filename: String,
    pub bytes: u64,
}

fn backup_directory(app: &AppHandle, instance_id: &str) -> Result<PathBuf, String> {
    let _ = super::instances::read(app, instance_id)?;
    let root = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(root.join("backups").join(instance_id))
}

fn saves_directory(app: &AppHandle, instance_id: &str) -> Result<PathBuf, String> {
    let profile = super::instances::read(app, instance_id)?;
    Ok(super::instances::game_directory(app, instance_id, &profile.version)?.join("saves"))
}

fn add_folder(writer: &mut ZipWriter<fs::File>, dir: &Path, root: &Path, count: &mut usize) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
        if metadata.file_type().is_symlink() { continue; }
        *count += 1;
        if *count > 250_000 { return Err("Backup contains too many files".into()); }
        if metadata.is_dir() { add_folder(writer, &path, root, count)?; continue; }
        if !metadata.is_file() { continue; }
        let relative = path.strip_prefix(root).map_err(|e| e.to_string())?;
        let name = relative.to_string_lossy().replace('\\', "/");
        writer.start_file(name, SimpleFileOptions::default().compression_method(CompressionMethod::Deflated))
            .map_err(|e| e.to_string())?;
        let mut source = fs::File::open(&path).map_err(|e| e.to_string())?;
        std::io::copy(&mut source, writer).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn create_world_backup(app: AppHandle, instance_id: String) -> Result<Backup, String> {
    let saves = saves_directory(&app, &instance_id)?;
    if !saves.exists() {return Err("This instance does not have any saved worlds yet".into());}
    let folder = backup_directory(&app, &instance_id)?;
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs();
    let filename = format!("worlds-{timestamp}.zip");
    let target = folder.join(&filename);
    if target.exists() {return Err("A backup with this timestamp already exists".into());}
    let staging = folder.join(format!("{filename}.part"));
    let result = (|| {
        let file = fs::File::create(&staging).map_err(|e| e.to_string())?;
        let mut writer = ZipWriter::new(file);
        let mut count = 0;
        add_folder(&mut writer, &saves, &saves, &mut count)?;
        writer.finish().map_err(|e| e.to_string())?;
        if count == 0 {return Err("No files found in saves directory".into());}
        fs::rename(&staging, &target).map_err(|e| e.to_string())?;
        Ok(Backup {filename, bytes: fs::metadata(&target).map_err(|e| e.to_string())?.len()})
    })();
    if result.is_err() { let _ = fs::remove_file(staging); }
    result
}

#[tauri::command]
pub fn list_world_backups(app: AppHandle, instance_id: String) -> Result<Vec<Backup>, String> {
    let folder = backup_directory(&app, &instance_id)?;
    if !folder.exists() {return Ok(vec![]);}
    let mut items = Vec::new();
    for entry in fs::read_dir(&folder).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let filename = entry.file_name().to_string_lossy().to_string();
        if filename.starts_with("worlds-") && filename.ends_with(".zip") && entry.file_type().map_err(|e| e.to_string())?.is_file() {
            items.push(Backup {filename, bytes: entry.metadata().map_err(|e| e.to_string())?.len()});
        }
    }
    items.sort_by(|a,b|b.filename.cmp(&a.filename));
    Ok(items)
}

/// Restores worlds under uniquely named folders. Never overwrites existing worlds.
#[tauri::command]
pub fn restore_world_backup(app: AppHandle, instance_id: String, filename: String) -> Result<String, String> {
    if !filename.starts_with("worlds-") || !filename.ends_with(".zip")
       || filename.len() > 80 || !filename.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b,b'-'|b'.')) {
        return Err("Invalid backup filename".into());
    }
    let path = backup_directory(&app, &instance_id)?.join(&filename);
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| e.to_string())?;
    if archive.len() > 250_000 {return Err("Backup contains too many files".into());}
    let saves = saves_directory(&app, &instance_id)?;
    fs::create_dir_all(&saves).map_err(|e| e.to_string())?;
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs();
    let mut restored = 0usize;
    let mut total_size = 0u64;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let mode = entry.unix_mode().unwrap_or(0);
        if mode & 0o170000 == 0o120000 {return Err("Backup contains a symlink".into());}
        let relative = entry.enclosed_name().ok_or("Unsafe backup path")?.to_path_buf();
        let mut components = relative.components();
        let first = components.next().ok_or("Empty backup entry")?;
        let world = first.as_os_str().to_string_lossy();
        if world.is_empty() || world == "." || world == ".." {return Err("Invalid world name".into());}
        let folder = format!("FRE-Restored-{stamp}-{world}");
        let mut output = saves.join(folder);
        for component in components {
            if let std::path::Component::Normal(name) = component {output.push(name);}
            else {return Err("Unsafe archive entry".into());}
        }
        if entry.is_dir() {fs::create_dir_all(&output).map_err(|e| e.to_string())?;continue;}
        total_size = total_size.saturating_add(entry.size());
        if total_size > 20 * 1024 * 1024 * 1024u64 {return Err("Backup exceeds restore size limit".into());}
        if output.exists() {return Err("Restore destination already exists; existing worlds were not overwritten".into());}
        if let Some(parent) = output.parent() {fs::create_dir_all(parent).map_err(|e| e.to_string())?;}
        let mut file = fs::File::create(output).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut file).map_err(|e| e.to_string())?;
        file.flush().map_err(|e| e.to_string())?;
        restored += 1;
    }
    Ok(format!("Restored {restored} files into new world folders (no existing worlds replaced)"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn safe_backup_names() {
        let filename = "worlds-1234.zip";
        assert!(filename.starts_with("worlds-") && filename.ends_with(".zip"));
    }
}
