//! Non-executing inspection of downloaded Rust mod packages.
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, io::{Cursor, Read}};

const MAX_ENTRY: u64 = 8 << 20;
const MAX_TOTAL: u64 = 128 << 20;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageFacts {
    pub id: String,
    pub name: String,
    pub authors: Vec<String>,
    pub version: String,
    pub api: String,
    pub content_only: bool,
    pub files: usize,
    pub expanded_bytes: u64,
    pub code_files: usize,
    pub indicators: Vec<String>,
    pub engine_revision: Option<String>,
    pub mixins_revision: Option<String>,
    pub hooks: Option<u32>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContentManifest {
    id: String, name: String, version: String, api: String,
    #[serde(default)] description: String,
    #[serde(default)] authors: Vec<String>,
    #[serde(default = "client")] side: String,
    #[serde(default)] capabilities: Vec<String>,
    #[serde(default)] namespaces: Vec<String>,
    #[serde(default)] experimental: Vec<String>,
}
fn client() -> String { "client".into() }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NativeManifest {
    schema: u32, id: String, version: String, game: String,
    #[serde(rename = "horizon", alias = "fre")] engine: String,
    mixins: String, #[serde(default)] hooks: Option<u32>,
}
pub fn valid_id(id: &str) -> bool {
    (1..=64).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
pub fn revision(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub fn sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn plain_path(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 240 || name.starts_with('/') || name.contains(['\\', ':', '\0']) {
        return Err("Unsafe archive path".into());
    }
    for part in name.trim_end_matches('/').split('/') {
        if part.is_empty() || part == "." || part == ".." || part.ends_with(['.', ' '])
            || !part.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b)) {
            return Err(format!("Unsafe archive path: {name}"));
        }
        let stem = part.split('.').next().unwrap_or("").to_ascii_lowercase();
        if ["con", "prn", "aux", "nul"].contains(&stem.as_str())
            || (stem.len() == 4 && (stem.starts_with("com") || stem.starts_with("lpt")) && stem.as_bytes()[3].is_ascii_digit()) {
            return Err("Windows device name in archive".into());
        }
    }
    Ok(())
}
fn native_allowed(path: &str, id: &str) -> bool {
    let Some(rest) = path.strip_prefix(&format!("mixins/{id}/")) else { return false; };
    let name = rest.rsplit('/').next().unwrap_or("");
    path.ends_with(".rs") || path.ends_with(".toml") || rest.starts_with("data/")
        || rest.split('/').nth(1) == Some("data")
        || ["README", "LICENSE", "LICENCE", "COPYING", "NOTICE"].iter().any(|prefix| name.starts_with(prefix))
}
fn data_path(path: &str, namespaces: &BTreeSet<String>) -> bool {
    let parts: Vec<_> = path.split('/').collect();
    if parts.len() < 3 || !["assets", "data"].contains(&parts[0]) { return false; }
    if !path.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"/._-".contains(&b)) { return false; }
    let namespace = parts[1];
    if !namespaces.contains(namespace) && !(parts[0] == "data" && ["minecraft", "c"].contains(&namespace) && parts[2] == "tags") { return false; }
    [".json", ".png", ".ogg", ".mcmeta", ".mcfunction"].iter().any(|suffix| path.ends_with(suffix))
}

pub fn inspect(bytes: &[u8], kind: &str, expected: Option<(&str, &str, &str)>) -> Result<PackageFacts, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| "Invalid ZIP mod archive")?;
    if zip.len() == 0 || zip.len() > 4096 { return Err("Archive file count exceeds limits".into()); }
    let mut files = Vec::new();
    let mut seen = BTreeSet::new();
    let mut total = 0u64;
    let mut pixels = 0u64;
    let mut code_files = 0;
    let mut indicators = BTreeSet::new();
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(|_| "Cannot read ZIP entry")?;
        let name = file.name().to_owned();
        plain_path(&name)?;
        if file.unix_mode().is_some_and(|mode| mode & 0o170000 == 0o120000) { return Err("Links are not allowed in mod archives".into()); }
        if !seen.insert(name.to_ascii_lowercase()) { return Err("Duplicate archive path".into()); }
        if file.is_dir() { continue; }
        if file.size() > MAX_ENTRY { return Err("Expanded mod file exceeds 8 MiB".into()); }
        total = total.checked_add(file.size()).ok_or("Archive size overflow")?;
        if total > MAX_TOTAL { return Err("Expanded mod archive exceeds 128 MiB".into()); }
        let mut data = Vec::new();
        file.by_ref().take(MAX_ENTRY + 1).read_to_end(&mut data).map_err(|_| "Invalid, encrypted or corrupt mod entry")?;
        if data.len() as u64 != file.size() || data.len() as u64 > MAX_ENTRY { return Err("Mod entry length mismatch".into()); }
        if data.starts_with(b"MZ") || data.starts_with(b"\x7fELF") { return Err("Executable binary hidden in mod archive".into()); }
        if name.ends_with(".png") {
            if data.len() < 24 || &data[..8] != b"\x89PNG\r\n\x1a\n" || &data[12..16] != b"IHDR" { return Err("Invalid PNG in mod archive".into()); }
            let width = u32::from_be_bytes(data[16..20].try_into().unwrap());
            let height = u32::from_be_bytes(data[20..24].try_into().unwrap());
            if width == 0 || height == 0 || width > 4096 || height > 4096 { return Err("Mod texture dimensions exceed limits".into()); }
            pixels += u64::from(width) * u64::from(height);
            if pixels > 16_777_216 { return Err("Mod texture pixel budget exceeds limits".into()); }
        }
        if name.ends_with(".json") || name.ends_with(".mcmeta") {
            let _: serde_json::Value = serde_json::from_slice(&data).map_err(|_| "Malformed mod JSON")?;
        }
        if name.ends_with(".rs") || name.ends_with(".toml") {
            let text = std::str::from_utf8(&data).map_err(|_| "Mod source is not UTF-8")?;
            let compact: String = text.chars().filter(|c| !c.is_whitespace()).flat_map(char::to_lowercase).collect();
            if name.ends_with(".rs") { code_files += 1; }
            for (needle, label) in [("std::process", "process"), ("command::new", "process"), ("powershell", "process"),
                ("reqwest", "network"), ("ureq", "network"), ("std::net", "network"), ("std::env", "environment"),
                ("unsafe", "unsafe"), ("extern\"c\"", "ffi"), ("build.rs", "build-script"), ("proc-macro=true", "build-script")] {
                if compact.contains(needle) || name.ends_with(needle) { indicators.insert(label.to_owned()); }
            }
        }
        if name.ends_with(".mcfunction") {
            let text=std::str::from_utf8(&data).map_err(|_|"Minecraft data function is not UTF-8")?;
            if text.lines().any(|line|line.len()>65536) { return Err("Minecraft data function line exceeds limits".into()); }
            indicators.insert("game-commands".into());
        }
        if name.ends_with(".wasm") { code_files += 1; indicators.insert("legacy-wasm".into()); }
        files.push((name, data));
    }
    if kind == "native-mod" {
        let manifests: Vec<_> = files.iter().filter(|(name, _)| ["horizon-native.json", "fre-native.json"].contains(&name.as_str())).collect();
        if manifests.len() != 1 { return Err("Exactly one native manifest is required".into()); }
        let manifest: NativeManifest = serde_json::from_slice(&manifests[0].1).map_err(|_| "Invalid native mod manifest")?;
        if manifest.schema != 1 || !valid_id(&manifest.id) || !revision(&manifest.engine) || !revision(&manifest.mixins)
            || manifest.game != "26.3" || manifest.hooks == Some(0) { return Err("Invalid native compatibility declaration".into()); }
        let api = format!("native-1/{}/{}", manifest.engine, manifest.mixins);
        if let Some((id, version, expected_api)) = expected {
            if manifest.id != id || manifest.version != version || api != expected_api { return Err("Native manifest does not match the catalog".into()); }
        }
        for (name, _) in &files {
            if !["horizon-native.json", "fre-native.json"].contains(&name.as_str()) && !native_allowed(name, &manifest.id) {
                return Err(format!("Unexpected native bundle file: {name}"));
            }
        }
        return Ok(PackageFacts { name:manifest.id.clone(), authors:Vec::new(), id:manifest.id, version:manifest.version, api, content_only:false,
            files:files.len(), expanded_bytes:total, code_files, indicators:indicators.into_iter().collect(),
            engine_revision:Some(manifest.engine), mixins_revision:Some(manifest.mixins), hooks:manifest.hooks });
    }
    if kind != "mod" { return Err("This catalog item needs a separate pack validator".into()); }
    let manifests: Vec<_> = files.iter().filter(|(name, _)| ["horizon-mod.toml", "fre-mod.toml"].contains(&name.as_str())).collect();
    if manifests.len() != 1 { return Err("Exactly one content mod manifest is required".into()); }
    let manifest: ContentManifest = toml::from_str(std::str::from_utf8(&manifests[0].1).map_err(|_| "Invalid manifest encoding")?)
        .map_err(|_| "Invalid content mod manifest")?;
    if !valid_id(&manifest.id) || manifest.name.is_empty() || manifest.name.len()>256
        || manifest.version.is_empty() || manifest.version.len()>80 || manifest.description.len()>65536
        || manifest.authors.len()>100 || manifest.authors.iter().any(|name|name.len()>256)
        || manifest.namespaces.len()>100 || !["0.1", "0.2", "0.3"].contains(&manifest.api.as_str())
        || !["client", "server", "both"].contains(&manifest.side.as_str()) { return Err("Invalid content mod identity or API".into()); }
    if let Some((id, version, api)) = expected {
        if manifest.id != id || manifest.version != version || manifest.api != api { return Err("Content manifest does not match the catalog".into()); }
    }
    let mut namespaces: BTreeSet<_> = manifest.namespaces.into_iter().collect();
    namespaces.insert(manifest.id.clone());
    if namespaces.iter().any(|name| !valid_id(name) || ["minecraft", "c", "horizon", "fre"].contains(&name.as_str())) { return Err("Content namespace is reserved or invalid".into()); }
    let content_only = manifest.api == "0.2" && code_files == 0 && manifest.capabilities == ["registries"];
    for (name, _) in &files {
        if ["horizon-mod.toml", "fre-mod.toml"].contains(&name.as_str()) { continue; }
        if name == "mod.wasm" { continue; }
        if !data_path(name, &namespaces) { return Err(format!("Unsupported content file or namespace: {name}")); }
    }
    let _ = (manifest.description, manifest.experimental);
    Ok(PackageFacts { id:manifest.id, name:manifest.name, authors:manifest.authors, version:manifest.version, api:manifest.api, content_only,
        files:files.len(), expanded_bytes:total, code_files, indicators:indicators.into_iter().collect(),
        engine_revision:None, mixins_revision:None, hooks:None })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    fn bundle(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name,data) in files {
            zip.start_file(*name,zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }
    const MANIFEST: &[u8] = b"id='demo'\nname='Demo'\nversion='1'\napi='0.2'\ncapabilities=['registries']\n";
    #[test]
    fn accepts_data_but_never_marks_code_as_installable() {
        let data=bundle(&[("fre-mod.toml",MANIFEST),("assets/demo/lang/en_us.json",b"{}")]);
        assert!(inspect(&data,"mod",Some(("demo","1","0.2"))).unwrap().content_only);
        let code=bundle(&[("fre-mod.toml",MANIFEST),("mod.wasm",b"\0asm\x01\0\0\0")]);
        assert!(!inspect(&code,"mod",None).unwrap().content_only);
    }
    #[test]
    fn rejects_traversal_windows_paths_and_reserved_names() {
        for path in ["../evil.rs","C:/evil.rs","assets\\demo\\x.json","assets/demo/con.json","assets/demo/x.json:evil"] {
            let data=bundle(&[("fre-mod.toml",MANIFEST),(path,b"{}")]);
            assert!(inspect(&data,"mod",None).is_err(),"{path}");
        }
    }
    #[test]
    fn rejects_identity_changes_namespace_escapes_and_dual_manifests() {
        let data=bundle(&[("fre-mod.toml",MANIFEST)]);
        assert!(inspect(&data,"mod",Some(("other","1","0.2"))).is_err());
        let huge_name=format!("id='demo'\nname='{}'\nversion='1'\napi='0.2'\n","x".repeat(257));
        assert!(inspect(&bundle(&[("fre-mod.toml",huge_name.as_bytes())]),"mod",None).is_err());

        let data=bundle(&[("fre-mod.toml",MANIFEST),("assets/minecraft/lang/en_us.json",b"{}")]);
        assert!(inspect(&data,"mod",None).is_err());
        let data=bundle(&[("fre-mod.toml",MANIFEST),("horizon-mod.toml",MANIFEST)]);
        assert!(inspect(&data,"mod",None).is_err());
    }
    #[test]
    fn validates_json_png_and_hidden_executables() {
        for (name,bytes) in [("assets/demo/a.json",b"{".as_slice()),("assets/demo/a.png",b"bad".as_slice()),("assets/demo/a.ogg",b"MZevil".as_slice())] {
            let data=bundle(&[("fre-mod.toml",MANIFEST),(name,bytes)]);
            assert!(inspect(&data,"mod",None).is_err());
        }
    }
    #[test]
    fn distinguishes_minecraft_functions_from_native_code_and_refuses_unknown_binary_formats() {
        let data=bundle(&[("fre-mod.toml",MANIFEST),("data/demo/function/check.mcfunction",b"say hello")]);
        let facts=inspect(&data,"mod",None).unwrap();
        assert!(facts.content_only);
        assert_eq!(facts.indicators,["game-commands"]);
        let data=bundle(&[("fre-mod.toml",MANIFEST),("data/demo/structure/demo.nbt",b"unknown")]);
        assert!(inspect(&data,"mod",None).is_err());
    }
    #[test]
    fn rejects_expansion_bombs_and_invalid_native_revisions() {
        let huge=vec![0u8; (MAX_ENTRY+1) as usize];
        let data=bundle(&[("fre-mod.toml",MANIFEST),("assets/demo/huge.ogg",&huge)]);
        assert!(inspect(&data,"mod",None).is_err());
        let manifest=br#"{"schema":1,"id":"demo","version":"1","game":"26.3","horizon":"main","mixins":"main"}"#;
        let data=bundle(&[("horizon-native.json",manifest)]);
        assert!(inspect(&data,"native-mod",None).is_err());
    }
    #[test]
    fn native_source_is_read_only_and_needs_review_even_with_clean_structure() {
        let manifest=format!(r#"{{"schema":1,"id":"demo","version":"1","game":"26.3","horizon":"{}","mixins":"{}","hooks":1}}"#,"1".repeat(40),"2".repeat(40));
        let data=bundle(&[("horizon-native.json",manifest.as_bytes()),("mixins/demo/a.rs",b"fn hook(){std::process::Command::new(\"x\");}")]);
        let result=inspect(&data,"native-mod",None).unwrap();
        assert!(!result.content_only);
        assert_eq!(result.code_files,1);
        assert_eq!(result.indicators,["process"]);
    }
}
