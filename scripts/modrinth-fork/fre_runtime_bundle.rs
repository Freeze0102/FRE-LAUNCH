//! Recover the MIT runtime when the desktop EXE is moved without its resources.
use serde::Deserialize;
use sha2::{Digest,Sha256};
use std::{collections::BTreeSet,fs,io::{Read,Write},path::{Component,Path,PathBuf},sync::Mutex};
const PAYLOAD:&[u8]=include_bytes!("../fre-native-runtime.zip");
const MANIFEST:&str="_FRE-RUNTIME.json";
static EXTRACTION:Mutex<()>=Mutex::new(());
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {schema:u32,files:Vec<File>}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {path:String,size:u64,sha256:String}
fn error(message:impl std::fmt::Display)->String {format!("Cannot restore the bundled FRE game: {message}")}
fn digest(data:&[u8])->String {format!("{:x}",Sha256::digest(data))}
fn safe(path:&str)->bool {
    !path.is_empty() && !path.contains(['\\',':']) && Path::new(path).components().all(|part|matches!(part,Component::Normal(_)))
}
fn read_manifest(payload:&[u8])->Result<Manifest,String> {
    let mut archive=zip::ZipArchive::new(std::io::Cursor::new(payload)).map_err(error)?;
    let mut text=Vec::new();archive.by_name(MANIFEST).map_err(error)?.take((1<<20)+1).read_to_end(&mut text).map_err(error)?;
    if text.len()>1<<20 {return Err(error("runtime manifest is oversized"));}
    let manifest:Manifest=serde_json::from_slice(&text).map_err(error)?;
    if manifest.schema!=1 || manifest.files.len()>2048 {return Err(error("unknown runtime manifest"));}
    let mut paths=BTreeSet::new();let mut size=0u64;
    for file in &manifest.files {
        size=size.checked_add(file.size).ok_or_else(||error("runtime size overflow"))?;
        if !safe(&file.path) || file.size>256<<20 || size>1<<30 || file.sha256.len()!=64 || !file.sha256.bytes().all(|b|b.is_ascii_hexdigit()) || !paths.insert(file.path.to_lowercase()) {return Err(error("unsafe runtime manifest entry"));}
    }
    if archive.len()!=manifest.files.len()+1 {return Err(error("unexpected runtime archive entries"));}
    let mut names=BTreeSet::new();
    for index in 0..archive.len() {
        let entry=archive.by_index(index).map_err(error)?;
        if !names.insert(entry.name().to_lowercase()) || !safe(entry.name()) || entry.is_dir() || entry.unix_mode().is_some_and(|mode|mode & 0o170000==0o120000) {return Err(error("unsafe runtime archive path"));}
        if entry.name()!=MANIFEST && !paths.contains(&entry.name().to_lowercase()) {return Err(error("undeclared runtime archive path"));}
    }
    Ok(manifest)
}
fn ordinary_directory(path:&Path)->bool {path.symlink_metadata().is_ok_and(|m|m.file_type().is_dir())}
fn matches(directory:&Path,manifest:&Manifest)->bool {
    if !ordinary_directory(directory) {return false;}
    let mut directories=BTreeSet::new();let mut expected=BTreeSet::new();
    for file in &manifest.files {
        let relative=Path::new(&file.path);expected.insert(relative.to_path_buf());
        let mut parent=relative.parent();
        while let Some(path)=parent.filter(|path|!path.as_os_str().is_empty()) {
            if !ordinary_directory(&directory.join(path)) {return false;}
            directories.insert(path.to_path_buf());parent=path.parent();
        }
        let path=directory.join(relative);
        if !path.symlink_metadata().is_ok_and(|m|m.file_type().is_file() && m.len()==file.size) {return false;}
        let Ok(mut input)=fs::File::open(&path) else{return false};
        let mut hash=Sha256::new();let mut block=[0u8;65536];
        loop {match input.read(&mut block) {Ok(0)=>break,Ok(n)=>hash.update(&block[..n]),Err(_)=>return false}}
        if format!("{:x}",hash.finalize())!=file.sha256 {return false;}
    }
    let mut pending=vec![PathBuf::new()];
    while let Some(relative)=pending.pop() {
        let Ok(entries)=fs::read_dir(directory.join(&relative)) else{return false};
        for entry in entries {
            let Ok(entry)=entry else{return false};let child=relative.join(entry.file_name());
            let Ok(kind)=entry.file_type() else{return false};
            if kind.is_dir() && directories.contains(&child) {pending.push(child)}
            else if !kind.is_file() || !expected.contains(&child) {return false;}
        }
    }
    true
}
fn ensure(root:&Path,external:&Path,payload:&[u8])->Result<PathBuf,String> {
    let manifest=read_manifest(payload)?;
    if manifest.files.is_empty() {
        // Source-only CI can compile the app without distributing a native game.
        if external.join("BUILD-INFO.json").is_file() {return Ok(external.into());}
        return Err("This developer build has no native runtime. Install the complete FRE LAUNCH Windows build.".into());
    }
    if matches(external,&manifest) {return Ok(external.into());}
    let _guard=EXTRACTION.lock().map_err(error)?;
    fs::create_dir_all(root).map_err(error)?;
    if !ordinary_directory(root) {return Err(error("runtime root must be an ordinary directory"));}
    let parent=root.join("native-runtime");
    if !parent.exists() {match fs::create_dir(&parent) {Ok(())=>{},Err(e) if e.kind()==std::io::ErrorKind::AlreadyExists=>{},Err(e)=>return Err(error(e))}}
    if !ordinary_directory(&parent) {return Err(error("runtime cache must not be linked"));}
    let key=digest(payload);
    for slot in 0..8 {
        let destination=parent.join(format!("{key}-{slot}"));
        if destination.symlink_metadata().is_ok() {
            if matches(&destination,&manifest) {return Ok(destination);}
            // Preserve damaged caches and unexpected files; never delete user data.
            continue;
        }
        let staging=tempfile::Builder::new().prefix(".runtime-").tempdir_in(&parent).map_err(error)?;
        let mut archive=zip::ZipArchive::new(std::io::Cursor::new(payload)).map_err(error)?;
        for file in &manifest.files {
            let mut input=archive.by_name(&file.path).map_err(error)?;
            if input.size()!=file.size {return Err(error("runtime entry size mismatch"));}
            let target=staging.path().join(&file.path);
            fs::create_dir_all(target.parent().unwrap()).map_err(error)?;
            let mut output=fs::OpenOptions::new().write(true).create_new(true).open(target).map_err(error)?;
            let mut bytes=Vec::new();input.by_ref().take(file.size+1).read_to_end(&mut bytes).map_err(error)?;
            if bytes.len() as u64!=file.size || digest(&bytes)!=file.sha256 {return Err(error("runtime entry checksum mismatch"));}
            output.write_all(&bytes).and_then(|()|output.sync_all()).map_err(error)?;
        }
        match fs::rename(staging.path(),&destination) {
            Ok(())=>return Ok(destination),
            Err(_) if matches(&destination,&manifest)=>return Ok(destination),
            Err(e) if destination.symlink_metadata().is_err()=>return Err(error(e)),
            Err(_)=>{},
        }
    }
    Err(error("eight preserved runtime caches are damaged; inspect the native-runtime folder"))
}
pub fn resolve(root:&Path,external:&Path)->Result<PathBuf,String> {ensure(root,external,PAYLOAD)}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    fn payload(path:&str,data:&[u8])->Vec<u8> {
        let manifest=serde_json::json!({"schema":1,"files":[{"path":path,"size":data.len(),"sha256":digest(data)}]});
        let mut archive=zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options=zip::write::SimpleFileOptions::default();
        archive.start_file(path,options).unwrap();archive.write_all(data).unwrap();
        archive.start_file(MANIFEST,options).unwrap();archive.write_all(&serde_json::to_vec(&manifest).unwrap()).unwrap();
        archive.finish().unwrap().into_inner()
    }
    #[test]
    #[ignore = "Extracts and launches the full embedded Windows runtime"]
    fn actual_embedded_runtime_recovers_and_starts_without_external_files() {
        let manifest=read_manifest(PAYLOAD).unwrap();
        for name in ["BUILD-INFO.json","fre-rust-client.exe","fre-rust-prepare.exe","fre-mixins.exe","LICENSE.txt","fre-engine-source.zip"] {
            assert!(manifest.files.iter().any(|file|file.path==name),"missing {name}");
        }
        let temporary=tempfile::tempdir().unwrap();let external=temporary.path().join("no-files-beside-exe");
        let restored=ensure(temporary.path(),&external,PAYLOAD).unwrap();
        assert!(matches(&restored,&manifest));
        assert_eq!(ensure(temporary.path(),&external,PAYLOAD).unwrap(),restored);
        let output=std::process::Command::new(restored.join("fre-rust-client.exe")).arg("--launch-api").output().unwrap();
        assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));
        assert!(String::from_utf8_lossy(&output.stdout).contains("launch_api"));
        eprintln!("Recovered {} verified MIT runtime files and started the game launch API",manifest.files.len());
    }
    #[test] fn moved_executable_recovers_runtime_and_reuses_it() {
        let temp=tempfile::tempdir().unwrap();let bytes=payload("fre-rust-client.exe",b"owned runtime");
        let external=temp.path().join("missing-external");let root=temp.path().join("cache");
        let first=ensure(&root,&external,&bytes).unwrap();
        assert_eq!(fs::read(first.join("fre-rust-client.exe")).unwrap(),b"owned runtime");
        assert_eq!(ensure(&root,&external,&bytes).unwrap(),first);
        assert_eq!(fs::read_dir(root.join("native-runtime")).unwrap().count(),1);
    }
    #[test] fn damaged_and_unexpected_cache_files_are_preserved_and_repaired_separately() {
        let temp=tempfile::tempdir().unwrap();let bytes=payload("fre-rust-client.exe",b"owned runtime");
        let external=temp.path().join("missing");let first=ensure(temp.path(),&external,&bytes).unwrap();
        fs::write(first.join("fre-rust-client.exe"),b"broken").unwrap();fs::write(first.join("world.dat"),b"preserve").unwrap();
        let second=ensure(temp.path(),&external,&bytes).unwrap();assert_ne!(first,second);
        assert_eq!(fs::read(first.join("world.dat")).unwrap(),b"preserve");
        assert_eq!(fs::read(second.join("fre-rust-client.exe")).unwrap(),b"owned runtime");
    }
    #[test] fn unsafe_or_tampered_embedded_archives_are_rejected() {
        let temp=tempfile::tempdir().unwrap();
        for path in ["../escape","C:/escape","nested/../../escape","nested\\escape"] {
            assert!(ensure(temp.path(),&temp.path().join("missing"),&payload(path,b"bad")).is_err());
        }
        let mut bytes=payload("game.exe",b"valid");bytes[0]=0;
        assert!(ensure(temp.path(),&temp.path().join("missing"),&bytes).is_err());
        assert!(!temp.path().join("escape").exists());
    }
}
