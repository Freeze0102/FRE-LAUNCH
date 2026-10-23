//! Preflight checks are run before expensive Minecraft downloads.
//! They report the *actual* game data directory, which can differ from the installer drive.
use serde::Serialize;
use std::{fs::{self, OpenOptions}, io::Write, path::PathBuf};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all="camelCase")]
pub struct InstallPreflight {
    pub path: String,
    pub ready: bool,
    pub detail: String,
}

fn target(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app.path().app_data_dir().map_err(|e| format!("Не вдалося знайти папку програми: {e}"))?
        .join("minecraft"))
}

fn test_write(path: &PathBuf) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("Не вдалося створити папку Minecraft: {e}"))?;
    let name = format!(".fre-write-test-{}", Uuid::new_v4());
    let probe = path.join(name);
    let result = (|| {
        let mut file=OpenOptions::new().write(true).create_new(true).open(&probe)
            .map_err(|e| format!("Немає доступу для запису на диск: {e}"))?;
        file.write_all(b"FRE LAUNCH disk check").map_err(|e| format!("Не вдалося записати тестовий файл: {e}"))?;
        file.sync_all().map_err(|e| format!("Помилка запису на диск: {e}"))?;
        Ok(())
    })();
    let _=fs::remove_file(&probe);
    result
}

pub fn ensure_write_access(app: &AppHandle) -> Result<PathBuf, String> {
    let path=target(app)?;
    test_write(&path)?;
    Ok(path)
}

#[tauri::command]
pub async fn install_preflight(app: AppHandle) -> Result<InstallPreflight, String> {
    tokio::task::spawn_blocking(move || {
        let path=target(&app)?;
        let display=path.to_string_lossy().into_owned();
        match test_write(&path) {
            Ok(())=>Ok(InstallPreflight{
                path:display,
                ready:true,
                detail:"Каталог Minecraft доступний для запису".into()
            }),
            Err(e)=>Ok(InstallPreflight{path:display,ready:false,detail:e})
        }
    }).await.map_err(|e| format!("Перевірка диска перервалася: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn write_probe_works_for_a_regular_directory() {
        let path=std::env::temp_dir().join(format!("fre-install-preflight-{}",Uuid::new_v4()));
        test_write(&path).expect("test directory is writable");
        let entries=fs::read_dir(&path).expect("directory exists").count();
        assert_eq!(entries,0,"test must not leave probe files");
        fs::remove_dir(&path).expect("cleanup");
    }
}
