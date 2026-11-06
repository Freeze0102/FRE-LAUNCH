//! Reviewed native-mod build pipeline. The source and compiler inputs are pinned;
//! native mod code remains executable code with the player's privileges.
use super::*;
use std::{
    io::{Read, Write},
    sync::OnceLock,
};
#[path = "fre_native_toolchain.rs"]
mod rust_toolchain;
static BUILD_ROOT: OnceLock<PathBuf> = OnceLock::new();
static BUILD_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
mod process {
    pub const CREATE_NO_WINDOW: u32 = 0x08000000;
    pub fn command(program: impl AsRef<std::ffi::OsStr>) -> std::process::Command {
        let mut cmd = std::process::Command::new(program);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        cmd
    }
}
mod backend {
    use super::*;
    pub fn build_dir() -> PathBuf {
        BUILD_ROOT
            .get()
            .expect("validated native build root")
            .clone()
    }
    pub fn download(
        url: &str,
        _size: u64,
        sha: &str,
        path: &Path,
        progress: &impl Fn(f32),
    ) -> Result<(), String> {
        let parsed = reqwest::Url::parse(url).map_err(|e| e.to_string())?;
        if parsed.scheme() != "https"
            || !matches!(
                parsed.host_str(),
                Some("static.rust-lang.org" | "github.com")
            )
        {
            return Err("Untrusted compiler download".into());
        }
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(1200))
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() > 5
                    || attempt.url().scheme() != "https"
                    || !matches!(
                        attempt.url().host_str(),
                        Some(
                            "static.rust-lang.org"
                                | "github.com"
                                | "release-assets.githubusercontent.com"
                                | "objects.githubusercontent.com"
                        )
                    )
                {
                    attempt.error("Untrusted compiler redirect")
                } else {
                    attempt.follow()
                }
            }))
            .build()
            .map_err(|e| e.to_string())?;
        let mut response = client
            .get(parsed)
            .send()
            .and_then(|r| r.error_for_status())
            .map_err(|e| format!("Compiler download: {e}"))?;
        let len = response.content_length().unwrap_or(512 * 1024 * 1024);
        if len > 1024 * 1024 * 1024 {
            return Err("Compiler archive too large".into());
        }
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        let mut hash = Sha256::new();
        let mut total = 0;
        let mut buf = [0u8; 65536];
        let result = (|| {
            loop {
                let n = response.read(&mut buf).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                total += n as u64;
                if total > 1024 * 1024 * 1024 {
                    return Err("Compiler archive too large".into());
                }
                hash.update(&buf[..n]);
                file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
                progress(total as f32 / len.max(1) as f32);
            }
            if format!("{:x}", hash.finalize()) != sha {
                return Err("Compiler SHA-256 mismatch".into());
            }
            file.sync_all().map_err(|e| e.to_string())
        })();
        drop(file);
        if result.is_err() {
            let _ = fs::remove_file(path);
        }
        result
    }
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildReceipt {
    schema: u32,
    key: String,
    game_sha256: String,
    source_sha256: String,
    mods: Vec<(String, String)>,
}
#[derive(Deserialize)]
struct Tools {
    schema: u32,
    source_sha256: String,
    mixins_sha256: String,
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn hash_file(path: &Path) -> Result<String, String> {
    let mut f = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut h = Sha256::new();
    let mut b = [0u8; 65536];
    loop {
        let n = f.read(&mut b).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        h.update(&b[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
fn tool_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let text = path.to_string_lossy();
        if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{rest}"));
        }
        if let Some(rest) = text.strip_prefix(r"\\?\") {
            return PathBuf::from(rest);
        }
    }
    path
}
fn owned_dir(parent: &Path, name: &str) -> Result<PathBuf, String> {
    let path = parent.join(name);
    if !path.exists() {
        fs::create_dir(&path).map_err(|e| e.to_string())?;
    }
    if !path
        .symlink_metadata()
        .is_ok_and(|m| m.file_type().is_dir())
    {
        return Err("Unsafe native build directory".into());
    }
    let actual = path.canonicalize().map_err(|e| e.to_string())?;
    if actual.parent() != Some(parent.canonicalize().map_err(|e| e.to_string())?.as_path()) {
        return Err("Native build directory leaves its cache".into());
    }
    Ok(tool_path(actual))
}
fn normalize(text: &str) -> String {
    text.replace("horizon-", "fre-")
        .replace("horizon_", "fre_")
        .replace("horizon/", "fre/")
        .replace("horizon:", "fre:")
        .replace("HORIZON_", "FRE_")
        .replace("Horizon", "Fre")
}
fn unpack_source(archive: &Path, out: &Path) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(fs::File::open(archive).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let mut total = 0u64;
    if zip.len() > 10000 {
        return Err("Source package has too many files".into());
    }
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = entry
            .enclosed_name()
            .ok_or("Unsafe source entry")?
            .to_owned();
        if entry.is_dir() {
            continue;
        }
        total += entry.size();
        if total > 512 * 1024 * 1024 {
            return Err("Source package too large".into());
        }
        let path = out.join(name);
        fs::create_dir_all(path.parent().ok_or("Invalid source path")?)
            .map_err(|e| e.to_string())?;
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut file).map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn stage_mod(bytes: &[u8], facts: &mod_audit::PackageFacts, out: &Path) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let mut manifests = 0;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_owned();
        if entry.is_dir() || !name.starts_with("mixins/") {
            continue;
        }
        let relative = Path::new(&name)
            .strip_prefix("mixins")
            .map_err(|e| e.to_string())?;
        if relative
            .components()
            .any(|p| !matches!(p, std::path::Component::Normal(_)))
        {
            return Err("Unsafe mixin path".into());
        }
        let renamed = normalize(&relative.to_string_lossy());
        let path = out.join(renamed);
        if !name.ends_with(".rs") && !name.ends_with(".toml") {
            return Err("This native mod needs build inputs that FRE has not enabled".into());
        }
        let mut text = String::new();
        entry.read_to_string(&mut text).map_err(|e| e.to_string())?;
        text = normalize(&text);
        if path.file_name().is_some_and(|n| n == "fre-mixin.toml") {
            let mut value: toml::Value = toml::from_str(&text).map_err(|e| e.to_string())?;
            if value.get("crate").is_some() || value.get("post_build").is_some() {
                return Err("This mod supplies extra crates or post-build programs; FRE currently builds source mixins only".into());
            }
            if let Some(hooks) = facts.hooks {
                if hooks > 80 {
                    return Err("This mod needs a newer native hook API".into());
                }
                if let Some(table) = value
                    .get_mut("compatibility")
                    .and_then(|v| v.as_table_mut())
                {
                    table.insert(
                        "fre_revision".into(),
                        toml::Value::String("975c07d9151847e6af13afc4b5ab37b6bbe78ef5".into()),
                    );
                }
            } else if facts.engine_revision.as_deref()
                != Some("975c07d9151847e6af13afc4b5ab37b6bbe78ef5")
            {
                return Err("This mod requires another engine revision".into());
            }
            text = toml::to_string_pretty(&value).map_err(|e| e.to_string())?;
            manifests += 1;
        }
        fs::create_dir_all(path.parent().ok_or("Missing mixin parent")?)
            .map_err(|e| e.to_string())?;
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .and_then(|mut f| f.write_all(text.as_bytes()))
            .map_err(|e| e.to_string())?;
    }
    if manifests == 0 {
        return Err("This native archive has no mixin manifest".into());
    }
    Ok(())
}
fn run(cmd: &mut std::process::Command, log: &Path, what: &str) -> Result<(), String> {
    rust_toolchain::low_priority(cmd);
    let output = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
        .map_err(|e| e.to_string())?;
    let status = cmd
        .stdout(output.try_clone().map_err(|e| e.to_string())?)
        .stderr(output)
        .status()
        .map_err(|e| format!("{what}: {e}"))?;
    if !status.success() {
        let text = fs::read_to_string(log).unwrap_or_default();
        let tail = text
            .lines()
            .rev()
            .take(18)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!("{what} failed ({status}).\n{}", normalize(&tail)));
    }
    Ok(())
}
/// Build on the exact bundled source. A failed build never replaces the bundled game.
pub(super) async fn ensure<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    root: &Path,
    profile: &Path,
    runtime: &Path,
    profile_id: &str,
) -> Result<Option<PathBuf>, String> {
    let app = app.clone();
    let id = profile_id.to_owned();
    ensure_with_progress(
        root,
        profile,
        runtime,
        std::sync::Arc::new(move |stage| {
            let _ = app.emit(
                "fre-rust-mod-build",
                serde_json::json!({"profileId":id,"stage":stage}),
            );
        }),
    )
    .await
}
async fn ensure_with_progress(
    root: &Path,
    profile: &Path,
    runtime: &Path,
    progress: std::sync::Arc<dyn Fn(&str) + Send + Sync>,
) -> Result<Option<PathBuf>, String> {
    let _guard = BUILD_LOCK.lock().await;
    let enabled = catalog::enabled_content(profile)?;
    let mods = catalog::verified_native(root, profile, &enabled).await?;
    if mods.is_empty() {
        return Ok(None);
    }
    let tools_path = runtime.join("fre-build-tools.json");
    let tools: Tools =
        serde_json::from_slice(&fs::read(&tools_path).map_err(|_| {
            "Native build tools are missing; install the current FRE LAUNCH update"
        })?)
        .map_err(|e| e.to_string())?;
    if tools.schema != 1 {
        return Err("Unsupported native build tools".into());
    }
    let source = runtime.join("fre-engine-source.zip");
    let tool = runtime.join("fre-mixins.exe");
    if hash_file(&source)? != tools.source_sha256 || hash_file(&tool)? != tools.mixins_sha256 {
        return Err("Native build tools failed their SHA-256 check".into());
    }
    let identities: Vec<(String, String)> = mods
        .iter()
        .map(|(id, bytes)| (id.clone(), sha(bytes)))
        .collect();
    let key = sha(&serde_json::to_vec(&(
        &tools.source_sha256,
        &tools.mixins_sha256,
        rust_toolchain::VERSION,
        "fast",
        1u32,
        &identities,
    ))
    .map_err(|e| e.to_string())?);
    let builds = owned_dir(root, "native-builds")?;
    let selected = owned_dir(&builds, &key)?;
    let exe = selected.join("fre-rust-client.exe");
    let receipt = selected.join("receipt.json");
    if exe.exists() && receipt.exists() {
        let previous: BuildReceipt =
            serde_json::from_slice(&fs::read(&receipt).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        if previous.schema == 1
            && previous.key == key
            && previous.mods == identities
            && previous.source_sha256 == tools.source_sha256
            && hash_file(&exe)? == previous.game_sha256
        {
            return Ok(Some(exe));
        }
        return Err("A cached native build changed; preserve it and rebuild".into());
    }
    let build_root = owned_dir(root, "native-build-cache")?;
    if let Some(existing) = BUILD_ROOT.get() {
        if *existing != build_root {
            return Err("The native build cache moved; restart FRE LAUNCH".into());
        }
    } else {
        let _ = BUILD_ROOT.set(build_root.clone());
    }
    let executable = exe.clone();
    let source_hash = tools.source_sha256.clone();
    let built = tokio::task::spawn_blocking(move || -> Result<BuildReceipt, String> {
        let notify = |stage: &str| {
            progress(stage);
        };
        notify("compiler");
        rust_toolchain::linker()?;
        let rust = rust_toolchain::install(|_| {})?;
        if !rust
            .version()?
            .starts_with(&format!("rustc {}", rust_toolchain::VERSION))
        {
            return Err("The managed Rust compiler version changed".into());
        }
        let sources = owned_dir(&build_root, "sources")?;
        let pristine = sources.join(&source_hash);
        if !pristine.exists() {
            let incoming = tempfile::tempdir_in(&sources).map_err(|e| e.to_string())?;
            unpack_source(&source, incoming.path())?;
            fs::rename(incoming.path(), &pristine).map_err(|e| e.to_string())?;
        }
        let incoming = tempfile::tempdir_in(&build_root).map_err(|e| e.to_string())?;
        let mods_dir = owned_dir(incoming.path(), "mods")?;
        for (id, bytes) in &mods {
            let facts = mod_audit::inspect(bytes, "native-mod", None)?;
            if facts.id != *id {
                return Err("Native mod identity changed".into());
            }
            stage_mod(bytes, &facts, &mods_dir)?;
        }
        notify("dependencies");
        let log = selected.join("build.log");
        let mut fetch = rust.command(&rust.cargo());
        fetch
            .args(["fetch", "--locked", "--manifest-path"])
            .arg(pristine.join("Cargo.toml"));
        run(&mut fetch, &log, "Downloading pinned engine dependencies")?;
        notify("applying");
        let stage = build_root.join("stage");
        let mut apply = rust.command(&tool);
        apply
            .arg("apply")
            .arg("--workspace")
            .arg(&pristine)
            .arg("--mods")
            .arg(&mods_dir)
            .arg("--out")
            .arg(&stage);
        run(&mut apply, &log, "Applying native mods")?;
        // The CLI may add local source modules, but no new downloaded packages or build programs.
        if fs::read(pristine.join("Cargo.lock")).map_err(|e| e.to_string())?
            != fs::read(stage.join("Cargo.lock")).map_err(|e| e.to_string())?
        {
            return Err("Native mods changed the pinned dependency lock".into());
        }
        notify("compiling");
        let target = build_root.join("target");
        let mut compile = rust.command(&rust.cargo());
        compile
            .args([
                "build",
                "--locked",
                "--offline",
                "--profile",
                "fast",
                "--manifest-path",
            ])
            .arg(stage.join("Cargo.toml"))
            .args(["-p", "fre-client", "--bin", "fre-client"])
            .env("CARGO_TARGET_DIR", &target);
        run(&mut compile, &log, "Compiling the modded FRE game")?;
        let binary = target.join("fast/fre-client.exe");
        fs::copy(&binary, &executable).map_err(|e| e.to_string())?;
        notify("scanning");
        Ok(BuildReceipt {
            schema: 1,
            key,
            game_sha256: hash_file(&executable)?,
            source_sha256: source_hash,
            mods: identities,
        })
    })
    .await
    .map_err(|e| e.to_string())??;
    catalog::scan_native_executable(&exe).await?;
    if hash_file(&exe)? != built.game_sha256 {
        return Err("The native game changed during its scan".into());
    }
    let mut pending =
        tempfile::NamedTempFile::new_in(receipt.parent().ok_or("Missing build directory")?)
            .map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(&mut pending, &built).map_err(|e| e.to_string())?;
    pending.as_file().sync_all().map_err(|e| e.to_string())?;
    pending.persist(receipt).map_err(|e| e.to_string())?;
    Ok(Some(exe))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore = "Downloads the pinned compiler, scans a real registry mod and compiles the native game"]
    async fn real_native_catalog_install_builds_a_scanned_fullbright_game() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .canonicalize()
            .unwrap();
        let root = repo.join(".audit/native-builder-e2e");
        fs::create_dir_all(&root).unwrap();
        let profile = root.join("profile");
        fs::create_dir_all(&profile).unwrap();
        let runtime = repo.join("scripts/modrinth-fork/fre-rust-runtime");
        if !profile.join("fre-mods/fullbright-1.0.0.hnmod").exists() {
            let report = catalog::install(
                &root,
                &profile,
                "fullbright",
                "573c5e04188e0b1a26eb42a8aeaeeefeb6fb334a096afaa834e71b8efa353ad7",
            )
            .await
            .unwrap();
            assert!(report.installable);
        }
        let binary = ensure_with_progress(
            &root,
            &profile,
            &runtime,
            std::sync::Arc::new(|stage| eprintln!("FRE native build: {stage}")),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(binary.is_file());
        let cached = ensure_with_progress(
            &root,
            &profile,
            &runtime,
            std::sync::Arc::new(|_| panic!("A verified build must be reused")),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(cached, binary);
        fs::write(
            root.join("verified-game-path.txt"),
            binary.to_string_lossy().as_bytes(),
        )
        .unwrap();
    }
    #[tokio::test]
    #[ignore = "Downloads the compiler and builds the real compatible AppleSkin release"]
    async fn real_native_appleskin_fallback_builds_a_scanned_game() {
        let repo=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..").canonicalize().unwrap();
        let root=repo.join(".audit/appleskin-builder-e2e");let profile=root.join("profile");
        fs::create_dir_all(&profile).unwrap();
        let view=catalog::view(&root,true).await.unwrap();
        let project=view.projects.iter().find(|project|project.id=="appleskin").unwrap();
        let release=project.versions.iter().find(|release|release.supported).unwrap();
        assert!(release.hooks.is_none_or(|hooks|hooks<=80));
        eprintln!("AppleSkin compatible release: {} {}",release.version,release.file.sha256);
        if !profile.join("fre-mods").join(&release.file.name).is_file() {
            let report=catalog::install(&root,&profile,"appleskin",&release.file.sha256).await.unwrap();
            assert!(report.installable);
        }
        let runtime=repo.join("scripts/modrinth-fork/fre-rust-runtime");
        let binary=ensure_with_progress(&root,&profile,&runtime,std::sync::Arc::new(|stage|eprintln!("AppleSkin build: {stage}"))).await.unwrap().unwrap();
        assert!(binary.is_file());
        let reused=ensure_with_progress(&root,&profile,&runtime,std::sync::Arc::new(|_|panic!("A verified AppleSkin build must be reused"))).await.unwrap().unwrap();
        assert_eq!(binary,reused);
        fs::write(root.join("verified-game-path.txt"),binary.to_string_lossy().as_bytes()).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn compiler_paths_strip_windows_verbatim_prefixes_after_validation() {
        assert_eq!(
            tool_path(PathBuf::from(r"\\?\C:\Users\Player\FRE")),
            PathBuf::from(r"C:\Users\Player\FRE")
        );
        assert_eq!(
            tool_path(PathBuf::from(r"C:\Users\Player\FRE")),
            PathBuf::from(r"C:\Users\Player\FRE")
        );
    }
    #[test]
    fn native_branding_does_not_rewrite_unrelated_words() {
        assert_eq!(
            normalize("horizon_client::game horizon-mixin.toml"),
            "fre_client::game fre-mixin.toml"
        );
        assert_eq!(normalize("horizontal"), "horizontal");
        assert_eq!(normalize("HorizonState"), "FreState");
    }
    #[test]
    fn build_cache_refuses_a_file_instead_of_a_directory() {
        let t = tempfile::tempdir().unwrap();
        fs::write(t.path().join("cache"), b"keep").unwrap();
        assert!(owned_dir(t.path(), "cache").is_err());
    }
}
