"""Keep local skin selected after reload and reflect uploaded PNG in avatar preview."""
from pathlib import Path

def exactly(path, old, new):
    s=path.read_text(encoding="utf-8")
    n=s.count(old)
    if n!=1:
        raise RuntimeError(f"FRE LAUNCH local skin preview anchor drift: {path} ({n} matches): {old[:100]!r}")
    path.write_text(s.replace(old,new,1),encoding="utf-8")

def apply(root: Path):
    rust=root/"packages/app-lib/src/api/offline_skins.rs"
    with rust.open("a",encoding="utf-8") as f:
        f.write(r"""

/// Read a saved offline skin for the owning local account, for local preview.
/// No Mojang authentication tokens or profiles are returned.
pub async fn get_local_skin(id: Uuid) -> crate::Result<Option<String>> {
    let state = State::get().await?;
    ensure_offline_account(id, &state).await?;
    let path = local_skin_path(&state.directories.settings_dir, id);
    if !tokio::fs::try_exists(&path).await? {
        return Ok(None);
    }
    let bytes = tokio::fs::read(&path).await?;
    validate_skin_png(&bytes)?;
    Ok(Some(base64::engine::general_purpose::STANDARD.encode(bytes)))
}
""")

    auth=root/"apps/app/src/api/auth.rs"
    exactly(auth,"            has_local_skin,\n","            has_local_skin,\n            get_local_skin,\n")
    with auth.open("a",encoding="utf-8") as f:
        f.write("""

/// Load a saved nickname skin for preview inside the launcher.
#[tauri::command]
pub async fn get_local_skin(profile_id: uuid::Uuid) -> Result<Option<String>> {
    Ok(theseus::offline_skins::get_local_skin(profile_id).await?)
}
""")
    exactly(root/"apps/app/build.rs",
            '                        "has_local_skin",\n',
            '                        "has_local_skin",\n                        "get_local_skin",\n')
    helper=root/"apps/app-frontend/src/helpers/skins.ts"
    # Local PNGs are already valid data URLs. Do not pass them into the Rust
    # URL-only downloader; this also makes the account avatar render correctly.
    exactly(helper,
        "export async function get_normalized_skin_texture(skin: Skin): Promise<string> {\n",
        "export async function get_normalized_skin_texture(skin: Skin): Promise<string> {\n"
        "\tif (skin.texture.startsWith('data:image/png;base64,')) return skin.texture\n")
    skins=root/"apps/app-frontend/src/pages/Skins.vue"
    exactly(skins,
        "async function loadSkins() {",
        """function skinFingerprint(base64: string): string {
    let checksum = 2166136261
    for (let i = 0; i < base64.length; i++) {
        checksum = Math.imul(checksum ^ base64.charCodeAt(i), 16777619)
    }
    return (checksum >>> 0).toString(16)
}

async function loadSkins() {""")
    exactly(skins,
            "\t\tconst loadedSkins = (await get_available_skins()) ?? []",
            """\t\tconst loadedSkins = (await get_available_skins()) ?? []
        // Load the nick's last selected PNG into the original skin preview.
        // This is local texture data, not a Mojang-authenticated upload.
        if (isLocalAccount.value && currentUserId.value) {
            const pngBase64 = await invoke<string | null>('plugin:auth|get_local_skin', {
                profileId: currentUserId.value,
            })
            if (pngBase64) {
                const localSkin: Skin = {
                    source: 'custom_external',
                    texture_key: 'fre-local-' + currentUserId.value + '-' + skinFingerprint(pngBase64),
                    texture: 'data:image/png;base64,' + pngBase64,
                    variant: 'CLASSIC',
                    is_equipped: true,
                    name: currentUser.value?.profile.name ?? 'Local skin',
                }
                for (const skin of loadedSkins) skin.is_equipped = false
                loadedSkins.unshift(localSkin)
            }
        }""")
    exactly(skins,
        "\t\t\tlocalSkinSaved.value = true\n\t\t\tsetLocallyEquippedSkin(skinToApply)",
        "\t\t\tlocalSkinSaved.value = true\n\t\t\tawait loadSkins()")
    exactly(skins,
        "        localSkinSaved.value = true\n        addNotification({",
        "        localSkinSaved.value = true\n        await loadSkins()\n        addNotification({")
    print("Launcher avatar and gallery remember last PNG for each local nickname across restarts")

if __name__=="__main__":
    raise SystemExit("Import through prepare.py")
