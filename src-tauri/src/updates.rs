//! Explicit, integrity-checked updater powered by the official GitHub Releases API.
//! No unattended execution: the user must choose "Install update" in the UI.
//! The checksum comes from the same official GitHub release (not a publisher signature).
use futures::StreamExt;
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{path::Path, sync::atomic::{AtomicBool, Ordering}};
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::AsyncWriteExt;

// Public release channel can differ from the private source repository.
fn release_repo() -> &'static str {
    option_env!("FRE_LAUNCH_RELEASE_REPO")
        .filter(|repo| !repo.trim().is_empty())
        .unwrap_or("Freeze0102/FRE-LAUNCH")
}
fn latest_url() -> String {
    format!("https://api.github.com/repos/{}/releases/latest", release_repo())
}
const MAX_INSTALLER_BYTES: u64 = 160 * 1024 * 1024;
static UPDATING: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

#[derive(Clone, Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    body: Option<String>,
    assets: Vec<ReleaseAsset>,
    #[serde(default)]
    draft: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LauncherUpdate {
    pub current_version: String,
    pub version: String,
    pub notes: String,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateProgress {
    downloaded_bytes: u64,
    total_bytes: u64,
    phase: &'static str,
}

fn http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("FRE-LAUNCH-Updater/0.5.4 (https://github.com/Freeze0102/FRE-LAUNCH)")
        .timeout(std::time::Duration::from_secs(90))
        .build().map_err(|e|e.to_string())
}

fn release_version(release: &GithubRelease) -> Result<Version, String> {
    Version::parse(release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name))
        .map_err(|_| "GitHub release tag is not a supported SemVer".to_string())
}

fn our_version() -> Result<Version, String> {
    Version::parse(env!("CARGO_PKG_VERSION")).map_err(|e|format!("Local version: {e}"))
}

fn expected_asset(release: &GithubRelease, filename: &str) -> Result<ReleaseAsset, String> {
    let asset=release.assets.iter().find(|a|a.name==filename).cloned()
        .ok_or_else(||format!("The release does not contain {filename}"))?;
    if asset.size==0 || asset.size>MAX_INSTALLER_BYTES {
        return Err(format!("Unexpected size for {filename}"));
    }
    // Exclude mirrors, external domains and path substitution. Only Github
    // release download URLs of this exact repository and tagged release qualify.
    let parsed=reqwest::Url::parse(&asset.browser_download_url).map_err(|e|e.to_string())?;
    let prefix=format!("/{}/releases/download/{}/",release_repo(),release.tag_name);
    if parsed.scheme()!="https" || parsed.host_str()!=Some("github.com")
        || parsed.path()!=format!("{prefix}{filename}")
        || parsed.query().is_some() || parsed.fragment().is_some() {
        return Err("Installer origin is not the official GitHub repository".into());
    }
    Ok(asset)
}

fn expected_checksum_asset(release: &GithubRelease, filename: &str) -> Result<ReleaseAsset, String> {
    let name=format!("{filename}.sha256");
    let asset=release.assets.iter().find(|a|a.name==name).cloned()
        .ok_or("Release is missing its SHA-256 checksum")?;
    if asset.size==0 || asset.size>4096 {
        return Err("Unexpected checksum file size".into());
    }
    let parsed=reqwest::Url::parse(&asset.browser_download_url).map_err(|e|e.to_string())?;
    if parsed.scheme()!="https" || parsed.host_str()!=Some("github.com")
        || parsed.path()!=format!("/{}/releases/download/{}/{name}",release_repo(),release.tag_name)
        || parsed.query().is_some() || parsed.fragment().is_some() {
        return Err("Checksum origin is not the official GitHub repository".into());
    }
    Ok(asset)
}

async fn latest_release(client: &reqwest::Client) -> Result<Option<GithubRelease>, String> {
    let response=client.get(latest_url()).send().await.map_err(|e|format!("GitHub update check: {e}"))?;
    if response.status()==reqwest::StatusCode::NOT_FOUND {
        return Err("Канал оновлень недоступний: немає публічного релізу або репозиторій приватний".into());
    }
    let response=response.error_for_status().map_err(|e|format!("GitHub releases: {e}"))?;
    let release:GithubRelease=response.json().await.map_err(|e|format!("Invalid release metadata: {e}"))?;
    if release.draft {return Ok(None);}
    Ok(Some(release))
}

#[tauri::command]
pub async fn check_launcher_update() -> Result<Option<LauncherUpdate>, String> {
    let http=http()?;
    let Some(release)=latest_release(&http).await? else {return Ok(None);};
    let remote=release_version(&release)?;
    if remote<=our_version()? {return Ok(None);}
    let installer=expected_asset(&release,"FRE-LAUNCH-CurrentUser-Setup.exe")?;
    expected_checksum_asset(&release,"FRE-LAUNCH-CurrentUser-Setup.exe")?;
    expected_asset(&release,"FRE-LAUNCH-Administrator-Setup.exe")?;
    expected_checksum_asset(&release,"FRE-LAUNCH-Administrator-Setup.exe")?;
    Ok(Some(LauncherUpdate {
        current_version:env!("CARGO_PKG_VERSION").to_owned(),
        version:remote.to_string(),
        notes:release.body.unwrap_or_default().chars().take(2000).collect(),
        size_bytes:installer.size
    }))
}

fn validate_checksum(input: &str) -> Result<String, String> {
    let hash=input.split_whitespace().next().unwrap_or("");
    if hash.len()!=64 || !hash.bytes().all(|b|b.is_ascii_hexdigit()) {
        return Err("Release SHA-256 checksum is not valid".into());
    }
    Ok(hash.to_ascii_lowercase())
}

fn asset_filename(mode: &str) -> Result<&'static str, String> {
    match mode {
        "currentUser"=>Ok("FRE-LAUNCH-CurrentUser-Setup.exe"),
        "allUsers"=>Ok("FRE-LAUNCH-Administrator-Setup.exe"),
        _=>Err("Unknown updater installation mode".into())
    }
}

async fn perform_update(app: &AppHandle, wanted_version: &str, mode: &str) -> Result<String, String> {
    let expected_version=Version::parse(wanted_version).map_err(|_|"Invalid update version")?;
    let http=http()?;
    let release=latest_release(&http).await?.ok_or("No published update release found")?;
    let latest=release_version(&release)?;
    if latest!=expected_version || latest<=our_version()? {
        return Err("Update version changed; check for updates again".into());
    }
    let filename=asset_filename(mode)?;
    let installer=expected_asset(&release,filename)?;
    let checksum_asset=expected_checksum_asset(&release,filename)?;
    let response=http.get(&checksum_asset.browser_download_url).send().await
        .map_err(|e|format!("Checksum download: {e}"))?
        .error_for_status().map_err(|e|format!("Checksum HTTP: {e}"))?;
    let checksum_data=response.bytes().await.map_err(|e|e.to_string())?;
    if checksum_data.len()>4096 {return Err("Oversized checksum".into());}
    let text=std::str::from_utf8(&checksum_data).map_err(|_|"Invalid checksum encoding")?;
    let expected_hash=validate_checksum(text)?;

    let folder=app.path().app_cache_dir().map_err(|e|e.to_string())?.join("launcher-updates");
    tokio::fs::create_dir_all(&folder).await.map_err(|e|format!("Update cache: {e}"))?;
    let target=folder.join(format!("{}-{filename}",release.tag_name));
    let temporary=folder.join(format!("{}-{filename}.download",release.tag_name));
    let mut response=http.get(&installer.browser_download_url).send().await
        .map_err(|e|format!("Installer download: {e}"))?
        .error_for_status().map_err(|e|format!("Installer HTTP: {e}"))?;
    if response.content_length().is_some_and(|len|len>MAX_INSTALLER_BYTES) {
        return Err("The update installer is too large".into());
    }
    let mut file=tokio::fs::File::create(&temporary).await.map_err(|e|format!("Update disk write: {e}"))?;
    let mut checksum=Sha256::new();
    let mut total=0u64;
    let mut last_report=0u64;
    while let Some(chunk)=response.chunk().await.map_err(|e|format!("Update download interrupted: {e}"))? {
        total=total.checked_add(chunk.len() as u64).ok_or("Update download overflow")?;
        if total>MAX_INSTALLER_BYTES {return Err("Update installer exceeds safety limit".into());}
        checksum.update(&chunk);
        file.write_all(&chunk).await.map_err(|e|format!("Update disk write failed: {e}"))?;
        if total.saturating_sub(last_report)>256*1024 {
            let _=app.emit("launcher-update-progress",UpdateProgress{
                downloaded_bytes:total,total_bytes:installer.size,phase:"download"
            });
            last_report=total;
        }
    }
    file.sync_all().await.map_err(|e|format!("Update sync failed: {e}"))?;
    drop(file);
    if total!=installer.size {
        let _=tokio::fs::remove_file(&temporary).await;
        return Err("Downloaded installer size does not match GitHub release".into());
    }
    if format!("{:x}",checksum.finalize())!=expected_hash {
        let _=tokio::fs::remove_file(&temporary).await;
        return Err("Update installer SHA-256 validation failed. Nothing was executed.".into());
    }
    if tokio::fs::try_exists(&target).await.unwrap_or(false) {
        tokio::fs::remove_file(&target).await.map_err(|e|e.to_string())?;
    }
    tokio::fs::rename(&temporary,&target).await.map_err(|e|format!("Update staging: {e}"))?;
    let _=app.emit("launcher-update-progress",UpdateProgress{
        downloaded_bytes:total,total_bytes:total,phase:"verified"
    });
    super::diagnostics::write(app,"LAUNCHER_UPDATE",
        &format!("Verified installer {} SHA-256; launching updater",target.display()));
    launch_installer(&target)?;
    let to_exit=app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(600)).await;
        super::startup::clear_boot_marker_for_expected_exit();
        to_exit.exit(0);
    });
    Ok("Installer verified and started; FRE LAUNCH will close for the update".into())
}

fn launch_installer(path: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new(path)
            .creation_flags(0x08000000)
            .spawn().map_err(|e|format!("Could not start installer: {e}"))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _=path;
        Err("In-app installer updates are currently available on Windows only".into())
    }
}

#[tauri::command]
pub async fn install_launcher_update(app: AppHandle, version: String, mode: String) -> Result<String, String> {
    if UPDATING.compare_exchange(false,true,Ordering::AcqRel,Ordering::Acquire).is_err() {
        return Err("Another update is already in progress".into());
    }
    let result=perform_update(&app,&version,&mode).await;
    // Keep future checks possible if the installer launch failed.
    UPDATING.store(false,Ordering::Release);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn rejects_forged_assets() {
        let release=GithubRelease {
            tag_name:"v0.5.4".into(),body:None,draft:false,
            assets:vec![ReleaseAsset {
                name:"FRE-LAUNCH-CurrentUser-Setup.exe".into(),
                browser_download_url:"https://example.com/installer.exe".into(),
                size:12345
            }]
        };
        assert!(expected_asset(&release,"FRE-LAUNCH-CurrentUser-Setup.exe").is_err());
    }
    #[test] fn exact_installer_mode_only() {
        assert!(asset_filename("currentUser").is_ok());
        assert!(asset_filename("allUsers").is_ok());
        assert!(asset_filename("../escape").is_err());
    }
    #[test] fn checksum_must_be_64_hex() {
        assert!(validate_checksum(&"a".repeat(64)).is_ok());
        assert!(validate_checksum("bad checksum").is_err());
        assert!(validate_checksum(&("b".repeat(63)+"Z")).is_err());
    }
    #[test] fn strict_semver_comparison() {
        assert!(Version::parse("0.5.4").unwrap()>Version::parse("0.5.3").unwrap());
        assert!(Version::parse("0.5.3").unwrap()==Version::parse("0.5.3").unwrap());
    }
}
