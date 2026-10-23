//! Compatible Fabric mod installation from Modrinth, including required dependencies.
//! Third-party code is never executed by FRE LAUNCH; the user chooses to launch
//! Minecraft with selected JARs, so mod downloads must be explicitly initiated.
use reqwest::Client;
use futures::StreamExt;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha512};
use std::{collections::{HashSet, VecDeque}, path::Path};
use tauri::AppHandle;
use tokio::fs;

const MAX_MOD_BYTES: u64 = 256 * 1024 * 1024;
const MAX_DEPENDENCIES: usize = 32;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledMod {
    pub filename: String,
    pub enabled: bool,
    pub bytes: u64,
}

fn client() -> Result<Client, String> {
    reqwest::Client::builder()
        .user_agent(concat!("FRE-LAUNCH/", env!("CARGO_PKG_VERSION"), " (+https://github.com/Freeze0102/FRE-LAUNCH)"))
        .connect_timeout(std::time::Duration::from_secs(12))
        .timeout(std::time::Duration::from_secs(120))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() > 3 {
                return attempt.error("Too many Modrinth redirects");
            }
            let host = attempt.url().host_str().unwrap_or("").to_ascii_lowercase();
            if attempt.url().scheme() == "https" &&
               matches!(host.as_str(), "api.modrinth.com" | "cdn.modrinth.com") {
                attempt.follow()
            } else {
                attempt.error("Unexpected Modrinth redirect destination")
            }
        }))
        .build().map_err(|e|e.to_string())
}

fn valid_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len()>100 || !id.bytes().all(|c|c.is_ascii_alphanumeric() || matches!(c,b'-'|b'_')) {
        return Err("Invalid Modrinth project or version ID".into());
    }
    Ok(())
}
fn valid_filename(name: &str, suffix: &str) -> bool {
    name.len()<=180 && name.len()>suffix.len() && name.to_ascii_lowercase().ends_with(suffix)
    // Local mods may be renamed with Ukrainian or other Unicode letters.
    // Still reject separators, control characters, and traversal components.
    && name.chars().all(|c|c.is_alphanumeric() || matches!(c,'.'|'-'|'_'|'+'|'('|')'|' '))
    && !name.starts_with('.') && !name.contains("..") && !name.ends_with('.') && !name.ends_with(' ')
}
async fn fetch_json(client: &Client, url: &str) -> Result<Value, String> {
    client.get(url).send().await.map_err(|e|e.to_string())?
        .error_for_status().map_err(|e|e.to_string())?
        .json::<Value>().await.map_err(|e|e.to_string())
}
fn is_compatible(version: &Value, game: &str) -> bool {
    version["game_versions"].as_array().is_some_and(|array|array.iter().any(|v|v.as_str()==Some(game))) &&
    version["loaders"].as_array().is_some_and(|array|array.iter().any(|v|v.as_str()==Some("fabric")))
}

async fn select_version(client: &Client, project_id: &str, exact_version: Option<&str>, game_version: &str) -> Result<Value, String> {
    if let Some(pinned) = exact_version {
        valid_id(pinned)?;
        let version = fetch_json(client,&format!("https://api.modrinth.com/v2/version/{pinned}")).await?;
        if version["project_id"].as_str()!=Some(project_id) || !is_compatible(&version,game_version) {
            return Err("Required mod version is incompatible with selected Minecraft/Fabric".into());
        }
        return Ok(version);
    }
    let game_versions=serde_json::to_string(&vec![game_version]).map_err(|e|e.to_string())?;
    let loaders=serde_json::to_string(&vec!["fabric"]).map_err(|e|e.to_string())?;
    let versions: Vec<Value> = client.get(format!("https://api.modrinth.com/v2/project/{project_id}/version"))
        .query(&[("game_versions",game_versions.as_str()),("loaders",loaders.as_str())])
        .send().await.map_err(|e|e.to_string())?
        .error_for_status().map_err(|e|e.to_string())?
        .json().await.map_err(|e|e.to_string())?;
    versions.into_iter().find(|v|is_compatible(v,game_version))
        .ok_or_else(||format!("No compatible Fabric mod found for {project_id} on Minecraft {game_version}"))
}
async fn resolve_project_type(client: &Client, project_id: &str) -> Result<(), String> {
    valid_id(project_id)?;
    let project=fetch_json(client,&format!("https://api.modrinth.com/v2/project/{project_id}")).await?;
    if project["project_type"].as_str()!=Some("mod") {
        return Err(format!("Project {project_id} is not a Minecraft mod"));
    }
    Ok(())
}

fn mods_dir(app: &AppHandle, instance_id: &str) -> Result<std::path::PathBuf, String> {
    let instance=super::instances::read(app,instance_id)?;
    super::instances::game_directory(app,instance_id,&instance.version).map(|p|p.join("mods"))
}
async fn verified_jar(client: &Client, version: &Value, dir: &Path) -> Result<String, String> {
    let files=version["files"].as_array().ok_or("Modrinth version is missing files")?;
    let file=files.iter().find(|f| f["primary"].as_bool()==Some(true)
        && f["filename"].as_str().is_some_and(|s|s.to_ascii_lowercase().ends_with(".jar")))
        .or_else(||files.iter().find(|f|f["filename"].as_str().is_some_and(|s|s.to_ascii_lowercase().ends_with(".jar"))))
        .ok_or("No JAR file available")?;
    let filename=file["filename"].as_str().ok_or("Missing filename")?;
    if !valid_filename(filename,".jar"){return Err("Unsafe mod JAR filename".into());}
    let url=file["url"].as_str().ok_or("Modrinth file URL missing")?;
    let parsed=reqwest::Url::parse(url).map_err(|e|e.to_string())?;
    if parsed.scheme()!="https" || parsed.host_str()!=Some("cdn.modrinth.com"){
        return Err("Mod must be hosted on the verified Modrinth CDN".into());
    }
    let digest=file["hashes"]["sha512"].as_str().ok_or("Missing SHA-512 digest")?;
    if digest.len()!=128 || !digest.bytes().all(|c|c.is_ascii_hexdigit()){return Err("Invalid SHA-512 digest".into());}
    let expected_size=file["size"].as_u64().ok_or("Missing file size")?;
    if expected_size==0 || expected_size>MAX_MOD_BYTES {return Err("Mod file exceeds size limit".into());}
    let path=dir.join(filename);
    if fs::try_exists(&path).await.map_err(|e|e.to_string())? {
        let current=fs::read(&path).await.map_err(|e|e.to_string())?;
        if format!("{:x}",Sha512::digest(&current)).eq_ignore_ascii_case(digest) {return Ok(filename.into());}
        return Err(format!("An existing file named {filename} differs from the expected mod; remove or rename it first"));
    }
    let response=client.get(url).send().await.map_err(|e|e.to_string())?
        .error_for_status().map_err(|e|e.to_string())?;
    if response.content_length().is_some_and(|n|n>MAX_MOD_BYTES) {
        return Err("Mod download exceeds size limit".into());
    }
    // Stream under the declared size limit; never buffer unbounded HTTP replies.
    let mut bytes=Vec::new();
    let mut received=0u64;
    let mut chunks=response.bytes_stream();
    while let Some(chunk)=chunks.next().await {
        let chunk=chunk.map_err(|e|format!("Mod stream interrupted: {e}"))?;
        received=received.checked_add(chunk.len() as u64).ok_or("Mod size overflow")?;
        if received>expected_size || received>MAX_MOD_BYTES {
            return Err("Mod download exceeded declared size".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    if received!=expected_size {
        return Err("Mod file size does not match Modrinth metadata".into());
    }
    // JAR is a ZIP container. Reject mislabeled downloads even if their
    // metadata appears otherwise consistent; never execute downloaded files.
    if !bytes.starts_with(b"PK\x03\x04") {
        return Err("Downloaded mod is not a valid JAR/ZIP container".into());
    }
    if !format!("{:x}",Sha512::digest(&bytes)).eq_ignore_ascii_case(digest){
        return Err("Mod SHA-512 integrity check failed".into());
    }
    fs::create_dir_all(dir).await.map_err(|e|e.to_string())?;
    let tmp=dir.join(format!("{filename}.fre-download"));
    fs::write(&tmp,&bytes).await.map_err(|e|e.to_string())?;
    let result=fs::rename(&tmp,&path).await.map_err(|e|e.to_string());
    if result.is_err(){let _=fs::remove_file(&tmp).await;}
    result?;
    Ok(filename.into())
}

#[tauri::command]
pub async fn install_fabric_mod(app: AppHandle, instance_id: String, project_id: String, version_id: Option<String>) -> Result<Vec<String>, String> {
    valid_id(&project_id)?;
    let instance=super::instances::read(&app,&instance_id)?;
    if instance.loader!="fabric" {
        return Err("Switch this Minecraft instance to Fabric before installing mods".into());
    }
    let client=client()?;
    if let Some(ref id)=version_id { valid_id(id)?; }
    let mut queue=VecDeque::from([(project_id,version_id)]);
    let mut visited=HashSet::new();
    let mut versions=Vec::new();
    while let Some((project,version_id))=queue.pop_front() {
        valid_id(&project)?;
        if !visited.insert(project.clone()){continue;}
        if visited.len()>MAX_DEPENDENCIES {return Err("Too many required mod dependencies".into());}
        resolve_project_type(&client,&project).await?;
        let version=select_version(&client,&project,version_id.as_deref(),&instance.version).await?;
        for dependency in version["dependencies"].as_array().into_iter().flatten() {
            if dependency["dependency_type"].as_str()!=Some("required"){continue;}
            let pinned=dependency["version_id"].as_str().map(str::to_string);
            let linked=if let Some(project)=dependency["project_id"].as_str(){
                project.to_string()
            }else if let Some(exact)=pinned.as_deref(){
                valid_id(exact)?;
                let found=fetch_json(&client,&format!("https://api.modrinth.com/v2/version/{exact}")).await?;
                found["project_id"].as_str().ok_or("Dependency version missing project ID")?.to_owned()
            }else{return Err("Unresolvable required mod dependency".into());};
            queue.push_back((linked,pinned));
        }
        versions.push(version);
    }
    // Resolve and validate all dependencies before writing any JARs.
    let directory=mods_dir(&app,&instance_id)?;
    let mut added=Vec::new();
    for version in versions.iter().rev() {
        let name=verified_jar(&client,version,&directory).await?;
        added.push(name);
    }
    // Catalog installations receive the same built-in archive inspection.
    scan_instance_mods(&app, &instance_id).await?;
    Ok(added)
}

#[tauri::command]
pub async fn list_installed_mods(app: AppHandle, instance_id: String) -> Result<Vec<InstalledMod>, String> {
    let directory=mods_dir(&app,&instance_id)?;
    if !fs::try_exists(&directory).await.map_err(|e|e.to_string())?{return Ok(vec![]);}
    let mut dir=fs::read_dir(&directory).await.map_err(|e|e.to_string())?;
    let mut mods=Vec::new();
    while let Some(entry)=dir.next_entry().await.map_err(|e|e.to_string())? {
        if !entry.file_type().await.map_err(|e|e.to_string())?.is_file(){continue;}
        let name=entry.file_name().to_string_lossy().to_string();
        if !valid_filename(&name,".jar") && !valid_filename(&name,".jar.disabled"){continue;}
        let enabled=name.to_ascii_lowercase().ends_with(".jar");
        let bytes=entry.metadata().await.map_err(|e|e.to_string())?.len();
        mods.push(InstalledMod{filename:name,enabled,bytes});
    }
    mods.sort_by(|a,b|a.filename.cmp(&b.filename));
    Ok(mods)
}

#[tauri::command]
pub async fn toggle_installed_mod(app: AppHandle, instance_id: String, filename: String) -> Result<(), String> {
    if !valid_filename(&filename,".jar") && !valid_filename(&filename,".jar.disabled"){
        return Err("Unsafe mod filename".into());
    }
    let directory=mods_dir(&app,&instance_id)?;
    let source=directory.join(&filename);
    let target=if filename.ends_with(".jar.disabled"){
        directory.join(filename.trim_end_matches(".disabled"))
    }else{directory.join(format!("{filename}.disabled"))};
    if fs::try_exists(&target).await.map_err(|e|e.to_string())?{
        return Err("A mod with that name already exists".into());
    }
    fs::rename(&source,&target).await.map_err(|e|e.to_string())
}

#[tauri::command]
pub async fn remove_installed_mod(app: AppHandle, instance_id: String, filename: String) -> Result<(), String> {
    if !valid_filename(&filename,".jar") && !valid_filename(&filename,".jar.disabled"){
        return Err("Unsafe mod filename".into());
    }
    let directory=mods_dir(&app,&instance_id)?;
    fs::remove_file(directory.join(filename)).await.map_err(|e|e.to_string())
}

/// Defensive structural inspection, not a malware signature scanner.
/// Reject corrupt, excessive, or executable-bearing ZIPs before loading mods.
/// This cannot establish that arbitrary Java bytecode is harmless.
fn inspect_jar(path: &Path) -> Result<(), String> {
    const MAX_FILES: usize = 20_000;
    const MAX_UNPACKED: u64 = 768 * 1024 * 1024;
    let file = std::fs::File::open(path).map_err(|e|e.to_string())?;
    if file.metadata().map_err(|e|e.to_string())?.len() > MAX_MOD_BYTES {
        return Err("Mod JAR exceeds the size limit".into());
    }
    let mut archive = zip::ZipArchive::new(file).map_err(|e|format!("Invalid JAR archive: {e}"))?;
    if archive.len()==0 || archive.len()>MAX_FILES {return Err("JAR has an invalid number of entries".into());}
    let mut total=0u64;
    let mut has_mod_descriptor=false;
    for i in 0..archive.len() {
        let mut entry=archive.by_index(i).map_err(|e|e.to_string())?;
        let name=entry.name().replace('\\',"/");
        if name.starts_with('/') || name.split('/').any(|part|part=="..") {
            return Err("JAR contains unsafe paths".into());
        }
        if entry.is_dir() {continue;}
        total=total.checked_add(entry.size()).ok_or("Unpacked size overflow")?;
        if total>MAX_UNPACKED {return Err("JAR expands beyond safe limits".into());}
        let lower=name.to_ascii_lowercase();
        if lower.ends_with(".exe") || lower.ends_with(".bat") || lower.ends_with(".cmd")
            || lower.ends_with(".ps1") || lower.ends_with(".vbs") || lower.ends_with(".scr") {
            return Err(format!("JAR includes a risky executable or script: {name}"));
        }
        if lower=="fabric.mod.json" || lower=="quilt.mod.json" || lower=="meta-inf/mods.toml" || lower=="meta-inf/neoforge.mods.toml" {
            has_mod_descriptor=true;
        }
        // Read every entry to verify decompression and CRC rather than trusting headers.
        std::io::copy(&mut entry,&mut std::io::sink()).map_err(|e|format!("Damaged JAR entry: {e}"))?;
    }
    if !has_mod_descriptor {return Err("Not a recognized Fabric/Quilt/Forge mod archive".into());}
    Ok(())
}

pub async fn scan_instance_mods(app: &AppHandle, instance_id: &str) -> Result<(), String> {
    let directory = mods_dir(app, instance_id)?;
    let root = directory.parent().ok_or("Missing instance directory")?;
    // A symlinked instance or mods folder must never bypass the inspection gate.
    for folder in [root, directory.as_path()] {
        match fs::symlink_metadata(folder).await {
            Ok(meta) if meta.file_type().is_symlink() || !meta.is_dir() =>
                return Err(format!("Unsafe Minecraft directory: {}", folder.display())),
            Ok(_) => {},
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if folder == directory.as_path() { return Ok(()); }
                return Err("Minecraft instance directory is missing".into());
            },
            Err(e) => return Err(e.to_string()),
        }
    }

    // Inspect every enabled JAR, including manually copied files with unusual
    // names. Never silently skip a file that Minecraft itself may try to load.
    let mut entries = fs::read_dir(&directory).await.map_err(|e| e.to_string())?;
    while let Some(entry) = entries.next_entry().await.map_err(|e|e.to_string())? {
        let filename = entry.file_name().to_string_lossy().into_owned();
        if !filename.to_ascii_lowercase().ends_with(".jar") { continue; }
        if !valid_filename(&filename, ".jar") {
            return Err(format!("Unsafe mod filename: {filename}. Rename or remove it before launching."));
        }
        let metadata = fs::symlink_metadata(entry.path()).await.map_err(|e|e.to_string())?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(format!("Refusing non-regular mod entry: {filename}"));
        }
        let path = entry.path();
        tokio::task::spawn_blocking(move || inspect_jar(&path))
            .await.map_err(|e|e.to_string())?
            .map_err(|e|format!("Mod {filename} failed archive inspection: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspect_jar_rejects_non_zip() {
        let path=std::env::temp_dir().join(format!("fre-fake-mod-{}.jar",uuid::Uuid::new_v4()));
        std::fs::write(&path,b"not a zip archive").unwrap();
        assert!(inspect_jar(&path).is_err());
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn inspect_jar_accepts_normal_fabric_metadata() {
        use std::io::Write;
        let path=std::env::temp_dir().join(format!("fre-normal-mod-{}.jar",uuid::Uuid::new_v4()));
        let file=std::fs::File::create(&path).unwrap();
        let mut zip=zip::ZipWriter::new(file);
        zip.start_file("fabric.mod.json",zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(br#"{"schemaVersion":1,"id":"safe-test","version":"1.0.0"}"#).unwrap();
        zip.finish().unwrap();
        assert!(inspect_jar(&path).is_ok());
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn inspect_jar_rejects_embedded_executables() {
        use std::io::Write;
        let path=std::env::temp_dir().join(format!("fre-risky-mod-{}.jar",uuid::Uuid::new_v4()));
        let file=std::fs::File::create(&path).unwrap();
        let mut zip=zip::ZipWriter::new(file);
        zip.start_file("fabric.mod.json",zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(b"{}").unwrap();
        zip.start_file("payload.exe",zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(b"dummy").unwrap();
        zip.finish().unwrap();
        assert!(inspect_jar(&path).is_err());
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn validates_mod_filenames(){
        assert!(valid_filename("sodium-fabric-1.21.jar",".jar"));
        assert!(!valid_filename("../evil.jar",".jar"));
        assert!(!valid_filename("foo.zip",".jar"));
        assert!(valid_filename("abc.jar.disabled",".jar.disabled"));
        assert!(valid_filename("мій-мод-1.21.jar",".jar"));
        assert!(!valid_filename("subdir/evil.jar",".jar"));
        assert!(!valid_filename("bad\\\\name.jar",".jar"));
    }
}
