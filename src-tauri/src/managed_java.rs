//! Optional, verified Temurin JRE download for Windows x64.
//! No elevated privileges, registry modifications or shell installers required.
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs as stdfs, io, path::{Path, PathBuf}};
use tauri::{AppHandle, Emitter, Manager};
use tokio::{fs, io::AsyncWriteExt};
use uuid::Uuid;

const MAX_ARCHIVE_SIZE: u64 = 420 * 1024 * 1024;
const MAX_UNPACKED: u64 = 1400 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
struct RuntimeProgress { phase:String, done:u64, total:u64, message:String }

fn report(app: &AppHandle, phase:&str, done:u64, total:u64, message:&str) {
    let _=app.emit("java-install-progress",RuntimeProgress{
        phase:phase.into(),done,total,message:message.into()
    });
}
fn runtime_folder(app: &AppHandle, major:u32) -> Result<PathBuf,String>{
    Ok(app.path().app_data_dir().map_err(|e|e.to_string())?
        .join("java").join(format!("temurin-{major}")))
}
fn java_path(folder:&Path) -> PathBuf {
    folder.join("bin").join(if cfg!(windows){"java.exe"}else{"java"})
}
fn supported(major:u32)->bool{matches!(major,8|11|17|21|25)}
fn validate_package_url(raw:&str,major:u32)->Result<(),String>{
    let url=reqwest::Url::parse(raw).map_err(|e|e.to_string())?;
    let expected=format!("/adoptium/temurin{major}-binaries/releases/download/");
    if url.scheme()!="https" || url.host_str()!=Some("github.com") || !url.path().starts_with(&expected) {
        return Err("Temurin download URL failed trust validation".into());
    }
    if !url.path().to_ascii_lowercase().ends_with(".zip"){
        return Err("Temurin runtime must be distributed as a ZIP archive".into());
    }
    Ok(())
}
fn http()->Result<reqwest::Client,String>{
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .connect_timeout(std::time::Duration::from_secs(15))
        .user_agent(concat!("FRE-LAUNCH/", env!("CARGO_PKG_VERSION")))
        .redirect(reqwest::redirect::Policy::custom(|attempt|{
            if attempt.previous().len()>5 {return attempt.error("Too many Java download redirects");}
            let host=attempt.url().host_str().unwrap_or("");
            if attempt.url().scheme()=="https"
                && matches!(host,"api.adoptium.net"|"github.com"|"release-assets.githubusercontent.com"|"objects.githubusercontent.com") {
                attempt.follow()
            }else{attempt.error("Untrusted Java download redirect")}
        }))
        .build().map_err(|e|e.to_string())
}
fn safe_extract(archive:&Path,output:&Path)->Result<(),String>{
    let file=stdfs::File::open(archive).map_err(|e|e.to_string())?;
    let mut zip=zip::ZipArchive::new(file).map_err(|e|e.to_string())?;
    if zip.len()>30_000{return Err("Java archive has too many files".into());}
    stdfs::create_dir_all(output).map_err(|e|e.to_string())?;
    let mut unpacked=0u64;
    for i in 0..zip.len(){
        let mut entry=zip.by_index(i).map_err(|e|e.to_string())?;
        if entry.unix_mode().is_some_and(|mode|mode&0o170000==0o120000){
            return Err("Java archive contains an unsupported symlink".into());
        }
        let relative=entry.enclosed_name().ok_or("Unsafe path in Java ZIP")?;
        let components:Vec<_>=relative.components().collect();
        if components.len()<2 {continue;} // Skip the archive's root JRE folder
        if components.iter().any(|c| !matches!(c,std::path::Component::Normal(_))) {
            return Err("Unsafe Java archive path".into());
        }
        let mut destination=output.to_path_buf();
        for component in components.iter().skip(1){destination.push(component.as_os_str());}
        if entry.is_dir(){stdfs::create_dir_all(&destination).map_err(|e|e.to_string())?;continue;}
        unpacked=unpacked.checked_add(entry.size()).ok_or("Java archive size overflow")?;
        if unpacked>MAX_UNPACKED {return Err("Unpacked Java archive exceeds safety limit".into());}
        if let Some(parent)=destination.parent(){stdfs::create_dir_all(parent).map_err(|e|e.to_string())?;}
        let mut result=stdfs::File::create(destination).map_err(|e|e.to_string())?;
        io::copy(&mut entry,&mut result).map_err(|e|e.to_string())?;
    }
    if !java_path(output).is_file(){return Err("Downloaded archive has no bin/java.exe".into());}
    Ok(())
}

#[tauri::command]
pub async fn install_managed_java(app: AppHandle, major: u32) -> Result<String, String> {
    if !supported(major){return Err("Supported managed Java versions: 8, 11, 17, 21, 25".into());}
    if !cfg!(windows) || !cfg!(target_arch="x86_64") {
        return Err("Automatic Temurin installation currently requires Windows x64".into());
    }
    let target=runtime_folder(&app,major)?;
    if java_path(&target).is_file(){
        return Ok(java_path(&target).to_string_lossy().to_string());
    }
    if target.exists(){return Err("A previous Java runtime folder is incomplete; please remove it before reinstalling".into());}
    let api=format!("https://api.adoptium.net/v3/assets/latest/{major}/hotspot");
    report(&app,"metadata",0,1,&format!("Пошук Eclipse Temurin {major}"));
    let client=http()?;
    let assets:Vec<Value>=client.get(&api)
        .query(&[("architecture","x64"),("image_type","jre"),("os","windows"),("vendor","eclipse")])
        .send().await.map_err(|e|e.to_string())?
        .error_for_status().map_err(|e|e.to_string())?
        .json().await.map_err(|e|e.to_string())?;
    let package=assets.first().ok_or("No supported Temurin JRE release found")?
        .get("binary").and_then(|b|b.get("package")).ok_or("Temurin package metadata not found")?;
    let url=package["link"].as_str().ok_or("Temurin binary link missing")?;
    validate_package_url(url,major)?;
    let expected=package["checksum"].as_str().ok_or("Temurin SHA-256 checksum missing")?;
    if expected.len()!=64||!expected.bytes().all(|b|b.is_ascii_hexdigit()){
        return Err("Invalid Temurin SHA-256 checksum".into());
    }
    let expected_size=package["size"].as_u64().ok_or("Temurin file size missing")?;
    if expected_size==0||expected_size>MAX_ARCHIVE_SIZE {
        return Err("Temurin runtime package is too large".into());
    }
    let response=client.get(url).send().await.map_err(|e|e.to_string())?
        .error_for_status().map_err(|e|e.to_string())?;
    if response.content_length().is_some_and(|size|size>MAX_ARCHIVE_SIZE){
        return Err("Temurin download exceeds safety limit".into());
    }
    let root=target.parent().ok_or("Invalid Java runtime destination")?;
    fs::create_dir_all(root).await.map_err(|e|e.to_string())?;
    let staging=root.join(format!(".temurin-{major}-{}",Uuid::new_v4()));
    fs::create_dir_all(&staging).await.map_err(|e|e.to_string())?;
    let archive=staging.join("runtime.zip");
    let result=async {
        let mut file=fs::File::create(&archive).await.map_err(|e|e.to_string())?;
        let mut chunks=response.bytes_stream();
        let mut hasher=Sha256::new();
        let mut received=0u64;
        while let Some(chunk)=chunks.next().await{
            let chunk=chunk.map_err(|e|e.to_string())?;
            received=received.checked_add(chunk.len() as u64).ok_or("Archive size overflow")?;
            if received>MAX_ARCHIVE_SIZE{return Err("Java download too large".into());}
            file.write_all(&chunk).await.map_err(|e|e.to_string())?;
            hasher.update(&chunk);
            report(&app,"download",received,expected_size,"Завантаження Eclipse Temurin");
        }
        file.flush().await.map_err(|e|e.to_string())?;
        drop(file);
        if received!=expected_size {return Err("Temurin downloaded size does not match release metadata".into());}
        if !format!("{:x}",hasher.finalize()).eq_ignore_ascii_case(expected){
            return Err("Temurin download failed SHA-256 integrity verification".into());
        }
        report(&app,"extract",0,1,"Розпакування Java");
        let extracted=staging.join("unpacked");
        let archive_for_extract=archive.clone();
        let extracted_for_task=extracted.clone();
        tokio::task::spawn_blocking(move ||safe_extract(&archive_for_extract,&extracted_for_task))
            .await.map_err(|e|e.to_string())??;
        let executable=java_path(&extracted);
        let mut check=std::process::Command::new(&executable);
        check.arg("-version");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            check.creation_flags(0x08000000);
        }
        let test=check.output().map_err(|e|format!("Cannot validate downloaded Java: {e}"))?;
        if !test.status.success(){return Err("Downloaded Java could not start".into());}
        let info=String::from_utf8_lossy(&test.stderr);
        let detected=info.lines().next().unwrap_or("");
        let major_text=if major==8{"1.8".to_string()}else{major.to_string()};
        if !detected.contains(&major_text){
            return Err(format!("Downloaded Java reports an unexpected version: {detected}"));
        }
        fs::rename(extracted,&target).await.map_err(|e|format!("Cannot activate Temurin runtime: {e}"))?;
        report(&app,"complete",1,1,"Eclipse Temurin встановлено");
        Ok(java_path(&target).to_string_lossy().to_string())
    }.await;
    let _=fs::remove_dir_all(&staging).await;
    result
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test] fn trusted_temurin_link(){
        assert!(validate_package_url("https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.12%2B7/OpenJDK21U-jre_x64_windows_hotspot_21.0.12_7.zip",21).is_ok());
        assert!(validate_package_url("https://github.com/fake/temurin21-binaries/releases/download/jdk.zip",21).is_err());
        assert!(validate_package_url("http://github.com/adoptium/temurin21-binaries/releases/download/jdk.zip",21).is_err());
    }
}
