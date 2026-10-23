use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub version: String,
    pub favorite: bool,
    pub memory_mb: u32,
    pub java_path: Option<String>,
    pub loader: String,
    pub loader_version: Option<String>,
}

fn db_file(app: &AppHandle) -> Result<PathBuf, String> {
    let folder = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    Ok(folder.join("fre-launch.sqlite3"))
}

pub(super) fn db(app: &AppHandle) -> Result<Connection, String> {
    let connection = Connection::open(db_file(app)?).map_err(|e| e.to_string())?;
    connection.execute_batch(
        "PRAGMA busy_timeout=5000;
         CREATE TABLE IF NOT EXISTS instances (
             id TEXT PRIMARY KEY NOT NULL,
             name TEXT NOT NULL,
             version TEXT NOT NULL,
             favorite INTEGER NOT NULL DEFAULT 0,
             memory_mb INTEGER NOT NULL DEFAULT 4096,
             java_path TEXT
         );"
    ).map_err(|e| e.to_string())?;
    // Upgrade databases created by older FRE LAUNCH builds without losing worlds.
    let schema_version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|e| e.to_string())?;
    if schema_version < 1 {
        connection.execute_batch(
            "BEGIN IMMEDIATE;
             ALTER TABLE instances ADD COLUMN loader TEXT NOT NULL DEFAULT 'vanilla';
             ALTER TABLE instances ADD COLUMN loader_version TEXT;
             PRAGMA user_version=1;
             COMMIT;"
        ).map_err(|e| format!("Instance schema migration failed: {e}"))?;
    }
    Ok(connection)
}

fn validate_id(id: &str) -> Result<(), String> {
    Uuid::parse_str(id).map(|_| ()).map_err(|_| "Invalid instance ID".to_string())
}

fn row_instance(row: &rusqlite::Row<'_>) -> rusqlite::Result<Instance> {
    Ok(Instance {
        id: row.get(0)?,
        name: row.get(1)?,
        version: row.get(2)?,
        favorite: row.get::<_, i64>(3)? != 0,
        memory_mb: row.get::<_, u32>(4)?,
        java_path: row.get(5)?,
        loader: row.get(6)?,
        loader_version: row.get(7)?,
    })
}

pub fn read(app: &AppHandle, id: &str) -> Result<Instance, String> {
    validate_id(id)?;
    db(app)?.query_row(
        "SELECT id,name,version,favorite,memory_mb,java_path,loader,loader_version FROM instances WHERE id=?1",
        params![id], row_instance,
    ).map_err(|_| "Instance does not exist".to_string())
}

pub fn game_directory(app: &AppHandle, id: &str, version: &str) -> Result<PathBuf, String> {
    let instance = read(app, id)?;
    if instance.version != version {
        return Err("Selected instance belongs to a different Minecraft version".into());
    }
    let root = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(root.join("minecraft").join("instances").join(id))
}

fn game_directory_path(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    validate_id(id)?;
    Ok(app.path().app_data_dir().map_err(|e| e.to_string())?
        .join("minecraft").join("instances").join(id))
}

#[tauri::command]
pub fn list_instances(app: AppHandle) -> Result<Vec<Instance>, String> {
    let connection = db(&app)?;
    let mut statement = connection.prepare(
        "SELECT id,name,version,favorite,memory_mb,java_path,loader,loader_version FROM instances ORDER BY favorite DESC,name COLLATE NOCASE ASC"
    ).map_err(|e| e.to_string())?;
    let results = statement.query_map([], row_instance).map_err(|e| e.to_string())?;
    results.map(|row| row.map_err(|e| e.to_string())).collect()
}

#[tauri::command]
pub fn create_instance(app: AppHandle, name: String, version: String) -> Result<Instance, String> {
    super::minecraft::validate_version(&version)?;
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 48 {
        return Err("Instance name must be 1–48 characters".into());
    }
    if name.chars().any(char::is_control) { return Err("Invalid control character in instance name".into()); }
    let instance = Instance {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        version,
        favorite: false,
        memory_mb: 4096,
        java_path: None,
        loader: "vanilla".into(),
        loader_version: None,
    };
    // Materialize the playable instance directory as soon as it is created.
    // This makes mods, saves and resourcepacks available before the first launch.
    let folder = game_directory_path(&app, &instance.id)?;
    super::game_folders::prepare_folders(&folder)?;
    let connection = db(&app)?;
    connection.execute(
        "INSERT INTO instances (id,name,version,favorite,memory_mb,java_path) VALUES (?1,?2,?3,0,?4,NULL)",
        params![instance.id, instance.name, instance.version, instance.memory_mb]
    ).map_err(|e| e.to_string())?;
    Ok(instance)
}

#[tauri::command]
pub fn update_instance(app: AppHandle, id: String, name: String, favorite: bool, memory_mb: u32, java_path: Option<String>) -> Result<Instance, String> {
    validate_id(&id)?;
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 48 || name.chars().any(char::is_control) {
        return Err("Invalid instance name".into());
    }
    if !(512..=32768).contains(&memory_mb) { return Err("RAM must be 512–32768 MB".into()); }
    let path = java_path.and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
    });
    if let Some(path) = &path {
        let p = std::path::Path::new(path);
        if !p.is_file() { return Err("The selected Java executable does not exist".into()); }
    }
    let connection = db(&app)?;
    let changed = connection.execute(
        "UPDATE instances SET name=?2, favorite=?3, memory_mb=?4, java_path=?5 WHERE id=?1",
        params![id,name,if favorite {1} else {0},memory_mb,path],
    ).map_err(|e| e.to_string())?;
    if changed == 0 { return Err("Instance does not exist".into()); }
    read(&app, &id)
}

/// Selects an actual supported loader; the Fabric metadata profile is validated at launch.
#[tauri::command]
pub fn set_instance_loader(app: AppHandle, id: String, loader: String, loader_version: Option<String>) -> Result<Instance, String> {
    validate_id(&id)?;
    let version = match loader.as_str() {
        "vanilla" => None,
        "fabric" | "quilt" => {
            let selected = loader_version.as_deref().ok_or("Choose a loader version")?;
            super::minecraft::validate_version(selected)?;
            Some(selected.to_owned())
        }
        _ => return Err("This modloader is not yet implemented".into()),
    };
    let changed = db(&app)?.execute(
        "UPDATE instances SET loader=?2, loader_version=?3 WHERE id=?1",
        params![id,loader,version],
    ).map_err(|e| e.to_string())?;
    if changed == 0 {return Err("Instance not found".into());}
    read(&app, &id)
}

#[tauri::command]
pub fn delete_instance(app: AppHandle, id: String, delete_files: bool) -> Result<(), String> {
    validate_id(&id)?;
    read(&app, &id)?;
    let path = app.path().app_data_dir().map_err(|e| e.to_string())?
        .join("minecraft").join("instances").join(&id);
    // Preserve saves by default; explicitly deleting files requires a second UI confirmation.
    if delete_files && path.exists() {
        fs::remove_dir_all(&path).map_err(|e| format!("Cannot delete instance files: {e}"))?;
    }
    db(&app)?.execute("DELETE FROM instances WHERE id=?1",params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn instance_ids_must_be_uuids() {
        assert!(validate_id("../../../system").is_err());
        assert!(validate_id("1234").is_err());
        assert!(validate_id(&Uuid::new_v4().to_string()).is_ok());
    }
}
