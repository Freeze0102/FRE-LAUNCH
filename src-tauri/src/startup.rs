//! Crash recovery bootstrap for Windows WebView2.
//! No elevated privileges. Never deletes user worlds, profiles or Minecraft data.
#[cfg(windows)]
fn marker_path() -> Option<std::path::PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    Some(std::path::PathBuf::from(local).join("FRE-LAUNCH").join("bootstrap.pending"))
}


const BOOTSTRAP_MAX_BYTES: u64 = 256 * 1024;

#[cfg(windows)]
fn bootstrap_log_path() -> Option<std::path::PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    Some(std::path::PathBuf::from(local).join("FRE-LAUNCH").join("startup.log"))
}

/// Best-effort trace before the Tauri window, AppHandle, or WebView2 exists.
pub fn log_bootstrap(stage: &str, message: &str) {
    #[cfg(windows)]
    {
        use std::{fs::{self, OpenOptions}, io::Write, time::{SystemTime, UNIX_EPOCH}};
        let Some(path) = bootstrap_log_path() else {return;};
        let Some(parent) = path.parent() else {return;};
        if fs::create_dir_all(parent).is_err() {return;}
        if fs::metadata(&path).is_ok_and(|meta|meta.len()>BOOTSTRAP_MAX_BYTES) {
            let _=fs::remove_file(path.with_extension("previous.log"));
            let _=fs::rename(&path,path.with_extension("previous.log"));
        }
        let timestamp=SystemTime::now().duration_since(UNIX_EPOCH)
            .map(|d|d.as_secs()).unwrap_or_default();
        let compact=message.replace('\n'," ").replace('\r'," ");
        if let Ok(mut f)=OpenOptions::new().append(true).create(true).open(path) {
            let _=writeln!(f,"[{timestamp}] [{stage}] {compact}");
        }
    }
    #[cfg(not(windows))]
    {let _=(stage,message);}
}

pub fn recent_bootstrap_log() -> Option<String> {
    #[cfg(windows)]
    {
        let contents=std::fs::read(bootstrap_log_path()?).ok()?;
        let start=contents.len().saturating_sub(32 * 1024);
        return Some(String::from_utf8_lossy(&contents[start..]).into_owned());
    }
    #[cfg(not(windows))]
    {None}
}

pub fn prepare_windows_webview() {
    log_bootstrap("PROCESS_START", "Executable entered Rust bootstrap");
    let previous_hook=std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        log_bootstrap("EARLY_PANIC", &panic.to_string());
        previous_hook(panic);
    }));
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
            log_bootstrap("WEBVIEW_SAFE_MODE", "GPU disabled; recovery profile selected");
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
                log_bootstrap("WEBVIEW_INITIALIZING", "Preparing WebView2 and Tauri window");
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
    log_bootstrap("FRONTEND_READY", "React interface responsive after 20 seconds");
    Ok(())
}

#[tauri::command]
pub fn mark_frontend_mounted() {
    log_bootstrap("FRONTEND_MOUNTED", "React home screen initialized");
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
