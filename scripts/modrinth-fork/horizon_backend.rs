//! FRE LAUNCH Horizon integration (uses the user's installed official launcher).
//! Never executes code downloaded from the internet or modifies Java instances.
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("fre-horizon")
        .invoke_handler(tauri::generate_handler![
            horizon_instances,
            horizon_create,
            horizon_play,
            horizon_logs,
            horizon_versions,
            horizon_install_version,
            horizon_set_version,
            horizon_prepare,
        ])
        .build()
}

fn validate_launcher(path: &str) -> Result<PathBuf, String> {
    let p = Path::new(path);
    if !p.is_absolute() {
        return Err("Choose the full path to horizon-launcher.exe.".into());
    }
    let absolute = p
        .canonicalize()
        .map_err(|e| format!("Horizon executable not found: {e}"))?;
    if !absolute.is_file()
        || !absolute
            .file_name()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.eq_ignore_ascii_case("horizon-launcher.exe"))
    {
        return Err("Choose the official horizon-launcher.exe extracted from Horizon's Windows release.".into());
    }
    Ok(absolute)
}

fn validate_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.len() > 64
        || name.is_empty()
        || name.starts_with('-')
        || name.ends_with('.')
        || name.ends_with(' ')
        || name.chars().any(|c| c.is_control() || "<>:\"/\\|?*".contains(c))
    {
        return Err("Use a name of 1–64 characters without Windows filename symbols.".into());
    }
    Ok(name)
}

fn command(path: &Path) -> Command {
    let mut cmd = Command::new(path);
    #[cfg(windows)]
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW for the CLI wrapper
    cmd
}

fn output_string(output: std::process::Output) -> Result<String, String> {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if output.status.success() {
        return Ok(stdout);
    }
    let info = if stderr.is_empty() { stdout } else { stderr };
    Err(format!(
        "Horizon launcher returned {}: {}",
        output.status,
        info.chars().take(1500).collect::<String>()
    ))
}

#[tauri::command]
async fn horizon_instances(exe_path: String) -> Result<String, String> {
    let exe = validate_launcher(&exe_path)?;
    tokio::task::spawn_blocking(move || {
        let out = command(&exe)
            .args(["instance", "list"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("Could not run the Horizon launcher: {e}"))?;
        output_string(out)
    })
    .await
    .map_err(|e| format!("Horizon task error: {e}"))?
}

#[tauri::command]
async fn horizon_create(exe_path: String, name: String) -> Result<String, String> {
    let exe = validate_launcher(&exe_path)?;
    let name = validate_name(&name)?.to_string();
    tokio::task::spawn_blocking(move || {
        let out = command(&exe)
            .args(["instance", "create", &name])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("Could not create Horizon instance: {e}"))?;
        output_string(out)?;
        Ok(format!("Horizon instance '{name}' created."))
    })
    .await
    .map_err(|e| format!("Horizon task error: {e}"))?
}

#[tauri::command]
async fn horizon_play(exe_path: String, name: String) -> Result<String, String> {
    let exe = validate_launcher(&exe_path)?;
    let name = validate_name(&name)?.to_string();
    tokio::task::spawn_blocking(move || {
        let child = command(&exe)
            .args(["play", "--instance", &name])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Could not start Horizon: {e}"))?;
        Ok(format!(
            "Passed instance '{name}' to Horizon launcher (process {}). Check the Horizon window or logs if it does not start.",
            child.id()
        ))
    })
    .await
    .map_err(|e| format!("Horizon task error: {e}"))?
}

/// Read the local Horizon log for troubleshooting, without uploading it.
#[tauri::command]
async fn horizon_logs(exe_path: String) -> Result<String, String> {
    let exe = validate_launcher(&exe_path)?;
    tokio::task::spawn_blocking(move || {
        let out = command(&exe)
            .args(["logs", "--lines", "120"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("Could not read Horizon logs: {e}"))?;
        let log = output_string(out)?;
        Ok(log.chars().take(24_000).collect::<String>())
    })
    .await
    .map_err(|e| format!("Horizon task error: {e}"))?
}

fn validate_version(version: &str) -> Result<&str, String> {
    let version = version.trim();
    if version.is_empty()
        || version.len() > 80
        || version.starts_with('-')
        || !version
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
    {
        return Err("Enter a Horizon version tag or 'latest'.".into());
    }
    Ok(version)
}

#[tauri::command]
async fn horizon_versions(exe_path: String, installed_only: bool) -> Result<String, String> {
    let exe = validate_launcher(&exe_path)?;
    tokio::task::spawn_blocking(move || {
        let mut cmd = command(&exe);
        cmd.arg("versions");
        if installed_only {
            cmd.arg("--installed");
        }
        let output = cmd.output()
            .map_err(|e| format!("Could not list Horizon versions: {e}"))?;
        output_string(output)
    }).await.map_err(|e| format!("Horizon task error: {e}"))?
}

#[tauri::command]
async fn horizon_install_version(exe_path: String, version: String) -> Result<String, String> {
    let exe = validate_launcher(&exe_path)?;
    let version = validate_version(&version)?.to_string();
    tokio::task::spawn_blocking(move || {
        let output = command(&exe)
            .args(["install", &version])
            .output()
            .map_err(|e| format!("Could not install Horizon version: {e}"))?;
        let details = output_string(output)?;
        Ok(format!("Horizon version '{version}' installation finished.\n{details}"))
    }).await.map_err(|e| format!("Horizon task error: {e}"))?
}

#[tauri::command]
async fn horizon_set_version(
    exe_path: String,
    name: String,
    version: String,
) -> Result<String, String> {
    let exe = validate_launcher(&exe_path)?;
    let name = validate_name(&name)?.to_string();
    let version = validate_version(&version)?.to_string();
    tokio::task::spawn_blocking(move || {
        let output = command(&exe)
            .args(["instance", "version", &name, &version])
            .output()
            .map_err(|e| format!("Could not select Horizon version: {e}"))?;
        output_string(output)?;
        Ok(format!("Set '{name}' to Horizon version '{version}'."))
    }).await.map_err(|e| format!("Horizon task error: {e}"))?
}

#[tauri::command]
async fn horizon_prepare(exe_path: String) -> Result<String, String> {
    let exe = validate_launcher(&exe_path)?;
    tokio::task::spawn_blocking(move || {
        let output = command(&exe)
            .arg("prepare")
            .output()
            .map_err(|e| format!("Could not prepare Horizon: {e}"))?;
        let details = output_string(output)?;
        Ok(format!("Horizon preparation finished.\n{details}"))
    }).await.map_err(|e| format!("Horizon task error: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_instance_name() {
        assert!(validate_name("Horizon Survival").is_ok());
        assert!(validate_name("../world").is_err());
        assert!(validate_name("--config").is_err());
        assert!(validate_name("").is_err());
        assert_eq!(validate_version("latest").unwrap(), "latest");
        assert_eq!(validate_version("0.2.31").unwrap(), "0.2.31");
        assert!(validate_version("--config").is_err());
        assert!(validate_version("../../bad").is_err());
    }
}
