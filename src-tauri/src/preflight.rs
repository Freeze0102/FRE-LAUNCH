//! Preflight checks are run before expensive Minecraft downloads.
//! They report the *actual* game data directory, which can differ from the installer drive.
use serde::Serialize;
use std::{fs::{self, OpenOptions}, io::Write, path::PathBuf};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

// Prevent starting a large Minecraft download on an almost-full system disk.
// This checks the game-data volume, which is often C: even when the launcher is on D:.
const MIN_GAME_DISK_SPACE: u64 = 900 * 1024 * 1024;
fn validate_free_space(free: u64) -> Result<(), String> {
    if free < MIN_GAME_DISK_SPACE {
        return Err(format!("Недостатньо місця на диску Minecraft: доступно {} МБ, рекомендовано щонайменше 900 МБ. Звільни місце на диску з даними гри.", free / (1024*1024)));
    }
    Ok(())
}

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
    let free=fs2::available_space(path).map_err(|e| format!("Не вдалося перевірити вільне місце: {e}"))?;
    validate_free_space(free)?;
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
    super::diagnostics::write(&app, "PREFLIGHT_BEGIN", "Checking Minecraft game-data volume");
    tokio::task::spawn_blocking(move || {
        let path=target(&app)?;
        let display=path.to_string_lossy().into_owned();
        match test_write(&path) {
            Ok(())=>{
                super::diagnostics::write(&app,"PREFLIGHT_OK",&display);
                Ok(InstallPreflight{
                path:display,
                ready:true,
                detail:"Каталог Minecraft доступний для запису, місця достатньо".into()
            })
            },
            Err(e)=>{
                super::diagnostics::write(&app,"PREFLIGHT_ERROR",&format!("{display}: {e}"));
                Ok(InstallPreflight{path:display,ready:false,detail:e})
            }
        }
    }).await.map_err(|e| format!("Перевірка диска перервалася: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_disk_with_insufficient_free_space() {
        assert!(validate_free_space(100 * 1024 * 1024).is_err());
        assert!(validate_free_space(MIN_GAME_DISK_SPACE).is_ok());
    }
    #[test]
    fn write_probe_works_for_a_regular_directory() {
        let path=std::env::temp_dir().join(format!("fre-install-preflight-{}",Uuid::new_v4()));
        test_write(&path).expect("test directory is writable");
        let entries=fs::read_dir(&path).expect("directory exists").count();
        assert_eq!(entries,0,"test must not leave probe files");
        fs::remove_dir(&path).expect("cleanup");
    }
}
