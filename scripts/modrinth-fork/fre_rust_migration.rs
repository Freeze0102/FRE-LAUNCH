//! Upgrade FRE-owned profile filenames without rewriting saved worlds.
use std::{fs, path::Path};

/// Only for FRE's generated resource/preparer caches, never player worlds.
pub fn cache(root: &Path) -> Result<(), String> {
    match fs::symlink_metadata(root) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Ok(meta) if meta.file_type().is_dir() => {},
        _ => return Err("Unsafe FRE generated cache directory.".into()),
    }
    let mut queue = vec![(root.to_path_buf(), 0usize)];
    let mut moves = Vec::new();
    let mut count = 0usize;
    while let Some((directory, depth)) = queue.pop() {
        if depth > 32 { return Err("Too many nested FRE cache directories.".into()); }
        for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            count += 1;
            if count > 100_000 { return Err("FRE cache contains too many files to upgrade.".into()); }
            let kind = entry.file_type().map_err(|e| e.to_string())?;
            if !kind.is_file() && !kind.is_dir() { return Err("Unsafe link in FRE generated cache.".into()); }
            if kind.is_dir() { queue.push((entry.path(), depth + 1)); }
            let old = entry.file_name();
            let Some(old) = old.to_str() else { continue };
            let new = old.replace("horizonlab", "frelaunch").replace("horizontal", "\u{1}")
                .replace("horizon", "fre").replace("\u{1}", "horizontal")
                .replace("Horizontal", "\u{2}").replace("Horizon", "Fre").replace("\u{2}", "Horizontal")
                .replace("HORIZONTAL", "\u{3}").replace("HORIZON", "FRE").replace("\u{3}", "HORIZONTAL");
            if new == old { continue; }
            let target = entry.path().with_file_name(new);
            match fs::symlink_metadata(&target) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
                _ => return Err("Both old and new FRE cache entries exist; preserved both. Resolve the duplicate before preparing.".into()),
            }
            moves.push((depth, entry.path(), target));
        }
    }
    moves.sort_by_key(|(depth, _, _)| std::cmp::Reverse(*depth));
    for (_, from, to) in moves {
        fs::rename(from, to).map_err(|e| format!("Could not upgrade FRE cache filenames: {e}"))?;
    }
    Ok(())
}

pub fn profile(root: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(root).map_err(|e| e.to_string())?;
    if !metadata.file_type().is_dir() { return Err("Unsafe FRE profile directory.".into()); }
    let mut moves = Vec::new();
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        // Only engine-owned names. Never rename user worlds or mod filenames.
        if name != "horizon.txt" && !name.starts_with("horizon-") && !name.starts_with("horizon_") { continue; }
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if !kind.is_dir() && !kind.is_file() { return Err("Unsafe legacy FRE profile entry.".into()); }
        let target = root.join(name.replacen("horizon", "fre", 1));
        match fs::symlink_metadata(&target) {
            Ok(_) => return Err(format!("Both old and new FRE profile files exist: {}. Keep both and resolve the duplicate before playing.", target.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
            Err(error) => return Err(error.to_string()),
        }
        moves.push((entry.path(), target));
    }
    if moves.is_empty() { return Ok(()); }
    // A previous launcher may still have a world open. Take vanilla's real
    // session locks before renaming anything, including profile settings.
    let mut world_locks = Vec::new();
    let saves = root.join("saves");
    if saves.exists() {
        if !fs::symlink_metadata(&saves).map_err(|e| e.to_string())?.file_type().is_dir() {
            return Err("Unsafe FRE saves directory.".into());
        }
        for entry in fs::read_dir(&saves).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if !entry.file_type().map_err(|e| e.to_string())?.is_dir() { continue; }
            if !entry.path().join("level.dat").is_file() { continue; }
            let lock_path = entry.path().join("session.lock");
            if fs::symlink_metadata(&lock_path).is_ok_and(|m| !m.file_type().is_file()) {
                return Err("Unsafe world session lock.".into());
            }
            let file = fs::OpenOptions::new().write(true).create(true).truncate(false)
                .open(lock_path).map_err(|e| e.to_string())?;
            file.try_lock().map_err(|_| "Close the running Rust game before upgrading this profile.")?;
            world_locks.push(file);
        }
    }

    for (source, target) in moves {
        fs::rename(source, target).map_err(|e| format!("Could not upgrade FRE profile filenames: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upgrades_nested_cache_names_without_changing_file_bytes() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("mods/horizon/dev/horizonlab")).unwrap();
        fs::write(temp.path().join("mods/horizon/dev/horizonlab/HorizonOracle.java"), b"original").unwrap();
        fs::write(temp.path().join("horizontal.txt"), b"geometry").unwrap();
        cache(temp.path()).unwrap();
        cache(temp.path()).unwrap();
        assert_eq!(fs::read(temp.path().join("mods/fre/dev/frelaunch/FreOracle.java")).unwrap(), b"original");
        assert_eq!(fs::read(temp.path().join("horizontal.txt")).unwrap(), b"geometry");
    }
    #[test]
    fn cache_conflicts_are_detected_before_any_rename() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("horizon-resources.json"), b"old").unwrap();
        fs::write(temp.path().join("fre-resources.json"), b"new").unwrap();
        fs::write(temp.path().join("horizon-prepare.json"), b"stamp").unwrap();
        assert!(cache(temp.path()).is_err());
        assert_eq!(fs::read(temp.path().join("horizon-resources.json")).unwrap(), b"old");
        assert!(temp.path().join("horizon-prepare.json").exists());
    }
    #[test]
    fn moves_owned_settings_and_mods_and_preserves_worlds() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir(root.join("horizon-mods")).unwrap();
        fs::write(root.join("horizon-mods/custom.hmod"), b"custom mod").unwrap();
        fs::write(root.join("horizon-player.txt"), b"Player").unwrap();
        fs::create_dir_all(root.join("saves/My horizon world")).unwrap();
        fs::write(root.join("saves/My horizon world/level.dat"), b"world bytes").unwrap();
        profile(root).unwrap();
        profile(root).unwrap();
        assert_eq!(fs::read(root.join("fre-mods/custom.hmod")).unwrap(), b"custom mod");
        assert_eq!(fs::read(root.join("fre-player.txt")).unwrap(), b"Player");
        assert_eq!(fs::read(root.join("saves/My horizon world/level.dat")).unwrap(), b"world bytes");
        assert!(!root.join("horizon-mods").exists());
    }
    #[test]
    fn conflicts_do_not_overwrite_or_partially_move_profiles() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("horizon-player.txt"), b"old").unwrap();
        fs::write(temp.path().join("horizon.txt"), b"settings").unwrap();
        fs::write(temp.path().join("fre-player.txt"), b"new").unwrap();
        assert!(profile(temp.path()).is_err());
        assert_eq!(fs::read(temp.path().join("fre-player.txt")).unwrap(), b"new");
        assert!(temp.path().join("horizon.txt").is_file());
        assert!(!temp.path().join("fre.txt").exists());
    }
    #[test]
    fn running_world_prevents_any_profile_migration() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("saves/World")).unwrap();
        fs::write(temp.path().join("saves/World/level.dat"), b"world").unwrap();
        fs::write(temp.path().join("horizon-player.txt"), b"old").unwrap();
        let held = fs::File::create(temp.path().join("saves/World/session.lock")).unwrap();
        held.try_lock().unwrap();
        assert!(profile(temp.path()).is_err());
        assert!(temp.path().join("horizon-player.txt").exists());
        assert!(!temp.path().join("fre-player.txt").exists());
    }
    #[cfg(unix)]
    #[test]
    fn refuses_links_without_touching_their_targets() {
        let temp = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink("/tmp", temp.path().join("horizon-mods")).unwrap();
        assert!(profile(temp.path()).is_err());
    }
}
