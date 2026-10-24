//! Install a Modrinth .mrpack into a newly-created FRE LAUNCH instance.
//! Independent implementation of the published modrinth.index.json format.
//! Security: verified SHA-512, strict URL hosts, path validation, ZIP limits,
//! isolated staging and failure rollback. No subprocesses or installer JARs.
use futures::StreamExt;
use serde::Deserialize;
use serde_json::Value;
use sha1::Sha1;
use sha2::{Digest,Sha512};
use std::{collections::{HashMap,HashSet},fs,io::Read,path::{Path,PathBuf},time::Duration};
use tauri::{AppHandle,Emitter,Manager};

const MAX_PACK:u64=128*1024*1024;
const MAX_UNCOMPRESSED:u64=384*1024*1024;
const MAX_DOWNLOAD:u64=256*1024*1024;
const MAX_CONTENT:u64=2*1024*1024*1024;
const MAX_FILES:usize=3000;

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
struct Index {
    format_version:u32,
    game:String,
    name:String,
    files:Vec<PackEntry>,
    dependencies:HashMap<String,String>,
}
#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
struct PackEntry {
    path:String,
    hashes:HashMap<String,String>,
    downloads:Vec<String>,
    file_size:u64,
    env:Option<HashMap<String,String>>,
}
#[derive(Clone,serde::Serialize)]
#[serde(rename_all="camelCase")]
struct Progress {
    current:usize,
    total:usize,
    message:String,
}
fn notify(app:&AppHandle,current:usize,total:usize,message:&str){
    let _=app.emit("fre-modpack-progress",Progress{
        current,total,message:message.into()
    });
}
fn safe_relative(value:&str)->Result<PathBuf,String>{
    if value.is_empty()||value.len()>230||value.starts_with('/')||
        value.contains('\\')||value.contains(':')||value.chars().any(char::is_control){
        return Err("Небезпечний шлях у модпаку".into());
    }
    let mut out=PathBuf::new();
    for segment in value.split('/') {
        if segment.is_empty()||segment=="."||segment==".."||segment.ends_with(' ')||segment.ends_with('.') {
            return Err(format!("Недозволений шлях модпака: {value}"));
        }
        let stem=segment.split('.').next().unwrap_or("").to_ascii_uppercase();
        if matches!(stem.as_str(),"CON"|"PRN"|"AUX"|"NUL"|"COM1"|"COM2"|"COM3"|"COM4"|"COM5"|"COM6"|"COM7"|"COM8"|"COM9"|"LPT1"|"LPT2"|"LPT3"|"LPT4"|"LPT5"|"LPT6"|"LPT7"|"LPT8"|"LPT9"){
            return Err("Зарезервоване ім'я Windows у модпаку".into());
        }
        out.push(segment);
    }
    Ok(out)
}
fn valid_hash(s:&str,len:usize)->bool{
    s.len()==len&&s.bytes().all(|b|b.is_ascii_hexdigit())
}
fn allowed_url(value:&str)->Result<reqwest::Url,String>{
    let url=reqwest::Url::parse(value).map_err(|_|"Некоректне посилання на файл")?;
    if url.scheme()!="https"||url.username()!=""||url.password().is_some()||url.port().is_some(){
        return Err("Потрібне безпечне HTTPS-посилання".into());
    }
    if !matches!(url.host_str(),Some(
        "cdn.modrinth.com"|"github.com"|"raw.githubusercontent.com"|
        "release-assets.githubusercontent.com"|"objects.githubusercontent.com"|"gitlab.com"
    )){
        return Err("Файл модпака розміщено поза перевіреними джерелами Modrinth/GitHub/GitLab".into());
    }
    Ok(url)
}
fn client()->Result<reqwest::Client,String>{
    reqwest::Client::builder().user_agent(concat!("FRE-LAUNCH/",env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(90))
        .redirect(reqwest::redirect::Policy::custom(|attempt|{
            if attempt.previous().len()>4{return attempt.error("Забагато перенаправлень");}
            match allowed_url(attempt.url().as_str()){
                Ok(_)=>attempt.follow(),
                Err(_)=>attempt.error("Недозволене перенаправлення")
            }
        }))
        .build().map_err(|e|e.to_string())
}
async fn download(client:&reqwest::Client,url:&str,max:u64)->Result<Vec<u8>,String>{
    allowed_url(url)?;
    let response=client.get(url).send().await.map_err(|e|format!("Завантаження перервано: {e}"))?
        .error_for_status().map_err(|e|format!("Помилка сервера файлів: {e}"))?;
    if response.content_length().is_some_and(|len|len>max){return Err("Файл перевищує обмеження розміру".into());}
    let mut result=Vec::new();
    let mut stream=response.bytes_stream();
    while let Some(chunk)=stream.next().await {
        let chunk=chunk.map_err(|e|format!("З'єднання перервано: {e}"))?;
        if result.len() as u64 + chunk.len() as u64>max{
            return Err("Файл завеликий".into());
        }
        result.extend_from_slice(&chunk);
    }
    Ok(result)
}
fn parse_pack(bytes:Vec<u8>)->Result<(Index,Vec<(PathBuf,Vec<u8>)>),String>{
    if !bytes.starts_with(b"PK\x03\x04"){return Err("Модпак не є ZIP/.mrpack".into());}
    let mut zip=zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e|format!("Пошкоджений .mrpack: {e}"))?;
    if zip.len()==0||zip.len()>MAX_FILES{return Err("Небезпечна кількість файлів в .mrpack".into());}
    let index_bytes={
        let mut entry=zip.by_name("modrinth.index.json").map_err(|_|"У модпаку немає modrinth.index.json".to_string())?;
        if entry.size()>2*1024*1024{return Err("Маніфест модпака завеликий".into());}
        let mut bytes=Vec::new();entry.read_to_end(&mut bytes).map_err(|e|e.to_string())?;
        bytes
    };
    let index:Index=serde_json::from_slice(&index_bytes).map_err(|e|format!("Некоректний modrinth.index.json: {e}"))?;
    if index.format_version!=1||index.game!="minecraft"{
        return Err("Непідтримувана версія формату .mrpack".into());
    }
    if index.files.len()>MAX_FILES{return Err("Занадто багато файлів у модпаку".into());}
    let mut overrides=Vec::new();
    let mut total=0u64;
    let mut seen=HashSet::new();
    for n in 0..zip.len(){
        let mut file=zip.by_index(n).map_err(|e|e.to_string())?;
        let full=file.name();
        let mode=file.unix_mode().unwrap_or(0);
        if mode & 0o170000==0o120000{return Err("ZIP містить символьне посилання".into());}
        // Validate even entries we do not extract, against zip-slip and duplicates.
        let clean=full.trim_end_matches('/');
        let _=safe_relative(clean)?;
        if file.is_dir(){continue;}
        total=total.checked_add(file.size()).ok_or("Переповнення ZIP")?;
        if total>MAX_UNCOMPRESSED{return Err("Архів перевищує ліміт розпакування".into());}
        let rel=if let Some(s)=full.strip_prefix("client-overrides/"){
            Some(s)
        }else{full.strip_prefix("overrides/")};
        if let Some(rel)=rel {
            let dest=safe_relative(rel)?;
            let key=dest.to_string_lossy().to_ascii_lowercase();
            if !seen.insert(key){return Err("У модпаку дублюються шляхи overrides".into());}
            let lower=rel.to_ascii_lowercase();
            if [".exe",".dll",".bat",".cmd",".ps1",".vbs",".scr",".msi",".com"]
                .iter().any(|ext|lower.ends_with(ext)){
                return Err("Пак містить виконуваний Windows-файл в overrides".into());
            }
            if file.size()>64*1024*1024{return Err("Один override-файл більший за 64 МБ".into());}
            let mut data=Vec::new();
            file.read_to_end(&mut data).map_err(|e|format!("Помилка ZIP: {e}"))?;
            overrides.push((dest,data));
        }
    }
    let mut combined=seen;
    let mut total_download=0u64;
    for entry in &index.files {
        let path=safe_relative(&entry.path)?;
        let key=path.to_string_lossy().to_ascii_lowercase();
        if !combined.insert(key){return Err("У модпаку конфліктують шляхи файлів".into());}
        if entry.file_size==0||entry.file_size>MAX_DOWNLOAD{
            return Err("Некоректний розмір файлу в маніфесті".into());
        }
        total_download=total_download.checked_add(entry.file_size).ok_or("Переповнення розміру")?;
        if total_download>MAX_CONTENT{return Err("Модпак потребує більше 2 ГБ завантажень".into());}
        if !entry.hashes.get("sha512").is_some_and(|hash|valid_hash(hash,128)){
            return Err("В одному з файлів немає коректної SHA-512 перевірки".into());
        }
    }
    Ok((index,overrides))
}
fn write_staged(stage:&Path,relative:&Path,bytes:&[u8])->Result<(),String>{
    let path=stage.join(relative);
    if let Some(parent)=path.parent(){fs::create_dir_all(parent).map_err(|e|e.to_string())?;}
    let mut file=fs::OpenOptions::new().write(true).create_new(true)
        .open(&path).map_err(|e|format!("Неможливо створити файл збірки: {e}"))?;
    std::io::Write::write_all(&mut file,bytes).map_err(|e|e.to_string())?;
    file.sync_all().map_err(|e|e.to_string())
}
fn move_staged_files(from:&Path,to:&Path)->Result<(),String>{
    for entry in fs::read_dir(from).map_err(|e|e.to_string())?{
        let entry=entry.map_err(|e|e.to_string())?;
        let metadata=fs::symlink_metadata(entry.path()).map_err(|e|e.to_string())?;
        if metadata.file_type().is_symlink(){return Err("Небезпечне посилання в папці підготовки".into());}
        let destination=to.join(entry.file_name());
        if metadata.is_dir(){
            fs::create_dir_all(&destination).map_err(|e|e.to_string())?;
            move_staged_files(&entry.path(),&destination)?;
        }else if metadata.is_file(){
            if fs::symlink_metadata(&destination).is_ok(){
                return Err("Конфлікт імені файлу в новій збірці".into());
            }
            fs::rename(entry.path(),destination).map_err(|e|format!("Помилка перенесення файлу: {e}"))?;
        }else{return Err("Невідомий тип файлу в збірці".into());}
    }
    Ok(())
}
fn choose_loader(dep:&HashMap<String,String>)->Result<(&'static str,Option<String>),String>{
    if dep.contains_key("forge")||dep.contains_key("neoforge")||dep.contains_key("neo-forge"){
        return Err("Цей модпак потребує Forge/NeoForge. Ці модлоадери ще не підтримуються FRE LAUNCH.".into());
    }
    if dep.contains_key("fabric-loader")&&dep.contains_key("quilt-loader"){
        return Err("Маніфест одночасно вимагає Fabric і Quilt".into());
    }
    if let Some(version)=dep.get("fabric-loader"){return Ok(("fabric",Some(version.clone())));}
    if let Some(version)=dep.get("quilt-loader"){return Ok(("quilt",Some(version.clone())));}
    Ok(("vanilla",None))
}
#[tauri::command]
pub async fn install_modrinth_modpack(app:AppHandle,project_id:String,version_id:Option<String>)
    ->Result<super::instances::Instance,String>{
    if project_id.is_empty()||project_id.len()>80||!project_id.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-'||b==b'_'){
        return Err("Некоректний Modrinth project ID".into());
    }
    let client=client()?;
    let project:Value=client.get(format!("https://api.modrinth.com/v2/project/{project_id}"))
        .send().await.map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?
        .json().await.map_err(|e|e.to_string())?;
    if project["project_type"].as_str()!=Some("modpack"){return Err("Це не модпак Modrinth".into());}
    let versions:Vec<Value>=client.get(format!("https://api.modrinth.com/v2/project/{project_id}/version"))
        .send().await.map_err(|e|e.to_string())?.error_for_status().map_err(|e|e.to_string())?
        .json().await.map_err(|e|e.to_string())?;
    let selected=match version_id.as_deref(){
        Some(id)=>versions.iter().find(|v|v["id"].as_str()==Some(id))
            .ok_or("Вибрана версія не належить цьому модпаку")?,
        None=>versions.first().ok_or("Модпак не має опублікованих версій")?,
    };
    let archive=selected["files"].as_array().ok_or("Modrinth не надав файли збірки")?
        .iter().find(|f|f["filename"].as_str().is_some_and(|n|n.ends_with(".mrpack")))
        .ok_or("У версії немає файлу .mrpack")?;
    let source=archive["url"].as_str().ok_or("Немає посилання .mrpack")?;
    let size=archive["size"].as_u64().ok_or("Невідомий розмір .mrpack")?;
    if size==0||size>MAX_PACK{return Err("Модпак надто великий для безпечного встановлення".into());}
    let digest=archive["hashes"]["sha512"].as_str().ok_or("Немає хешу .mrpack")?;
    if !valid_hash(digest,128){return Err("Некоректний SHA-512 .mrpack".into());}
    notify(&app,0,1,"Завантаження .mrpack");
    let bytes=download(&client,source,MAX_PACK).await?;
    if bytes.len() as u64!=size||!format!("{:x}",Sha512::digest(&bytes)).eq_ignore_ascii_case(digest){
        return Err("Неповне завантаження або невірний SHA-512 архіву".into());
    }
    notify(&app,0,1,"Перевірка структури .mrpack");
    let (index,overrides)=tokio::task::spawn_blocking(move||parse_pack(bytes))
        .await.map_err(|e|e.to_string())??;
    let game=index.dependencies.get("minecraft").ok_or("У модпаку не задано версію Minecraft")?.clone();
    super::minecraft::validate_version(&game)?;
    let (loader,loader_version)=choose_loader(&index.dependencies)?;
    if let Some(v)=loader_version.as_deref(){super::minecraft::validate_version(v)?;}
    let name=if index.name.chars().count()<=48&& !index.name.trim().is_empty() {
        index.name.trim().to_string()
    }else{
        project["title"].as_str().unwrap_or("Модпак").chars().take(48).collect()
    };
    // Stage away from the user's existing worlds/instances.
    let base=app.path().app_data_dir().map_err(|e|e.to_string())?.join("minecraft");
    fs::create_dir_all(&base).map_err(|e|e.to_string())?;
    let stage=base.join(format!(".fre-mrpack-{}",uuid::Uuid::new_v4()));
    fs::create_dir(&stage).map_err(|e|e.to_string())?;
    let result=async {
        for (rel,data) in &overrides {
            write_staged(&stage,rel,data)?;
        }
        let active:Vec<&PackEntry>=index.files.iter().filter(|item|
            item.env.as_ref().and_then(|env|env.get("client")).map(String::as_str)!=Some("unsupported")
        ).collect();
        let total=active.len();
        for (number,entry) in active.iter().enumerate(){
            let path=safe_relative(&entry.path)?;
            notify(&app,number,total,&format!("Файл {}/{}: {}",number+1,total,entry.path));
            let url=entry.downloads.iter().find(|url|allowed_url(url).is_ok())
                .ok_or(format!("Немає дозволеного HTTPS-джерела для {}",entry.path))?;
            let file=download(&client,url,MAX_DOWNLOAD).await?;
            if file.len() as u64!=entry.file_size{
                return Err(format!("Невірний розмір файлу {}",entry.path));
            }
            let expected=entry.hashes.get("sha512").ok_or("Немає SHA-512 файлу")?;
            if !format!("{:x}",Sha512::digest(&file)).eq_ignore_ascii_case(expected){
                return Err(format!("SHA-512 не збігається: {}",entry.path));
            }
            if let Some(sha1)=entry.hashes.get("sha1"){
                if valid_hash(sha1,40)&&!format!("{:x}",Sha1::digest(&file)).eq_ignore_ascii_case(sha1){
                    return Err(format!("SHA-1 не збігається: {}",entry.path));
                }
            }
            write_staged(&stage,&path,&file)?;
        }
        notify(&app,total,total,"Створення окремої Minecraft-збірки");
        let created=super::instances::create_instance(app.clone(),name,game)?;
        let id=created.id.clone();
        let placement=(||->Result<super::instances::Instance,String>{
            let dir=super::instances::game_directory(&app,&id,&created.version)?;
            // The newly created instance has only default folders; never delete them.
            // Move only staged, verified content into that dedicated directory.
            move_staged_files(&stage,&dir)?;
            super::game_folders::prepare_folders(&dir)?;
            let instance=super::instances::set_instance_loader(app.clone(),id.clone(),loader.into(),loader_version)?;
            Ok(instance)
        })();
        match placement {
            Ok(instance)=>{
                // Verify all .jar archives before marking a new pack ready.
                if let Err(error)=super::mods::scan_instance_mods(&app,&id).await {
                    let _=super::instances::delete_instance(app.clone(),id,true);
                    return Err(format!("Перевірка модів завершилася помилкою: {error}"));
                }
                Ok(instance)
            }
            Err(error)=>{
                let _=super::instances::delete_instance(app.clone(),id,true);
                Err(error)
            }
        }
    }.await;
    if stage.exists(){let _=fs::remove_dir_all(&stage);}
    if let Ok(ref instance)=result {
        notify(&app,index.files.len(),index.files.len(),"Модпак встановлено");
        super::diagnostics::write(&app,"MRPACK_INSTALLED",&format!("{} ({})",instance.name,instance.version));
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn path_traversal_protection(){
        for path in ["../run.exe","C:/bad","mods/../../bad","/root","mods\\evil.jar","NUL","mods/COM1.jar","foo//bar"]{
            assert!(safe_relative(path).is_err(),"{path}");
        }
        assert!(safe_relative("mods/fabric-api.jar").is_ok());
        assert!(safe_relative("config/create-client.toml").is_ok());
    }
    #[test]fn reject_untrusted_hosts(){
        assert!(allowed_url("https://cdn.modrinth.com/data/x/file.jar").is_ok());
        assert!(allowed_url("http://cdn.modrinth.com/file.jar").is_err());
        assert!(allowed_url("https://cdn.modrinth.com.evil.test/x").is_err());
        assert!(allowed_url("https://127.0.0.1/x").is_err());
    }
}
