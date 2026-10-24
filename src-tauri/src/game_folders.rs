//! Safe, instance-scoped game folders and Windows Explorer integration.
//! Keep the active version and all existing worlds in their original location.
use serde::Serialize;
use std::{fs, path::{Path, PathBuf}};
use tauri::{AppHandle, Manager};

const GAME_SUBFOLDERS: [(&str,&str); 8] = [
    ("mods","Моди"), ("saves","Світи"),
    ("resourcepacks","Ресурс-паки"), ("shaderpacks","Шейдери"),
    ("screenshots","Скріншоти"), ("config","Налаштування модів"),
    ("logs","Журнали гри"), ("datapacks","Датапаки"),
];

#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct GameFolder {
    pub key: String,
    pub name: String,
    pub path: String,
    pub exists: bool,
}

fn valid_kind(kind: &str) -> bool {
    kind=="root" || GAME_SUBFOLDERS.iter().any(|(key,_)|*key==kind)
}

fn ensure_real_folder(path: &Path) -> Result<(),String> {
    if let Ok(meta) = fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() || !meta.is_dir() {
            return Err(format!("Каталог має бути звичайною папкою, а не посиланням: {}",path.display()));
        }
    } else {
        fs::create_dir(path).map_err(|e|format!("Не вдалося створити {}: {e}",path.display()))?;
    }
    Ok(())
}

// The Minecraft root is the same one used by minecraft::launch_inner.
// No migration is attempted, so existing worlds remain where players left them.
fn game_root(app:&AppHandle, instance_id:Option<&str>, version:&str)->Result<PathBuf,String>{
    super::minecraft::validate_version(version)?;
    if let Some(id)=instance_id.filter(|x|!x.is_empty()) {
        super::instances::game_directory(app,id,version)
    } else {
        let base=app.path().app_data_dir().map_err(|e|e.to_string())?;
        Ok(base.join("minecraft").join("instances").join(version))
    }
}

pub(super) fn prepare_folders(base:&Path)->Result<(),String>{
    let parent=base.parent().ok_or("Invalid game data folder")?;
    // Reject reparse-point root and instance folders before opening Explorer.
    if !parent.exists(){fs::create_dir_all(parent).map_err(|e|e.to_string())?;}
    ensure_real_folder(parent)?;
    ensure_real_folder(base)?;
    for (key,_) in GAME_SUBFOLDERS {
        ensure_real_folder(&base.join(key))?;
    }
    Ok(())
}

#[tauri::command]
pub fn list_game_folders(app:AppHandle,instance_id:Option<String>,version:String)->Result<Vec<GameFolder>,String>{
    let base=game_root(&app,instance_id.as_deref(),&version)?;
    prepare_folders(&base)?;
    let root=GameFolder{
        key:"root".into(), name:"Папка гри".into(),
        path:base.display().to_string(),exists:true,
    };
    let others=GAME_SUBFOLDERS.into_iter().map(|(key,name)|GameFolder{
        key:key.into(),name:name.into(),path:base.join(key).display().to_string(),exists:true,
    });
    Ok(std::iter::once(root).chain(others).collect())
}

#[tauri::command]
pub fn open_game_folder(app:AppHandle,instance_id:Option<String>,version:String,kind:String)->Result<String,String>{
    if !valid_kind(&kind){return Err("Невідома папка гри".into());}
    let base=game_root(&app,instance_id.as_deref(),&version)?;
    prepare_folders(&base)?;
    let target=if kind=="root"{base}else{base.join(&kind)};
    ensure_real_folder(&target)?;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // No cmd.exe, no PowerShell and no shell script evaluation.
        std::process::Command::new("explorer.exe")
            .arg(&target)
            .creation_flags(0x08000000)
            .spawn().map_err(|e|format!("Провідник Windows не відкрився: {e}"))?;
    }
    #[cfg(not(windows))]
    return Err("Відкриття Провідника підтримується лише на Windows".into());
    Ok(target.display().to_string())
}


/// Read-only list of actual Minecraft worlds in an instance.
/// Do not recurse into worlds or follow symlinks/reparse points.
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct WorldSummary {
    pub name: String,
    pub last_modified: u64,
    pub bytes_on_disk: u64,
}
#[tauri::command]
pub fn list_instance_worlds(app:AppHandle,instance_id:String)->Result<Vec<WorldSummary>,String>{
    use std::time::UNIX_EPOCH;
    let instance=super::instances::read(&app,&instance_id)?;
    let base=game_root(&app,Some(&instance_id),&instance.version)?;
    prepare_folders(&base)?;
    let saves=base.join("saves");
    ensure_real_folder(&saves)?;
    let mut worlds=Vec::new();
    for entry in fs::read_dir(saves).map_err(|e|e.to_string())?.take(300) {
        let entry=entry.map_err(|e|e.to_string())?;
        let meta=fs::symlink_metadata(entry.path()).map_err(|e|e.to_string())?;
        if !meta.is_dir() || meta.file_type().is_symlink(){continue;}
        let level=entry.path().join("level.dat");
        let Ok(level_meta)=fs::symlink_metadata(level) else {continue;};
        if !level_meta.is_file() || level_meta.file_type().is_symlink(){continue;}
        let name=entry.file_name().to_string_lossy().into_owned();
        if name.is_empty()||name.chars().any(char::is_control){continue;}
        let last_modified=level_meta.modified().ok().and_then(|m|m.duration_since(UNIX_EPOCH).ok())
            .map(|duration|duration.as_secs()).unwrap_or(0);
        worlds.push(WorldSummary{name,last_modified,bytes_on_disk:level_meta.len()});
    }
    worlds.sort_by(|a,b|b.last_modified.cmp(&a.last_modified).then_with(||a.name.cmp(&b.name)));
    Ok(worlds)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn prevents_arbitrary_explorer_paths(){
        assert!(valid_kind("mods"));
        assert!(valid_kind("saves"));
        assert!(!valid_kind("../AppData"));
        assert!(!valid_kind("C:\\Windows"));
        assert!(!valid_kind("saves/../../"));
    }
    #[test]
    fn creates_normal_minecraft_directory_tree(){
        let root=std::env::temp_dir().join(format!("fre-game-folders-{}",uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.parent().unwrap()).unwrap();
        prepare_folders(&root).unwrap();
        assert!(root.join("mods").is_dir());
        assert!(root.join("saves").is_dir());
        assert!(root.join("resourcepacks").is_dir());
        assert!(root.join("shaderpacks").is_dir());
        assert!(root.join("screenshots").is_dir());
        std::fs::remove_dir_all(root).unwrap();
    }
}