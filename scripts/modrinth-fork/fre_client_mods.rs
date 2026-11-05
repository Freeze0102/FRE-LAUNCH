//! Settings shared by the launcher and the bundled native client mods.
use serde::{Deserialize, Serialize};
use std::{fs, io::{Read, Write}, path::Path};

pub const FILE: &str = "fre-client-mods.json";
const MAX_BYTES: u64 = 16 * 1024;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ZoomKey { #[default] C, V, Z }

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct ClientMods {
    pub zoom: bool,
    pub zoom_factor: u8,
    pub zoom_key: ZoomKey,
    pub fps_counter: bool,
    pub coordinates: bool,
}
impl Default for ClientMods {
    fn default() -> Self {
        Self { zoom: true, zoom_factor: 4, zoom_key: ZoomKey::C,
            fps_counter: false, coordinates: false }
    }
}
impl ClientMods {
    pub fn disabled() -> Self { Self { zoom: false, ..Self::default() } }
    pub fn validate(&self) -> Result<(), String> {
        if !(2..=10).contains(&self.zoom_factor) {
            return Err("Choose a zoom factor between 2 and 10.".into());
        }
        Ok(())
    }
    pub fn zoom_now(&self, held: bool, playing: bool, scoping: bool) -> Option<f32> {
        (self.zoom && held && playing && !scoping).then_some(f32::from(self.zoom_factor))
    }
}
fn regular(path: &Path) -> Result<bool, String> {
    match path.symlink_metadata() {
        Ok(meta) if meta.file_type().is_file() && meta.len() <= MAX_BYTES => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        _ => Err("Unsafe or oversized client mod settings file.".into()),
    }
}
pub fn read(root: &Path) -> Result<ClientMods, String> {
    let path = root.join(FILE);
    if !regular(&path)? { return Ok(ClientMods::default()); }
    let mut data = Vec::new();
    fs::File::open(path).and_then(|f| f.take(MAX_BYTES + 1).read_to_end(&mut data))
        .map_err(|e| e.to_string())?;
    if data.len() as u64 > MAX_BYTES { return Err("Client mod settings are too large.".into()); }
    let settings: ClientMods = serde_json::from_slice(&data)
        .map_err(|e| format!("Cannot read FRE client mod settings: {e}"))?;
    settings.validate()?;
    Ok(settings)
}
pub fn write(root: &Path, settings: &ClientMods) -> Result<(), String> {
    settings.validate()?;
    if !root.symlink_metadata().is_ok_and(|m| m.file_type().is_dir()) {
        return Err("Unsafe client mod profile directory.".into());
    }
    let target = root.join(FILE);
    let existed = regular(&target)?;
    let pending = root.join("fre-client-mods.json.pending");
    let backup = root.join("fre-client-mods.json.backup");
    if pending.symlink_metadata().is_ok() || backup.symlink_metadata().is_ok() {
        return Err("A previous client mod settings update needs recovery.".into());
    }
    let data = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&pending)
        .map_err(|e| e.to_string())?;
    file.write_all(&data).and_then(|()| file.sync_all()).map_err(|e| e.to_string())?;
    drop(file);
    if existed { fs::rename(&target, &backup).map_err(|e| e.to_string())?; }
    if let Err(error) = fs::rename(&pending, &target) {
        if existed { let _ = fs::rename(&backup, &target); }
        return Err(error.to_string());
    }
    if existed { fs::remove_file(&backup).map_err(|e| e.to_string())?; }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zoom_respects_release_menus_spyglass_and_disabled_state() {
        let mut settings = ClientMods::default();
        assert_eq!(settings.zoom_now(true, true, false), Some(4.0));
        for flags in [(false, true, false), (true, false, false), (true, true, true)] {
            assert_eq!(settings.zoom_now(flags.0, flags.1, flags.2), None);
        }
        settings.zoom = false;
        assert_eq!(settings.zoom_now(true, true, false), None);
    }
    #[test]
    fn settings_stay_per_profile_without_touching_worlds() {
        let one = tempfile::tempdir().unwrap();
        let two = tempfile::tempdir().unwrap();
        fs::create_dir(one.path().join("saves")).unwrap();
        fs::write(one.path().join("saves/world.dat"), b"world").unwrap();
        let settings = ClientMods { zoom_factor: 8, zoom_key: ZoomKey::V,
            coordinates: true, fps_counter: true, ..ClientMods::default() };
        write(one.path(), &settings).unwrap();
        assert_eq!(read(one.path()).unwrap(), settings);
        assert_eq!(read(two.path()).unwrap(), ClientMods::default());
        write(one.path(), &ClientMods::disabled()).unwrap();
        assert_eq!(read(one.path()).unwrap(), ClientMods::disabled());
        assert_eq!(fs::read(one.path().join("saves/world.dat")).unwrap(), b"world");
    }
    #[test]
    fn refuses_bad_values_unknown_keys_and_corrupt_files() {
        let temp = tempfile::tempdir().unwrap();
        for text in [r#"{"zoomFactor":0}"#, r#"{"zoomFactor":11}"#,
            r#"{"zoomKey":"execute"}"#, r#"{"unreviewedMod":true}"#, "broken"] {
            fs::write(temp.path().join(FILE), text).unwrap();
            assert!(read(temp.path()).is_err(), "{text}");
        }
        fs::write(temp.path().join(FILE), vec![b' '; MAX_BYTES as usize + 1]).unwrap();
        assert!(read(temp.path()).is_err());
    }
    #[test]
    fn preserves_interrupted_updates_and_non_regular_targets() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("fre-client-mods.json.pending"), b"keep").unwrap();
        assert!(write(temp.path(), &ClientMods::default()).is_err());
        assert_eq!(fs::read(temp.path().join("fre-client-mods.json.pending")).unwrap(), b"keep");
        fs::create_dir(temp.path().join(FILE)).unwrap();
        assert!(read(temp.path()).is_err());
        assert!(write(temp.path(), &ClientMods::default()).is_err());
    }
}
