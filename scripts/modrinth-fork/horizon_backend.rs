//! FRE LAUNCH delegates Horizon operations to the user-selected official CLI.
use std::{path::{Path, PathBuf}, process::{Output, Stdio}, time::Duration};
use tokio::{io::{AsyncRead, AsyncReadExt}, process::Command};

const OUTPUT_LIMIT: usize = 24_000;
const QUICK_TIMEOUT: Duration = Duration::from_secs(60);
const INSTALL_TIMEOUT: Duration = Duration::from_secs(30 * 60);
static OPERATION: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("fre-horizon")
        .invoke_handler(tauri::generate_handler![
            horizon_instances, horizon_create, horizon_play, horizon_logs,
            horizon_versions, horizon_install_version, horizon_set_version, horizon_prepare,
        ])
        .build()
}

fn validate_launcher(path: &str) -> Result<PathBuf, String> {
    let path = Path::new(path);
    if !path.is_absolute() {
        return Err("Choose the full path to horizon-launcher.exe.".into());
    }
    let absolute = path.canonicalize().map_err(|e| format!("Horizon executable not found: {e}"))?;
    if !absolute.is_file() || !absolute.file_name().and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("horizon-launcher.exe")) {
        return Err("Choose horizon-launcher.exe from the official Horizon Windows release.".into());
    }
    Ok(absolute)
}

fn validate_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 64 || name.starts_with('-')
        || name.ends_with('.') || name.chars().any(|c| c.is_control() || "<>:\"/\\|?*".contains(c)) {
        return Err("Use a name of 1–64 characters without Windows filename symbols.".into());
    }
    Ok(name)
}

fn validate_version(version: &str) -> Result<&str, String> {
    let version = version.trim();
    if version.is_empty() || version.len() > 80
        || !version.starts_with(|c: char| c.is_ascii_alphanumeric())
        || !version.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')) {
        return Err("Enter a Horizon version tag or 'latest'.".into());
    }
    Ok(version)
}

fn command(path: &Path) -> Command {
    let mut command = Command::new(path);
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    command
}

async fn bounded_output(mut stream: impl AsyncRead + Unpin) -> std::io::Result<Vec<u8>> {
    let mut retained = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        let count = stream.read(&mut buffer).await?;
        if count == 0 { return Ok(retained); }
        retained.extend_from_slice(&buffer[..count]);
        if retained.len() > OUTPUT_LIMIT {
            retained.drain(..retained.len() - OUTPUT_LIMIT);
        }
    }
}

async fn collect(mut child: tokio::process::Child) -> std::io::Result<Output> {
    let stdout = child.stdout.take().expect("stdout is piped");
    let stderr = child.stderr.take().expect("stderr is piped");
    let (status, stdout, stderr) = tokio::try_join!(child.wait(), bounded_output(stdout), bounded_output(stderr))?;
    Ok(Output { status, stdout, stderr })
}

fn output_string(output: Output) -> Result<String, String> {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if output.status.success() {
        return Ok(if stdout.is_empty() { stderr } else { stdout });
    }
    Err(format!("Horizon launcher returned {}: {}", output.status,
        if stderr.is_empty() { stdout } else { stderr }.chars().take(1500).collect::<String>()))
}

async fn run_cli(exe: &Path, args: &[&str], timeout: Duration) -> Result<String, String> {
    let _operation = OPERATION.try_lock().map_err(|_| "Another Horizon operation is still running.")?;
    let child = command(exe).args(args).kill_on_drop(true).spawn()
        .map_err(|e| format!("Could not run the Horizon launcher: {e}"))?;
    let output = tokio::time::timeout(timeout, collect(child)).await
        .map_err(|_| "Horizon operation timed out. Check the official launcher and logs before retrying.")?
        .map_err(|e| format!("Horizon output error: {e}"))?;
    output_string(output)
}

#[tauri::command]
async fn horizon_instances(exe_path: String) -> Result<String, String> {
    run_cli(&validate_launcher(&exe_path)?, &["instance", "list"], QUICK_TIMEOUT).await
}

#[tauri::command]
async fn horizon_create(exe_path: String, name: String) -> Result<String, String> {
    let name = validate_name(&name)?;
    run_cli(&validate_launcher(&exe_path)?, &["instance", "create", name], QUICK_TIMEOUT).await?;
    Ok(format!("Horizon instance '{name}' created."))
}

#[tauri::command]
async fn horizon_play(exe_path: String, name: String) -> Result<String, String> {
    let name = validate_name(&name)?;
    let _operation = OPERATION.try_lock().map_err(|_| "Another Horizon operation is still running.")?;
    let mut child = command(&validate_launcher(&exe_path)?).args(["play", "--instance", name])
        .stdout(Stdio::null()).stderr(Stdio::null())
        .spawn().map_err(|e| format!("Could not start Horizon: {e}"))?;
    let pid = child.id().unwrap_or_default();
    // No pipes: closing FRE LAUNCH must not break the game's stdout/stderr.
    let mut task = tokio::spawn(async move { child.wait().await });
    match tokio::time::timeout(Duration::from_millis(1000), &mut task).await {
        Ok(result) => {
            let status = result.map_err(|e| e.to_string())?.map_err(|e| e.to_string())?;
            let details = output_string(Output { status, stdout: Vec::new(),
                stderr: b"Check the official Horizon launcher and logs for details.".to_vec() })?;
            Ok(format!("Horizon launcher finished.\n{details}"))
        }
        Err(_) => Ok(format!("Passed instance '{name}' to Horizon launcher (process {pid}). Check Horizon or its logs for game status.")),
    }
}

#[tauri::command]
async fn horizon_logs(exe_path: String) -> Result<String, String> {
    run_cli(&validate_launcher(&exe_path)?, &["logs", "--lines", "120"], QUICK_TIMEOUT).await
}

#[tauri::command]
async fn horizon_versions(exe_path: String, installed_only: bool) -> Result<String, String> {
    let args: &[&str] = if installed_only { &["versions", "--installed"] } else { &["versions"] };
    run_cli(&validate_launcher(&exe_path)?, args, QUICK_TIMEOUT).await
}

#[tauri::command]
async fn horizon_install_version(exe_path: String, version: String) -> Result<String, String> {
    let version = validate_version(&version)?;
    run_cli(&validate_launcher(&exe_path)?, &["install", version], INSTALL_TIMEOUT).await
}

#[tauri::command]
async fn horizon_set_version(exe_path: String, name: String, version: String) -> Result<String, String> {
    let name = validate_name(&name)?;
    let version = validate_version(&version)?;
    run_cli(&validate_launcher(&exe_path)?, &["instance", "version", name, version], QUICK_TIMEOUT).await?;
    Ok(format!("Set '{name}' to Horizon version '{version}'."))
}

#[tauri::command]
async fn horizon_prepare(exe_path: String, minecraft_path: Option<String>, version: Option<String>) -> Result<String, String> {
    let exe = validate_launcher(&exe_path)?;
    let mut args = vec!["prepare"];
    if let Some(version) = version.as_deref().filter(|v| !v.trim().is_empty() && v.trim() != "latest") {
        args.extend(["--version", validate_version(version)?]);
    }
    if let Some(path) = minecraft_path.as_deref().filter(|p| !p.trim().is_empty()) {
        if !Path::new(path).is_absolute() || !Path::new(path).is_dir() {
            return Err("Choose an existing Minecraft installation folder.".into());
        }
        args.extend(["--minecraft", path]);
    }
    run_cli(&exe, &args, INSTALL_TIMEOUT).await
}

#[cfg(test)]
mod tests {
    use super::*;
    static PROCESS_TEST: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    #[test]
    fn validates_instance_name_and_version() {
        assert_eq!(validate_name("  Мій Horizon  ").unwrap(), "Мій Horizon");
        assert!(validate_name(&"ї".repeat(64)).is_ok());
        for name in ["../world", "--config", "", "world."] { assert!(validate_name(name).is_err()); }
        for version in ["latest", "v2026.10.13", "0.2.31"] { assert!(validate_version(version).is_ok()); }
        for version in ["--config", "../../bad", ".", "..", ""] { assert!(validate_version(version).is_err()); }
    }
    #[tokio::test]
    async fn output_is_bounded_and_keeps_the_end() {
        let bytes = vec![b'x'; OUTPUT_LIMIT * 4];
        let result = bounded_output(bytes.as_slice()).await.unwrap();
        assert_eq!(result, bytes[bytes.len() - OUTPUT_LIMIT..]);
    }
    #[tokio::test]
    async fn slow_cli_is_terminated_at_the_deadline() {
        let _test = PROCESS_TEST.lock().await;
        let exe = std::env::current_exe().unwrap();
        let error = run_cli(&exe,
            &["--ignored", "--exact", "api::horizon::tests::sleep_fixture"],
            Duration::from_millis(100)).await.unwrap_err();
        assert!(error.contains("timed out"), "{error}");
    }
    #[test]
    #[ignore = "child process fixture for the timeout regression"]
    fn sleep_fixture() { std::thread::sleep(Duration::from_secs(10)); }
    #[cfg(windows)]
    #[tokio::test]
    async fn captures_real_process_output_and_failure() {
        let _test = PROCESS_TEST.lock().await;
        let exe = PathBuf::from(std::env::var_os("COMSPEC").unwrap());
        assert!(run_cli(&exe, &["/D", "/C", "echo horizon-test"], QUICK_TIMEOUT).await.unwrap().contains("horizon-test"));
        assert!(run_cli(&exe, &["/D", "/C", "exit 7"], QUICK_TIMEOUT).await.is_err());
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn commands_match_official_cli_contract_using_isolated_fixture() {
        let _test = PROCESS_TEST.lock().await;
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("fixture.rs");
        let exe = directory.path().join("horizon-launcher.exe");
        std::fs::write(&source, r#"
            fn main() {
                let args: Vec<String> = std::env::args().skip(1).collect();
                let args: Vec<&str> = args.iter().map(String::as_str).collect();
                match args.as_slice() {
                    ["instance", "list"] => println!("Survival with spaces"),
                    ["instance", "create", "Survival with spaces"] => println!("created"),
                    ["play", "--instance", "Survival with spaces"] => println!("finished"),
                    ["play", "--instance", "Missing"] => {
                        eprintln!("Missing installed version"); std::process::exit(7);
                    }
                    ["logs", "--lines", "120"] => println!("fixture logs"),
                    ["versions", "--installed"] => println!("installed"),
                    ["versions"] => println!("available"),
                    ["install", "latest"] => println!("installed latest"),
                    ["instance", "version", "Survival with spaces", "latest"] => println!("selected"),
                    ["prepare"] | ["prepare", "--minecraft", _]
                        | ["prepare", "--version", "v2026.10.13"] => println!("prepared"),
                    _ => { eprintln!("Invalid CLI arguments: {args:?}"); std::process::exit(2); }
                }
            }
        "#).unwrap();
        let output = std::process::Command::new("rustc").arg(&source).arg("-o").arg(&exe).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let path = exe.to_str().unwrap().to_string();
        assert!(horizon_instances(path.clone()).await.unwrap().contains("Survival with spaces"));
        horizon_create(path.clone(), "Survival with spaces".into()).await.unwrap();
        assert_eq!(horizon_versions(path.clone(), true).await.unwrap(), "installed");
        assert_eq!(horizon_versions(path.clone(), false).await.unwrap(), "available");
        horizon_install_version(path.clone(), "latest".into()).await.unwrap();
        horizon_set_version(path.clone(), "Survival with spaces".into(), "latest".into()).await.unwrap();
        horizon_prepare(path.clone(), None, None).await.unwrap();
        horizon_prepare(path.clone(), None, Some("v2026.10.13".into())).await.unwrap();
        horizon_prepare(path.clone(), Some(directory.path().to_str().unwrap().into()), Some(" latest ".into())).await.unwrap();
        assert_eq!(horizon_logs(path.clone()).await.unwrap(), "fixture logs");
        horizon_play(path.clone(), "Survival with spaces".into()).await.unwrap();
        assert!(horizon_play(path, "Missing".into()).await.unwrap_err().contains('7'));
    }
}
