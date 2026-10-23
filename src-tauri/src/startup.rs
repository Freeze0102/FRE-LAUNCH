//! Crash recovery bootstrap for Windows WebView2.
//! No elevated privileges. Never deletes user worlds, profiles or Minecraft data.
#[cfg(windows)]
fn marker_path() -> Option<std::path::PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    Some(std::path::PathBuf::from(local).join("FRE-LAUNCH").join("bootstrap.pending"))
}

pub fn prepare_windows_webview() {
    #[cfg(windows)]
    {
        use std::time::Duration;
        let Some(marker) = marker_path() else {return;};
        let crashed_last_start = std::fs::metadata(&marker)
            .and_then(|meta|meta.modified())
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age < Duration::from_secs(8 * 3600));
        let recovery=marker.with_file_name("recovery.enabled");
        if crashed_last_start {
            // Retain recovery until the user explicitly opts out: a single
            // successful soft-rendered launch must not restore the crash loop.
            let _=std::fs::write(&recovery,b"Software-rendered WebView2 recovery active");
        }
        if crashed_last_start || recovery.is_file()
            || std::env::var_os("FRE_LAUNCH_FORCE_SOFTWARE").is_some() {
            // WebView2 reads this before its environment is created. A second launch
            // after an early Windows/graphics crash uses software compositing.
            let existing=std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").unwrap_or_default();
            std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
                format!("{existing} --disable-gpu --disable-accelerated-2d-canvas --disable-gpu-compositing"));
            std::env::set_var("FRE_LAUNCH_SAFE_MODE","1");
            // The default WebView2 profile can be corrupt or located next to
            // an admin-installed EXE, where the user has no write access.
            // Recovery uses a separate writable local profile; worlds stay intact.
            if let Some(parent)=marker.parent() {
                let safe_profile=parent.join("WebView2-Recovery");
                if std::fs::create_dir_all(&safe_profile).is_ok() {
                    std::env::set_var("WEBVIEW2_USER_DATA_FOLDER",safe_profile);
                }
            }
        }
        if let Some(parent) = marker.parent() {
            if std::fs::create_dir_all(parent).is_ok() {
                let _=std::fs::write(marker,b"startup pending; cleared after 20 seconds of responsive UI");
            }
        }
    }
}

/// A normal, user-initiated exit must not be treated as an early crash.
pub fn clear_boot_marker_for_expected_exit() {
    #[cfg(windows)]
    if let Some(path)=marker_path() {let _=std::fs::remove_file(path);}
}

#[tauri::command]
pub fn disable_software_recovery() -> Result<(),String> {
    #[cfg(windows)]
    if let Some(path)=marker_path() {
        let flag=path.with_file_name("recovery.enabled");
        match std::fs::remove_file(flag) {
            Ok(())=>{},
            Err(e) if e.kind()==std::io::ErrorKind::NotFound=>{},
            Err(e)=>return Err(format!("Recovery settings: {e}")),
        }
    }
    Ok(())
}

/// Called only after the React interface has stayed alive long enough.
#[tauri::command]
pub fn mark_frontend_ready() -> Result<(),String> {
    #[cfg(windows)]
    if let Some(path) = marker_path() {
        match std::fs::remove_file(path) {
            Ok(())=>{},
            Err(e) if e.kind()==std::io::ErrorKind::NotFound=>{},
            Err(e)=>return Err(format!("Startup marker cleanup: {e}")),
        }
    }
    Ok(())
}

#[tauri::command]
pub fn software_rendering() -> bool {
    std::env::var_os("FRE_LAUNCH_SAFE_MODE").is_some()
}

#[cfg(test)]
mod tests {
    #[test] fn startup_marker_is_private_to_launcher() {
        #[cfg(windows)] {
            let path=super::marker_path().expect("LOCALAPPDATA");
            assert!(path.ends_with("FRE-LAUNCH\\bootstrap.pending"));
        }
    }
}
