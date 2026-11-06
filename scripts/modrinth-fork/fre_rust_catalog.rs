//! Live native-mod catalog and non-executing, fail-closed file checks.
use super::mod_audit;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::{Path, PathBuf}, sync::LazyLock, time::{Duration, SystemTime, UNIX_EPOCH}};
use tokio::sync::Mutex;
use url::Url;

const INDEX_URL: &str = "https://horizon-rust.github.io/horizon-mods/index.json";
const MAX_INDEX: u64 = 16 << 20;
const MAX_MOD: u64 = 32 << 20;
const NATIVE_HOOKS: u64 = 80;
pub static OPERATIONS: Mutex<()> = Mutex::const_new(());
static CACHE: LazyLock<Mutex<Option<(u64, Index)>>> = LazyLock::new(|| Mutex::new(None));
fn now() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() }
fn digest(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }

#[derive(Clone, Deserialize, Serialize)]
pub struct Index {
    schema: u32, #[serde(default)] generated: String,
    projects: Vec<Project>, #[serde(default)] revoked: Vec<Revoked>,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct Project {
    pub id: String, pub kind: String, pub name: String, #[serde(default)] pub summary: String,
    #[serde(default)] pub description: String, #[serde(default)] pub authors: Vec<String>,
    pub license: String, pub source: String, #[serde(default)] pub icon: Option<String>,
    #[serde(default)] pub categories: Vec<String>, #[serde(default)] pub tags: Vec<String>,
    #[serde(default)] pub downloads: u64, #[serde(default)] pub key: Option<String>, pub versions: Vec<Version>,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct Version {
    pub version: String, pub api: String, pub game: String,
    #[serde(default)] pub published: String, pub file: File,
    #[serde(default)] pub hooks: Option<u64>,
    #[serde(skip_deserializing)] pub supported: bool,
    #[serde(skip_deserializing)] pub unsupported_reason: Option<String>,
}
#[derive(Clone, Deserialize, Serialize)]
pub struct File { pub name: String, pub url: String, pub size: u64, pub sha256: String, #[serde(default)] pub signature: Option<String> }
#[derive(Clone, Deserialize, Serialize)]
struct Revoked { id: String, #[serde(default)] sha256: Option<String>, reason: String }
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct View { pub projects: Vec<Project>, pub generated: String, pub fetched_at: u64, pub stale: bool, pub warning: Option<String> }
#[derive(Clone, Serialize)]
pub struct Check { pub code: &'static str, pub state: &'static str, pub detail: String }
#[derive(Clone, Serialize)]
#[serde(rename_all="camelCase")]
pub struct Report {
    pub project_id: String, pub version: String, pub filename: String, pub sha256: String,
    pub checked_at: u64, pub installable: bool, pub checks: Vec<Check>,
    pub facts: Option<mod_audit::PackageFacts>,
}
fn check(code: &'static str, state: &'static str, detail: impl Into<String>) -> Check { Check {code,state,detail:detail.into()} }

fn https(url: &Url) -> bool {
    url.scheme()=="https" && url.username().is_empty() && url.password().is_none()
        && url.port_or_known_default()==Some(443) && url.fragment().is_none()
}
fn github_repo(value: &str) -> Result<String,String> {
    let url=Url::parse(value).map_err(|_|"Invalid source URL")?;
    if !https(&url) || url.host_str()!=Some("github.com") || url.query().is_some() { return Err("Source must be a public GitHub repository URL".into()); }
    let parts:Vec<_>=url.path_segments().ok_or("Missing repository")?.collect();
    if parts.len()<2 || parts[..2].iter().any(|part| part.is_empty() || [".",".."].contains(part) || !part.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))) { return Err("Invalid GitHub repository".into()); }
    Ok(format!("{}/{}",parts[0],parts[1]))
}
fn file_name(value: &str) -> bool {
    (6..=128).contains(&value.len()) && !value.starts_with('.') && value.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
pub fn validate_file(project: &Project, version: &Version) -> Result<(),String> {
    if !mod_audit::valid_id(&project.id) || version.game!="26.3" || version.version.is_empty() || version.version.len()>80
        || !file_name(&version.file.name) || !mod_audit::sha256(&version.file.sha256) || version.file.size==0 || version.file.size>MAX_MOD {
        return Err("Invalid file identity, game version, size or SHA-256".into());
    }
    let repo=github_repo(&project.source)?;
    let url=Url::parse(&version.file.url).map_err(|_|"Invalid download URL")?;
    if !https(&url) || url.host_str()!=Some("github.com") || url.query().is_some() { return Err("Only public GitHub release assets are accepted".into()); }
    let parts:Vec<_>=url.path_segments().ok_or("Invalid release URL")?.collect();
    if parts.len()!=6 || parts[2]!="releases" || parts[3]!="download" || parts[4].is_empty() || parts[5]!=version.file.name
        || ![repo.as_str(),"horizon-rust/horizon-mods"].contains(&format!("{}/{}",parts[0],parts[1]).as_str()) {
        return Err("Download does not belong to this project or the registry".into());
    }
    let extension=match project.kind.as_str() {"mod"=>".hmod","native-mod"=>".hnmod","resourcepack"=>".zip","shader"=>".hshader",_=>return Err("Unsupported catalog kind".into())};
    if !version.file.name.ends_with(extension) { return Err("File extension does not match the catalog kind".into()); }
    Ok(())
}
fn allowed_host(url: &Url) -> bool {
    https(url) && matches!(url.host_str(),Some("github.com"|"horizon-rust.github.io"|"release-assets.githubusercontent.com"|"objects.githubusercontent.com"|"github-releases.githubusercontent.com"))
}
async fn fetch_primary(url: &str, limit: u64) -> Result<Vec<u8>,String> {
    let parsed=Url::parse(url).map_err(|e|e.to_string())?;
    if !allowed_host(&parsed) { return Err("Blocked download host".into()); }
    let client=reqwest::Client::builder().user_agent("FRE-LAUNCH/native-mod-checks")
        .http1_only().timeout(Duration::from_secs(60)).connect_timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len()>=5 || !allowed_host(attempt.url()) { attempt.error("Blocked mod download redirect") } else { attempt.follow() }
        })).build().map_err(|e|e.to_string())?;
    let mut response=None;
    for attempt in 0..2 {
        let mut request_url=parsed.clone();
        if attempt>0 && parsed.host_str()==Some("github.com") {
            request_url.query_pairs_mut().append_pair("download","1").append_pair("fre_retry",&now().to_string());
        }
        let result=client.get(request_url).header("Cache-Control","no-cache").send().await;
        match result {
            Ok(value) if value.status().is_server_error() && attempt==0 => { tokio::time::sleep(Duration::from_millis(300)).await; },
            Ok(value)=>{ response=Some(value.error_for_status().map_err(|error|format!("Mod catalog/download returned HTTP {} ({})",error.status().map_or(0,|status|status.as_u16()),error.url().and_then(|url|url.host_str()).unwrap_or("download server")))?); break; },
            Err(_) if attempt==0=>{ tokio::time::sleep(Duration::from_millis(300)).await; },
            Err(_)=>return Err("Cannot reach the public mod catalog/download".into()),
        }
    }
    let mut response=response.ok_or("Mod download did not complete")?;
    if response.content_length().is_some_and(|size|size>limit) { return Err("Download exceeds size limit".into()); }
    let mut data=Vec::new();
    while let Some(chunk)=response.chunk().await.map_err(|_|"Interrupted mod download")? {
        if data.len() as u64 + chunk.len() as u64 > limit { return Err("Download exceeds size limit".into()); }
        data.extend_from_slice(&chunk);
    }
    Ok(data)
}
const WINDOWS_DOWNLOAD:&str=r#"
$ErrorActionPreference='Stop'
Add-Type -AssemblyName System.Net.Http
$handler=[Net.Http.HttpClientHandler]::new()
$handler.AllowAutoRedirect=$false
$handler.UseDefaultCredentials=$false
$client=[Net.Http.HttpClient]::new($handler)
$client.Timeout=[TimeSpan]::FromSeconds(40)
$client.DefaultRequestHeaders.UserAgent.ParseAdd('FRE-LAUNCH/native-mod-checks')
$uri=[Uri]$env:FRE_MOD_DOWNLOAD_URL
$limit=[Int64]$env:FRE_MOD_DOWNLOAD_LIMIT
try {
 for ($hop=0; $hop -lt 6; $hop++) {
  $allowed=@('github.com','horizon-rust.github.io','release-assets.githubusercontent.com','objects.githubusercontent.com','github-releases.githubusercontent.com')
  if ($uri.Scheme -ne 'https' -or $uri.Port -ne 443 -or $uri.UserInfo -or $uri.Fragment -or $allowed -notcontains $uri.DnsSafeHost) { throw 'Blocked download redirect' }
  $response=$client.GetAsync($uri,[Net.Http.HttpCompletionOption]::ResponseHeadersRead).GetAwaiter().GetResult()
  $code=[int]$response.StatusCode
  if ($code -ge 300 -and $code -lt 400 -and $response.Headers.Location) {
   $next=$response.Headers.Location
   if (!$next.IsAbsoluteUri) { $next=[Uri]::new($uri,$next) }
   $response.Dispose()
   $uri=$next
   continue
  }
  if (!$response.IsSuccessStatusCode) { throw "Download HTTP $code" }
  if ($response.Content.Headers.ContentLength -gt $limit) { throw 'Oversized mod download' }
  $stream=$response.Content.ReadAsStreamAsync().GetAwaiter().GetResult()
  if ($stream.CanTimeout) { $stream.ReadTimeout=15000 }
  $file=[IO.File]::Open($env:FRE_MOD_DOWNLOAD_FILE,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
  try {
   $buffer=New-Object byte[] 65536
   $total=[Int64]0
   while (($count=$stream.Read($buffer,0,$buffer.Length)) -gt 0) {
    $total+=$count
    if ($total -gt $limit) { throw 'Oversized mod download' }
    $file.Write($buffer,0,$count)
   }
  } finally { $file.Dispose(); $stream.Dispose(); $response.Dispose() }
  Write-Output 'FRE_MOD_DOWNLOAD_COMPLETE'
  exit 0
 }
 throw 'Too many mod download redirects'
} finally { $client.Dispose(); $handler.Dispose() }
"#;
async fn fetch(url:&str,limit:u64)->Result<Vec<u8>,String> {
    match fetch_primary(url,limit).await {
        Ok(bytes)=>Ok(bytes),
        Err(original)=>{
            #[cfg(windows)] {
                // Windows' system HTTP stack is a fallback for GitHub/CDN transport failures.
                // The same host, byte, redirect and cryptographic checks still apply.
                let parsed=Url::parse(url).map_err(|_|original.clone())?;
                if !allowed_host(&parsed) { return Err(original); }
                use std::os::windows::process::CommandExt;
                let directory=tempfile::tempdir().map_err(|e|e.to_string())?;
                let target=directory.path().join("release.bin");
                let shell=PathBuf::from(std::env::var_os("SystemRoot").ok_or("Missing Windows system directory")?).join("System32/WindowsPowerShell/v1.0/powershell.exe");
                let mut command=std::process::Command::new(shell);
                command.args(["-NoProfile","-NonInteractive","-Command",WINDOWS_DOWNLOAD])
                    .env("FRE_MOD_DOWNLOAD_URL",url).env("FRE_MOD_DOWNLOAD_LIMIT",limit.to_string()).env("FRE_MOD_DOWNLOAD_FILE",&target).creation_flags(0x08000000);
                let mut command=tokio::process::Command::from(command);
                command.kill_on_drop(true);
                let output=tokio::time::timeout(Duration::from_secs(60),command.output()).await.map_err(|_|original.clone())?.map_err(|_|original.clone())?;
                if !output.status.success() || !String::from_utf8_lossy(&output.stdout).lines().any(|line|line.trim()=="FRE_MOD_DOWNLOAD_COMPLETE") { return Err(original); }
                read_bounded(&target,limit)
            }
            #[cfg(not(windows))] { let _=WINDOWS_DOWNLOAD; Err(original) }
        }
    }
}

fn parse_index(data: &[u8]) -> Result<Index,String> {
    if data.len() as u64>MAX_INDEX { return Err("Catalog exceeds size limit".into()); }
    let index:Index=serde_json::from_slice(data).map_err(|_|"Cannot read the mod catalog")?;
    if index.schema!=1 || index.projects.len()>5000 || index.revoked.len()>10000 { return Err("Unsupported catalog format".into()); }
    let mut seen=std::collections::BTreeSet::new();
    for project in &index.projects {
        if !mod_audit::valid_id(&project.id) || !seen.insert(&project.id) || project.versions.len()>250 || project.description.len()>65536
            || project.name.len()>256 || project.summary.len()>1024 || project.authors.len()>100 || project.license.len()>256 {
            return Err("Malformed catalog project".into());
        }
    }
    Ok(index)
}
fn directory(parent: &Path, name: &str) -> Result<PathBuf,String> {
    if !parent.symlink_metadata().is_ok_and(|m|m.file_type().is_dir()) { return Err("Unsafe catalog parent directory".into()); }
    let dir=parent.join(name);
    if !dir.exists() { fs::create_dir(&dir).map_err(|e|e.to_string())?; }
    if !dir.symlink_metadata().is_ok_and(|m|m.file_type().is_dir()) { return Err("Unsafe catalog directory".into()); }
    Ok(dir)
}
fn regular(path: &Path, max:u64) -> Result<(),String> {
    if !path.symlink_metadata().is_ok_and(|m|m.file_type().is_file() && m.len()<=max) { return Err("Unsafe or oversized catalog file".into()); }
    Ok(())
}
fn read_bounded(path:&Path,max:u64)->Result<Vec<u8>,String> {
    use std::io::Read;
    regular(path,max)?;
    let mut bytes=Vec::new();
    fs::File::open(path).and_then(|file|file.take(max+1).read_to_end(&mut bytes)).map_err(|e|e.to_string())?;
    if bytes.len() as u64>max { return Err("Catalog/mod file grew beyond its limit".into()); }
    Ok(bytes)
}
async fn index(root: &Path, force: bool) -> Result<(Index,bool,Option<String>),String> {
    let mut loaded=CACHE.lock().await;
    if !force {
        if let Some((at,value))=loaded.as_ref() {
            if now().saturating_sub(*at)<600 { return Ok((value.clone(),false,None)); }
        }
    }
    let dir=directory(root,"mod-catalog")?;
    let cache=dir.join("catalog.json");
    match fetch(INDEX_URL,MAX_INDEX).await.and_then(|bytes|parse_index(&bytes).map(|parsed|(parsed,bytes))) {
        Ok((value,bytes))=>{
            if cache.exists() { regular(&cache,MAX_INDEX)?; }
            fs::write(&cache,bytes).map_err(|e|e.to_string())?;
            *loaded=Some((now(),value.clone()));
            Ok((value,false,None))
        },
        Err(error) if !force=>{
            regular(&cache,MAX_INDEX)?;
            let value=parse_index(&read_bounded(&cache,MAX_INDEX)?)?;
            Ok((value,true,Some(error)))
        },
        Err(error)=>Err(error),
    }
}
fn unsupported_version(kind:&str,version:&Version)->Option<String> {
    if version.game!="26.3" { return Some("This release requires another Minecraft version".into()); }
    if kind=="native-mod" {
        if !version.api.starts_with("native-1/") { return Some("This release requires a newer native mod format".into()); }
        if version.hooks.is_some_and(|hooks|hooks>NATIVE_HOOKS) {
            return Some(format!("This release needs native hook API {}, but FRE supports up to {}. Choose a compatible release.",version.hooks.unwrap(),NATIVE_HOOKS));
        }
    }
    None
}
fn recommended_version<'a>(index:&Index,project:&'a Project)->Option<&'a Version> {
    project.versions.iter().find(|version|unsupported_version(&project.kind,version).is_none() && revoked(index,&project.id,&version.file.sha256).is_none())
}
pub async fn view(root:&Path, force:bool) -> Result<View,String> {
    let (mut value,stale,warning)=index(root,force).await?;
    let snapshot=value.clone();
    for project in &mut value.projects {
        for version in &mut project.versions {
            version.unsupported_reason=unsupported_version(&project.kind,version).or_else(||revoked(&snapshot,&project.id,&version.file.sha256));
            version.supported=version.unsupported_reason.is_none();
        }
    }
    value.projects.sort_by(|a,b|b.downloads.cmp(&a.downloads).then_with(||a.name.cmp(&b.name)));
    Ok(View {projects:value.projects,generated:value.generated,fetched_at:now(),stale,warning})
}
fn revoked(index:&Index, project:&str, hash:&str) -> Option<String> {
    index.revoked.iter().find(|entry| entry.id==project && entry.sha256.as_deref().is_none_or(|expected|expected==hash))
        .map(|entry|entry.reason.clone())
}
fn verify_signature(project:&Project, version:&Version, bytes:&[u8]) -> Result<bool,String> {
    let pins:BTreeMap<String,String>=serde_json::from_str(include_str!("fre_rust_catalog_keys.json")).map_err(|_|"Invalid FRE publisher key pins")?;
    match (&project.key,pins.get(&project.id)) {
        (None,None)=>Ok(false),
        (Some(key),Some(pin)) if key==pin=>{
            let signature=version.file.signature.as_deref().ok_or("The publisher requires a signature")?;
            let key=minisign_verify::PublicKey::from_base64(key).map_err(|_|"Invalid publisher key")?;
            let signature=minisign_verify::Signature::decode(signature).map_err(|_|"Invalid publisher signature")?;
            key.verify(bytes,&signature,false).map_err(|_|"Publisher signature does not match this file")?;
            Ok(true)
        },
        _=>Err("Publisher key is new, missing or changed; an updated FRE key review is required".into()),
    }
}
const SCAN_SCRIPT:&str=r#"
$ErrorActionPreference='Stop'
$status=Get-MpComputerStatus -ErrorAction Stop
if (!$status.AMServiceEnabled -or !$status.AntivirusEnabled) { throw 'Defender unavailable' }
$scanner=Get-ChildItem -LiteralPath (Join-Path $env:ProgramData 'Microsoft/Windows Defender/Platform') -Directory | Sort-Object Name -Descending | ForEach-Object { Join-Path $_.FullName 'MpCmdRun.exe' } | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
if (!$scanner) { $scanner=Join-Path $env:ProgramFiles 'Windows Defender/MpCmdRun.exe' }
$path=$env:FRE_MOD_SCAN_FILE
& $scanner -Scan -ScanType 3 -File $path -DisableRemediation
$scanExit=$LASTEXITCODE
$threats=@(Get-MpThreatDetection -ErrorAction Stop | Where-Object { @($_.Resources | Where-Object { ([string]$_).IndexOf($path,[StringComparison]::OrdinalIgnoreCase) -ge 0 }).Count -gt 0 })
if ($threats.Count -gt 0) { Write-Output 'FRE_MOD_SCAN_THREAT'; exit 3 }
if ($scanExit -ne 0) { throw 'Mod scan did not complete successfully' }
Write-Output 'FRE_MOD_SCAN_CLEAN'
"#;
async fn defender(path:&Path) -> Result<(),String> {
    #[cfg(windows)] {
        use std::os::windows::process::CommandExt;
        let shell=PathBuf::from(std::env::var_os("SystemRoot").ok_or("Missing Windows system directory")?).join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let mut command=std::process::Command::new(shell);
        command.args(["-NoProfile","-NonInteractive","-Command",SCAN_SCRIPT]).env("FRE_MOD_SCAN_FILE",path).creation_flags(0x08000000);
        let mut command=tokio::process::Command::from(command);
        command.kill_on_drop(true);
        let output=tokio::time::timeout(Duration::from_secs(60),command.output()).await
            .map_err(|_|"Defender scan timed out")?.map_err(|_|"Cannot start Defender scan")?;
        let text=String::from_utf8_lossy(&output.stdout);
        if text.lines().any(|line|line.trim()=="FRE_MOD_SCAN_THREAT") { return Err("DETECTED: Defender reported a threat in this mod file".into()); }
        if !output.status.success() || !text.lines().any(|line|line.trim()=="FRE_MOD_SCAN_CLEAN") {
            return Err("Defender did not confirm a completed clean scan".into());
        }
        Ok(())
    }
    #[cfg(not(windows))] { let _=(path,SCAN_SCRIPT); Err("Defender scan is not available on this system".into()) }
}
fn select<'a>(index:&'a Index,id:&str) -> Result<(&'a Project,&'a Version),String> {
    let project=index.projects.iter().find(|p|p.id==id).ok_or("Mod is not in the current catalog")?;
    let version=recommended_version(index,project).ok_or("No compatible, non-revoked Minecraft 26.3 release is available for this FRE engine")?;
    Ok((project,version))
}
async fn inspect_download(root:&Path,id:&str,expected_hash:&str) -> Result<(Report,PathBuf),String> {
    let (index,_,_)=index(root,true).await?;
    let (project,version)=select(&index,id)?;
    if version.file.sha256!=expected_hash { return Err("The catalog version changed. Refresh the list before checking or installing it.".into()); }
    let mut report=Report {project_id:id.into(),version:version.version.clone(),filename:version.file.name.clone(),sha256:version.file.sha256.clone(),checked_at:now(),installable:false,checks:Vec::new(),facts:None};
    validate_file(project,version)?;
    report.checks.push(check("origin","pass","Public GitHub release URL matches the project/registry"));
    if let Some(reason)=revoked(&index,id,&version.file.sha256) { report.checks.push(check("revocation","fail",reason)); return Ok((report,PathBuf::new())); }
    report.checks.push(check("revocation","pass","Not revoked in the freshly fetched catalog"));
    let bytes=fetch(&version.file.url,MAX_MOD).await?;
    if bytes.len() as u64!=version.file.size || digest(&bytes)!=version.file.sha256 {
        report.checks.push(check("hash","fail","Downloaded size or SHA-256 differs from the catalog")); return Ok((report,PathBuf::new()));
    }
    report.checks.push(check("hash","pass",version.file.sha256.clone()));
    match verify_signature(project,version,&bytes) {
        Ok(true)=>report.checks.push(check("signature","pass","Signature matches the pinned publisher key")),
        Ok(false)=>report.checks.push(check("signature","warning","The publisher did not sign this release")),
        Err(error)=>{ report.checks.push(check("signature","fail",error)); return Ok((report,PathBuf::new())); },
    }
    let facts=match mod_audit::inspect(&bytes,&project.kind,Some((&project.id,&version.version,&version.api))) {
        Ok(facts)=>facts,
        Err(error)=>{report.checks.push(check("archive","fail",error)); return Ok((report,PathBuf::new()));},
    };
    report.checks.push(check("archive","pass",format!("{} bounded entries, {} expanded bytes",facts.files,facts.expanded_bytes)));
    if project.kind=="native-mod" {
        let compatible=facts.hooks.map_or(facts.engine_revision.as_deref()==Some("975c07d9151847e6af13afc4b5ab37b6bbe78ef5"),|hooks|u64::from(hooks)<=NATIVE_HOOKS);
        report.checks.push(check("compatibility",if compatible {"pass"}else{"warning"},if compatible {"Declared hooks/source match the bundled engine; FRE builds it locally"}else{"This native mod targets another engine revision or newer hook API"}));
    } else {
        report.checks.push(check("compatibility",if facts.content_only {"pass"}else{"warning"},if facts.content_only {"Data format 0.2; companion native mods may still be required (see description)"}else{"Legacy executable .hmod code is not supported"}));
    }

    let dir=directory(root,"mod-catalog-downloads")?;
    let path=dir.join(format!("{}.{}",version.file.sha256,if project.kind=="native-mod" {"hnmod"}else{"hmod"}));
    if path.exists() { regular(&path,MAX_MOD)?; }
    fs::write(&path,&bytes).map_err(|e|e.to_string())?;
    match defender(&path).await {
        Ok(())=>report.checks.push(check("antivirus","pass","Defender completed a custom scan without a detection")),
        Err(error)=>report.checks.push(check("antivirus",if error.starts_with("DETECTED:") {"fail"} else {"unavailable"},error)),
    }
    if digest(&read_bounded(&path,MAX_MOD)?)!=version.file.sha256 { return Err("Mod file changed during inspection".into()); }
    if facts.content_only {
        report.checks.push(check("execution","pass","No native/WASM code; Minecraft data functions may affect the world (see description)"));
        report.installable=report.checks.iter().all(|c|c.state=="pass" || (c.code=="signature" && c.state=="warning"));
    } else {
        report.checks.push(check("execution","warning","Native code runs with your account permissions. Archive, signature and antivirus checks cannot guarantee that code is safe."));
        if project.kind=="native-mod" {
            match native_source_policy(&bytes) {
                Ok(())=>report.installable=report.checks.iter().all(|c|c.state=="pass" || (matches!(c.code,"signature"|"execution") && c.state=="warning")),
                Err(error)=>report.checks.push(check("compatibility","fail",error)),
            }
        }
    }
    report.facts=Some(facts);
    Ok((report,path))
}
pub async fn inspect_project(root:&Path,id:&str,expected_hash:&str) -> Result<Report,String> {
    let _guard=OPERATIONS.lock().await;
    inspect_download(root,id,expected_hash).await.map(|(report,_)|report)
}
#[derive(Serialize,Deserialize)]
struct Receipt { project:String, version:String, filename:String, sha256:String }
pub async fn install(root:&Path,profile:&Path,id:&str,expected_hash:&str) -> Result<Report,String> {
    let _guard=OPERATIONS.lock().await;
    let (report,path)=inspect_download(root,id,expected_hash).await?;
    if !report.installable { return Err("This file did not pass all installation checks or has an unsupported native format. Open the verification report.".into()); }
    let mods=directory(profile,"fre-mods")?;
    for entry in fs::read_dir(&mods).map_err(|e|e.to_string())? {
        let entry=entry.map_err(|e|e.to_string())?;
        if entry.path().extension().is_some_and(|ext|ext.eq_ignore_ascii_case("hmod") || ext.eq_ignore_ascii_case("hnmod")) {
            let old=read_bounded(&entry.path(),MAX_MOD)?;
            let kind=if entry.path().extension().is_some_and(|ext|ext.eq_ignore_ascii_case("hnmod")) {"native-mod"}else{"mod"};
            if mod_audit::inspect(&old,kind,None).is_ok_and(|facts|facts.id==id) {
                return Err("A version of this mod is already installed. Remove its file explicitly before replacing it.".into());
            }
        }
    }
    let target=mods.join(&report.filename);
    if target.symlink_metadata().is_ok() { return Err("A mod with this filename already exists; keep it until it is explicitly removed".into()); }
    let bytes=read_bounded(&path,MAX_MOD)?;
    if digest(&bytes)!=report.sha256 { return Err("Checked mod file changed before installation".into()); }
    use std::io::Write;
    let mut staged=tempfile::NamedTempFile::new_in(&mods).map_err(|e|e.to_string())?;
    staged.write_all(&bytes).and_then(|()|staged.as_file().sync_all()).map_err(|e|e.to_string())?;
    let receipts=directory(profile,".fre-mod-receipts")?;
    let receipt=Receipt {project:id.into(),version:report.version.clone(),filename:report.filename.clone(),sha256:report.sha256.clone()};
    let record=receipts.join(format!("{id}.json"));
    if record.exists() { regular(&record,16384)?; }
    let mut staged_record=tempfile::NamedTempFile::new_in(&receipts).map_err(|e|e.to_string())?;
    staged_record.write_all(&serde_json::to_vec(&receipt).map_err(|e|e.to_string())?)
        .and_then(|()|staged_record.as_file().sync_all()).map_err(|e|e.to_string())?;
    staged_record.persist(&record).map_err(|e|e.to_string())?;
    set_content_enabled(profile,id,true)?;
    staged.persist_noclobber(&target).map_err(|e|e.to_string())?;
    Ok(report)
}
pub fn forget_removed_file(profile:&Path,filename:&str)->Result<(),String> {
    if !profile.join(".fre-mod-receipts").exists() { return Ok(()); }
    let receipts=directory(profile,".fre-mod-receipts")?;
    for entry in fs::read_dir(&receipts).map_err(|e|e.to_string())? {
        let entry=entry.map_err(|e|e.to_string())?;
        if entry.path().extension().is_none_or(|ext|ext!="json") { continue; }
        let bytes=read_bounded(&entry.path(),16384)?;
        if let Ok(record)=serde_json::from_slice::<Receipt>(&bytes) {
            if mod_audit::valid_id(&record.project) && entry.file_name()==format!("{}.json",record.project).as_str()
                && record.filename.eq_ignore_ascii_case(filename) {
                fs::remove_file(entry.path()).map_err(|e|e.to_string())?;
            }
        }
    }
    Ok(())
}
pub fn local_facts(path:&Path)->Result<mod_audit::PackageFacts,String> {
    mod_audit::inspect(&read_bounded(path,MAX_MOD)?,if path.extension().is_some_and(|ext|ext.eq_ignore_ascii_case("hnmod")){"native-mod"}else{"mod"},None)
}
fn mod_settings(profile:&Path)->Result<String,String> {
    let path=profile.join("fre-mods.txt");
    if path.symlink_metadata().is_err_and(|error|error.kind()==std::io::ErrorKind::NotFound) { return Ok(String::new()); }
    String::from_utf8(read_bounded(&path,256<<10)?).map_err(|_|"Mod settings are not UTF-8; preserve and repair this file before changing mods".into())
}
pub fn enabled_content(profile:&Path)->Result<std::collections::BTreeSet<String>,String> {
    Ok(mod_settings(profile)?.lines().filter_map(|line|line.split_once(':'))
        .filter(|(_,value)|value.trim()=="true").filter_map(|(key,_)|key.trim().strip_prefix("mod."))
        .map(str::to_owned).collect())
}
pub fn set_content_enabled(profile:&Path,id:&str,enabled:bool)->Result<(),String> {
    if !mod_audit::valid_id(id) { return Err("Invalid content mod ID".into()); }
    let text=mod_settings(profile)?;
    let key=format!("mod.{id}");
    let mut updated:String=text.split_inclusive('\n').filter(|line|line.split_once(':').is_none_or(|(name,_)|name.trim()!=key)).collect();
    if !updated.is_empty() && !updated.ends_with('\n') { updated.push('\n'); }
    updated.push_str(&format!("{key}:{enabled}\n"));
    if updated.len()>256<<10 { return Err("Mod settings exceed their size limit".into()); }
    use std::io::Write;
    let mut staged=tempfile::NamedTempFile::new_in(profile).map_err(|e|e.to_string())?;
    staged.write_all(updated.as_bytes()).and_then(|()|staged.as_file().sync_all()).map_err(|e|e.to_string())?;
    staged.persist(profile.join("fre-mods.txt")).map_err(|e|e.to_string())?;
    Ok(())
}
pub async fn set_installed_enabled(root:&Path,profile:&Path,filename:&str,enabled:bool)->Result<mod_audit::PackageFacts,String> {
    if !file_name(filename) || !(filename.to_ascii_lowercase().ends_with(".hmod") || filename.to_ascii_lowercase().ends_with(".hnmod")) { return Err("Choose a .hmod content file".into()); }
    let facts=local_facts(&directory(profile,"fre-mods")?.join(filename))?;
    if enabled {
        if !facts.content_only && !filename.ends_with(".hnmod") { return Err("Legacy executable mod code cannot be enabled here".into()); }
        before_play(root,profile).await?;
    }
    set_content_enabled(profile,&facts.id,enabled)?;
    Ok(facts)
}
pub async fn inspect_local(root:&Path,path:&Path) -> Result<mod_audit::PackageFacts,String> {
    regular(path,MAX_MOD)?;
    let bytes=read_bounded(path,MAX_MOD)?;
    let facts=mod_audit::inspect(&bytes,"mod",None)?;
    if !facts.content_only { return Err("This .hmod contains legacy executable code. Only checked data-only content packages can be imported.".into()); }
    let (index,_,_)=index(root,true).await?;
    if let Some(reason)=revoked(&index,&facts.id,&digest(&bytes)) { return Err(format!("Mod was revoked: {reason}")); }
    defender(path).await?;
    if digest(&read_bounded(path,MAX_MOD)?)!=digest(&bytes) { return Err("Local mod changed during its scan".into()); }
    Ok(facts)
}
pub async fn before_play(root:&Path,profile:&Path) -> Result<(),String> {
    let mods=directory(profile,"fre-mods")?;
    let paths:Vec<_>=fs::read_dir(&mods).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    let mods:Vec<_>=paths.into_iter().filter(|e|e.path().extension().is_some_and(|e|e.eq_ignore_ascii_case("hmod"))).collect();
    if mods.is_empty() { return Ok(()); }
    if mods.len()>500 { return Err("Too many installed mods to verify".into()); }
    let (index,_,_)=index(root,true).await?;
    for entry in mods {
        let path=entry.path();
        regular(&path,MAX_MOD)?;
        let bytes=read_bounded(&path,MAX_MOD)?;
        let hash=digest(&bytes);
        let facts=mod_audit::inspect(&bytes,"mod",None).map_err(|e|format!("{}: {e}",entry.file_name().to_string_lossy()))?;
        if !facts.content_only { return Err(format!("{} contains unsupported executable mod code",facts.id)); }
        if let Some(reason)=revoked(&index,&facts.id,&hash) { return Err(format!("{} was revoked: {reason}",facts.id)); }
        let receipt=profile.join(".fre-mod-receipts").join(format!("{}.json",facts.id));
        if receipt.symlink_metadata().is_ok() {
            regular(&receipt,16384)?;
            let record:Receipt=serde_json::from_slice(&read_bounded(&receipt,16384)?).map_err(|_|"Invalid catalog mod receipt")?;
            let project=index.projects.iter().find(|p|p.id==record.project).ok_or("Installed mod disappeared from the current catalog")?;
            let version=project.versions.iter().find(|v|v.version==record.version && v.file.sha256==record.sha256).ok_or("Installed release is no longer in the catalog")?;
            validate_file(project,version)?;
            if hash!=record.sha256 || facts.id!=record.project || facts.version!=record.version || path.file_name().and_then(|s|s.to_str())!=Some(record.filename.as_str()) { return Err("Installed mod differs from its checked catalog receipt".into()); }
            verify_signature(project,version,&bytes)?;
        }
        defender(&path).await?;
        if digest(&read_bounded(&path,MAX_MOD)?)!=hash { return Err("Installed mod changed during the scan".into()); }
    }
    Ok(())
}

/// Restrict code placement to the game's source. In particular, a manifest
/// cannot inject modules into a Cargo build script or a compiler/tool crate.
fn native_game_targets(value:&toml::Value)->Result<(),String> {
    match value {
        toml::Value::Table(table)=>for (key,value) in table {
            if key=="target" {
                let target=value.as_str().ok_or("A native target must name a game function")?;
                if !target.starts_with("horizon_client::game::") && !target.starts_with("fre_client::game::") {
                    return Err("FRE currently supports game hooks only; compiler, build-script and other crate targets are not enabled".into());
                }
            }
            if key=="file" {
                let file=value.as_str().ok_or("A native module must name a game source file")?;
                let game=file.strip_prefix("crates/horizon-client/src/").or_else(||file.strip_prefix("crates/fre-client/src/"));
                if !game.is_some_and(|path|path=="game.rs" || path.starts_with("game/")) || file.contains("..") || file.contains('\\') {
                    return Err("Native modules may only extend game source files, never build scripts or compiler configuration".into());
                }
            }
            native_game_targets(value)?;
        },
        toml::Value::Array(values)=>for value in values {native_game_targets(value)?;},
        _=>{},
    }Ok(())
}
/// Only source mixins with the engine's locked dependencies are compiled.
/// Build programs and extra native crates need a separate reviewed integration.
fn native_source_policy(bytes:&[u8])->Result<(),String> {
    use std::io::Read;
    let mut archive=zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e|e.to_string())?;
    for i in 0..archive.len() {
        let mut entry=archive.by_index(i).map_err(|e|e.to_string())?;
        let name=entry.name().to_owned();
        if name.contains("native-crates/") || name.ends_with("build.rs") {return Err("This mod supplies extra native crates or build programs; FRE supports source mixins with the locked engine dependencies".into());}
        if name.ends_with("mixin.toml") {
            let mut text=String::new();entry.read_to_string(&mut text).map_err(|e|e.to_string())?;
            let value:toml::Value=toml::from_str(&text).map_err(|e|e.to_string())?;
            native_game_targets(&value)?;
            if value.get("crate").is_some() || value.get("post_build").is_some(){return Err("This mod needs extra crates or post-build programs that FRE has not enabled".into());}
        }
    }Ok(())
}
pub async fn scan_native_executable(path:&Path)->Result<(),String> { defender(path).await }
pub async fn verified_native(root:&Path,profile:&Path,enabled:&std::collections::BTreeSet<String>)->Result<Vec<(String,Vec<u8>)>,String> {
    let folder=directory(profile,"fre-mods")?;
    let mut archives=Vec::new();
    for entry in fs::read_dir(&folder).map_err(|e|e.to_string())? {
        let entry=entry.map_err(|e|e.to_string())?;
        if !entry.path().extension().is_some_and(|ext|ext.eq_ignore_ascii_case("hnmod")){continue;}
        let bytes=read_bounded(&entry.path(),MAX_MOD)?;
        let facts=mod_audit::inspect(&bytes,"native-mod",None)?;
        if enabled.contains(&facts.id){archives.push((entry.path(),bytes,facts));}
        if archives.len()>100{return Err("Too many active native mods".into());}
    }
    if archives.is_empty(){return Ok(Vec::new());}
    let (index,_,_)=index(root,true).await?;let mut result=Vec::new();
    for (path,bytes,facts) in archives {
        native_source_policy(&bytes)?;
        let record:Receipt=serde_json::from_slice(&read_bounded(&profile.join(".fre-mod-receipts").join(format!("{}.json",facts.id)),16384)?).map_err(|_|"Install native mods through the checked FRE catalog")?;
        let project=index.projects.iter().find(|p|p.id==record.project).ok_or("Native mod is no longer in the catalog")?;
        let version=project.versions.iter().find(|v|v.version==record.version && v.file.sha256==record.sha256).ok_or("Native release is no longer in the catalog")?;
        validate_file(project,version)?;
        if project.kind!="native-mod" || facts.id!=record.project || facts.version!=record.version || digest(&bytes)!=record.sha256 || path.file_name().and_then(|n|n.to_str())!=Some(record.filename.as_str()){return Err("Native mod differs from its checked receipt".into());}
        if let Some(reason)=revoked(&index,&facts.id,&record.sha256){return Err(format!("Native mod revoked: {reason}"));}
        verify_signature(project,version,&bytes)?;defender(&path).await?;
        if digest(&read_bounded(&path,MAX_MOD)?)!=record.sha256{return Err("Native mod changed during scan".into());}
        result.push((facts.id,bytes));
    }
    result.sort_by(|a,b|a.0.cmp(&b.0));Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_manifests_cannot_place_code_in_build_scripts_or_compiler_targets() {
        let game:toml::Value=toml::from_str("[[injection]]\ntarget=\"horizon_client::game::native_hooks::Game<'_>::native_frame\"\n[[module]]\nfile=\"crates/horizon-client/src/game.rs\"\n").unwrap();
        assert!(native_game_targets(&game).is_ok());
        for source in ["[[module]]\nfile='crates/horizon-client/build.rs'", "[[module]]\nfile='crates/horizon-client/src/game/../../build.rs'", "[[injection]]\ntarget='build_script_build::main'", "[[module]]\nfile='.cargo/config.toml'"] {
            let value:toml::Value=toml::from_str(source).unwrap();assert!(native_game_targets(&value).is_err(),"{source}");
        }
    }
    fn project() -> Project {
        serde_json::from_value(serde_json::json!({"id":"demo","kind":"mod","name":"Demo","license":"MIT","source":"https://github.com/owner/demo","versions":[{"version":"1","api":"0.2","game":"26.3","file":{"name":"demo-1.hmod","url":"https://github.com/owner/demo/releases/download/v1/demo-1.hmod","size":10,"sha256":"1".repeat(64)}}]})).unwrap()
    }
    #[test]
    fn accepts_only_the_projects_or_registry_release_assets() {
        let mut p=project();
        assert!(validate_file(&p,&p.versions[0]).is_ok());
        for url in ["http://github.com/owner/demo/releases/download/v1/demo-1.hmod","https://github.com.evil/owner/demo/releases/download/v1/demo-1.hmod","https://github.com/other/demo/releases/download/v1/demo-1.hmod","https://github.com/owner/demo/raw/main/demo-1.hmod","https://user@github.com/owner/demo/releases/download/v1/demo-1.hmod","https://127.0.0.1/demo-1.hmod"] {
            p.versions[0].file.url=url.into();
            assert!(validate_file(&p,&p.versions[0]).is_err(),"{url}");
        }
    }
    #[test]
    fn revocations_cover_whole_projects_and_exact_files() {
        let index=Index{schema:1,generated:String::new(),projects:vec![],revoked:vec![Revoked{id:"demo".into(),sha256:Some("1".repeat(64)),reason:"removed".into()}]};
        assert!(revoked(&index,"demo",&"1".repeat(64)).is_some());
        assert!(revoked(&index,"demo",&"2".repeat(64)).is_none());
    }
    #[test]
    fn redirects_cannot_reach_local_or_foreign_hosts() {
        for url in ["https://localhost/file","https://github.com.evil/file","http://release-assets.githubusercontent.com/file","file:///C:/private"] {
            assert!(!allowed_host(&Url::parse(url).unwrap()));
        }
    }
    #[test]
    fn removing_a_mod_forgets_only_its_receipt_and_preserves_other_files() {
        let profile=tempfile::tempdir().unwrap();
        let receipts=directory(profile.path(),".fre-mod-receipts").unwrap();
        for id in ["demo","other"] {
            let record=Receipt{project:id.into(),version:"1".into(),filename:format!("{id}.hmod"),sha256:"1".repeat(64)};
            fs::write(receipts.join(format!("{id}.json")),serde_json::to_vec(&record).unwrap()).unwrap();
        }
        fs::write(profile.path().join("world.dat"),b"preserve world").unwrap();
        forget_removed_file(profile.path(),"DEMO.hmod").unwrap();
        assert!(!receipts.join("demo.json").exists());
        assert!(receipts.join("other.json").exists());
        assert_eq!(fs::read(profile.path().join("world.dat")).unwrap(),b"preserve world");
    }
    #[test]
    fn content_toggles_match_the_game_and_preserve_keys_comments_and_worlds() {
        let profile=tempfile::tempdir().unwrap();
        let path=profile.path().join("fre-mods.txt");
        fs::write(&path,"# player settings\r\nkey.zoom.key:key.keyboard.c\r\nmod.demo:false\nmod.other:true\nmod.demo:true\nunknown:keep").unwrap();
        fs::write(profile.path().join("world.dat"),b"world").unwrap();
        set_content_enabled(profile.path(),"demo",false).unwrap();
        assert!(!enabled_content(profile.path()).unwrap().contains("demo"));
        assert!(enabled_content(profile.path()).unwrap().contains("other"));
        let changed=fs::read_to_string(&path).unwrap();
        assert!(changed.starts_with("# player settings\r\nkey.zoom.key:key.keyboard.c\r\n"));
        assert!(changed.contains("unknown:keep\n"));
        assert_eq!(changed.matches("mod.demo:").count(),1);
        set_content_enabled(profile.path(),"demo",true).unwrap();
        assert!(enabled_content(profile.path()).unwrap().contains("demo"));
        assert_eq!(fs::read(profile.path().join("world.dat")).unwrap(),b"world");
        let other=tempfile::tempdir().unwrap();
        assert!(enabled_content(other.path()).unwrap().is_empty());
    }
    #[test]
    fn invalid_or_oversized_game_mod_settings_are_preserved() {
        let profile=tempfile::tempdir().unwrap();
        let path=profile.path().join("fre-mods.txt");
        fs::write(&path,[255u8]).unwrap();
        assert!(set_content_enabled(profile.path(),"demo",true).is_err());
        assert_eq!(fs::read(&path).unwrap(),[255]);
        fs::write(&path,vec![b'x';(256<<10)+1]).unwrap();
        assert!(set_content_enabled(profile.path(),"demo",true).is_err());
        assert_eq!(fs::metadata(&path).unwrap().len(),(256<<10)+1);
        assert!(set_content_enabled(profile.path(),"../demo",true).is_err());
    }
    #[tokio::test]
    async fn enabling_rejects_foreign_paths_and_legacy_executable_mods_before_network_access() {
        use std::io::{Cursor,Write};
        let profile=tempfile::tempdir().unwrap();
        let mods=directory(profile.path(),"fre-mods").unwrap();
        let mut archive=zip::ZipWriter::new(Cursor::new(Vec::new()));
        archive.start_file("fre-mod.toml",zip::write::SimpleFileOptions::default()).unwrap();
        archive.write_all(b"id='demo'\nname='Demo'\nversion='1'\napi='0.1'\n").unwrap();
        archive.start_file("mod.wasm",zip::write::SimpleFileOptions::default()).unwrap();
        archive.write_all(b"\0asm").unwrap();
        fs::write(mods.join("demo.hmod"),archive.finish().unwrap().into_inner()).unwrap();
        assert!(set_installed_enabled(profile.path(),profile.path(),"../demo.hmod",true).await.is_err());
        assert!(set_installed_enabled(profile.path(),profile.path(),"demo.hmod",true).await.is_err());
        assert!(enabled_content(profile.path()).unwrap().is_empty());
    }
    #[test]
    fn unsigned_and_changed_keys_are_distinguished() {
        let mut p=project();
        assert_eq!(verify_signature(&p,&p.versions[0],b"demo"),Ok(false));
        p.key=Some("not-pinned".into());
        assert!(verify_signature(&p,&p.versions[0],b"demo").is_err());
    }
}

#[cfg(test)]
mod release_selection_tests {
    use super::*;
    fn registry()->Index {
        serde_json::from_value(serde_json::json!({"schema":1,"projects":[{
            "id":"appleskin","kind":"native-mod","name":"AppleSkin","license":"MIT","source":"https://github.com/example/appleskin",
            "versions":[
                {"version":"0.1.3","game":"26.3","api":"native-1/new/new","hooks":139,"supported":true,"file":{"name":"appleskin-0.1.3.hnmod","url":"https://github.com/example/appleskin/releases/download/v3/appleskin.hnmod","size":1,"sha256":"a".repeat(64)}},
                {"version":"0.1.2","game":"26.3","api":"native-1/old/old","hooks":1,"file":{"name":"appleskin-0.1.2.hnmod","url":"https://github.com/example/appleskin/releases/download/v2/appleskin.hnmod","size":1,"sha256":"b".repeat(64)}}
            ]
        }]})).unwrap()
    }
    #[test] fn appleskin_selects_the_compatible_signed_release_instead_of_newer_hooks() {
        let index=registry();let project=&index.projects[0];
        assert!(!project.versions[0].supported,"registry cannot set the computed compatibility flag");
        assert!(unsupported_version(&project.kind,&project.versions[0]).unwrap().contains("139"));
        assert_eq!(recommended_version(&index,project).unwrap().version,"0.1.2");
        assert_eq!(select(&index,"appleskin").unwrap().1.file.sha256,"b".repeat(64));
    }
    #[test] fn a_revoked_older_version_is_not_used_as_a_fallback() {
        let mut index=registry();index.revoked.push(Revoked{id:"appleskin".into(),sha256:Some("b".repeat(64)),reason:"withdrawn".into()});
        assert!(recommended_version(&index,&index.projects[0]).is_none());
        assert!(select(&index,"appleskin").is_err());
    }
    #[test] fn incompatible_releases_never_become_installable_from_registry_flags() {
        let mut index=registry();index.projects[0].versions.truncate(1);
        assert!(recommended_version(&index,&index.projects[0]).is_none());
    }
}
