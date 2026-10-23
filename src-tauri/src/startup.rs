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
        if crashed_last_start || std::env::var_os("FRE_LAUNCH_FORCE_SOFTWARE").is_some() {
            // WebView2 reads this before its environment is created. A second launch
            // after an early Windows/graphics crash uses software compositing.
            let existing=std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").unwrap_or_default();
            std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
                format!("{existing} --disable-gpu --disable-accelerated-2d-canvas --disable-gpu-compositing"));
            std::env::set_var("FRE_LAUNCH_SAFE_MODE","1");
        }
        if let Some(parent) = marker.parent() {
            if std::fs::create_dir_all(parent).is_ok() {
                let _=std::fs::write(marker,b"startup pending; cleared after 20 seconds of responsive UI");
            }
        }
    }
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
