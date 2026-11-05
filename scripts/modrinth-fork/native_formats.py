"""Read existing signed native packages while writing FRE's own format names."""


def apply(name, text, patch):
    if name == 'crates/horizon-mods/src/manifest.rs':
        text = patch(text,
            'let toml = std::fs::read_to_string(path.join(MANIFEST));',
            '''let manifest_path = if path.join(MANIFEST).is_file() {
                path.join(MANIFEST)
            } else { path.join("horizon-mod.toml") };
            let toml = std::fs::read_to_string(manifest_path);''')
        text = patch(text,
            'let f = match zip.by_name(name) {',
            '''let actual = if name == MANIFEST && !zip.file_names().any(|file| file.is_ok_and(|name| name == MANIFEST)) {
                    "horizon-mod.toml"
                } else { name };
                let f = match zip.by_name(actual) {''')
        text = patch(text,
            'p.extension().is_some_and(|e| e == "hmod") || p.join(MANIFEST).is_file()',
            'p.extension().is_some_and(|e| e == "hmod") || p.join(MANIFEST).is_file() || p.join("horizon-mod.toml").is_file()')
    elif name == 'crates/horizon-mods/src/modpack.rs':
        text = patch(text,
            '        zip.by_name(MANIFEST)\n',
            '''        let manifest_name = if zip.file_names().any(|file| file.is_ok_and(|name| name == MANIFEST)) {
            MANIFEST
        } else { "horizon-modpack.toml" };
        zip.by_name(manifest_name)
''')
        text = patch(text, 'if name == MANIFEST || entry.is_dir() {',
            'if name == manifest_name || entry.is_dir() {')
        text = patch(text, 'let target = home.join(name);',
            'let renamed = name.replacen("horizon-", "fre-", 1);\n            let target = home.join(renamed);')
        anchor = 'fn check_override(path: &str, manifest: &Manifest) -> Result<(), String> {\n'
        text = patch(text, anchor, anchor + '    let normalized = path.replacen("horizon-", "fre-", 1);\n    let path = normalized.as_str();\n')
    elif name == 'crates/horizon-mods/src/content.rs':
        text = patch(text, '!own && top == "data" && rest.starts_with("fre/")',
            '!own && top == "data" && (rest.starts_with("fre/") || rest.starts_with("horizon/"))')
        text = patch(text, '.then(|| rest.strip_prefix("fre/"))',
            '.then(|| rest.strip_prefix("fre/").or_else(|| rest.strip_prefix("horizon/")))')
    elif name == 'crates/horizon-mods/src/api_level.rs':
        text = patch(text, 'name.strip_prefix("fre:mods/")',
            'name.strip_prefix("fre:mods/").or_else(|| name.strip_prefix("horizon:mods/"))')
    elif name == 'crates/horizon-shader-api/src/package.rs':
        text = patch(text, '    path.join(MANIFEST).is_file()\n',
            '    (path.join(MANIFEST).is_file() || path.join("horizon-shaders.toml").is_file())\n')
        text = patch(text,
            '    pub fn from_files(files: BTreeMap<String, Vec<u8>>) -> Result<Pack, String> {\n',
            '''    pub fn from_files(mut files: BTreeMap<String, Vec<u8>>) -> Result<Pack, String> {
        if let Some(legacy) = files.remove("horizon-shaders.toml") {
            if files.contains_key(MANIFEST) {
                return Err("Multiple shader manifests in one FRE shader pack".into());
            }
            files.insert(MANIFEST.into(), legacy);
        }
''')
    return text
