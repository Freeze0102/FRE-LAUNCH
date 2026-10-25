"""Allow saved nickname-only Minecraft profiles to keep and use local PNG skins.

Offline Java sessions have no signed Mojang texture property. The actual game
texture can be displayed by an optional client-side mod such as CustomSkinLoader.
Never make a request to Mojang skin-write APIs with a local account.
"""
from pathlib import Path


def exactly(path, before, after):
    content=path.read_text(encoding="utf-8")
    matches=content.count(before)
    if matches!=1:
        raise RuntimeError(f"Source changed in {path}; expected one anchor, got {matches}: {before[:100]!r}")
    path.write_text(content.replace(before,after,1),encoding="utf-8")


def apply(root: Path):
    api=root/"packages/app-lib/src/api/offline_skins.rs"
    if api.exists():
        raise RuntimeError("Offline skin backend already exists")
    api.write_text(r"""//! Per-account client-side skins for FRE LAUNCH local/offline Minecraft profiles.
//! This never calls Mojang's skin-write endpoints or pretends that offline
//! profiles have a signed Minecraft Services texture.

use crate::{State, state::Credentials};
use anyhow::{Context, bail};
use base64::Engine;
use std::path::Path;
use uuid::Uuid;

const MAX_PNG_BYTES: usize = 512 * 1024;

fn local_skin_path(settings_dir: &Path, id: Uuid) -> std::path::PathBuf {
    settings_dir
        .join("fre-local-skins")
        .join(format!("{}.png", id.simple()))
}

fn validate_skin_png(bytes: &[u8]) -> crate::Result<()> {
    if bytes.len() > MAX_PNG_BYTES || bytes.len() < 24 {
        bail!("Minecraft skin PNG must be no larger than 512 KiB");
    }
    // Fully decode, not just inspect the PNG header.
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .context("Skin is not a valid PNG")?;
    if image.width() != 64 || !matches!(image.height(), 32 | 64) {
        bail!("Minecraft skin must be 64x64 or legacy 64x32 PNG");
    }
    Ok(())
}

async fn ensure_offline_account(id: Uuid, state: &State) -> crate::Result<()> {
    let all = Credentials::get_all(&state.pool).await?;
    if !all.get(&id).is_some_and(|entry| entry.is_offline_profile()) {
        bail!("Choose a local nickname profile before changing its local skin");
    }
    Ok(())
}

/// Saves a local skin under this account's UUID, not under the active Microsoft
/// credentials, and never modifies an instance while its game is running.
pub async fn save_local_skin(id: Uuid, encoded: &str) -> crate::Result<()> {
    let state = State::get().await?;
    ensure_offline_account(id, &state).await?;
    if encoded.len() > MAX_PNG_BYTES * 4 / 3 + 16 {
        bail!("Minecraft skin file is too large");
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .context("Invalid base64 skin data")?;
    validate_skin_png(&bytes)?;
    let path = local_skin_path(&state.directories.settings_dir, id);
    let parent = path.parent().context("Invalid local skin directory")?;
    tokio::fs::create_dir_all(parent).await?;
    let temp = parent.join(format!(".{}.tmp", Uuid::new_v4().simple()));
    tokio::fs::write(&temp, &bytes).await?;
    if let Err(error) = tokio::fs::rename(&temp, &path).await {
        // Windows rename cannot replace an existing file.
        if tokio::fs::try_exists(&path).await? {
            tokio::fs::remove_file(&path).await?;
            tokio::fs::rename(&temp, &path).await?;
        } else {
            let _ = tokio::fs::remove_file(&temp).await;
            return Err(error.into());
        }
    }
    Ok(())
}

pub async fn has_local_skin(id: Uuid) -> crate::Result<bool> {
    let state = State::get().await?;
    ensure_offline_account(id, &state).await?;
    Ok(tokio::fs::try_exists(local_skin_path(&state.directories.settings_dir, id)).await?)
}

/// Copy a saved PNG into standard client-side offline skin mod locations.
/// A compatible client-side mod (for example CustomSkinLoader) must be installed
/// by the player. Vanilla Minecraft does not honor unsigned local textures.
pub async fn prepare_instance_skin(
    credentials: &Credentials,
    instance_path: &Path,
    settings_dir: &Path,
) -> crate::Result<()> {
    if !credentials.is_offline_profile() {
        return Ok(());
    }
    let name = &credentials.offline_profile.name;
    if name.is_empty() || name.len() > 16 ||
        !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_')
    {
        bail!("Invalid local nickname for a skin filename");
    }
    let file = local_skin_path(settings_dir, credentials.offline_profile.id);
    if !tokio::fs::try_exists(&file).await? {
        return Ok(());
    }
    let bytes = tokio::fs::read(&file).await?;
    validate_skin_png(&bytes)?;

    // CustomSkinLoader's default LocalSkin/Legacy path.
    // Do not overwrite the instance's settings or install any mod without consent.
    let custom_skin_loader = instance_path
        .join("CustomSkinLoader").join("LocalSkin").join("skins");
    tokio::fs::create_dir_all(&custom_skin_loader).await?;
    tokio::fs::write(custom_skin_loader.join(format!("{name}.png")), &bytes).await?;

    // Compatibility with community OfflineSkins/LocalSkins Fabric mods.
    for folder in ["offlineskins", "localskins"] {
        let directory = instance_path.join("config").join(folder);
        tokio::fs::create_dir_all(&directory).await?;
        tokio::fs::write(directory.join(format!("{name}.png")), &bytes).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_non_skin_and_oversized_images() {
        assert!(validate_skin_png(b"not a PNG").is_err());
        assert!(validate_skin_png(&vec![0; MAX_PNG_BYTES + 1]).is_err());
    }
}
""", encoding="utf-8")

    exactly(root/"packages/app-lib/src/api/mod.rs",
            "pub mod minecraft_auth;\n",
            "pub mod minecraft_auth;\npub mod offline_skins;\n")

    auth=root/"apps/app/src/api/auth.rs"
    exactly(auth,
            "            create_offline_user,\n",
            "            create_offline_user,\n            save_local_skin,\n            has_local_skin,\n")
    with auth.open("a",encoding="utf-8") as out:
        out.write("""

/// Saves a client-side PNG for a local nickname profile; NEVER uploads to Mojang.
#[tauri::command]
pub async fn save_local_skin(profile_id: uuid::Uuid, png_base64: String) -> Result<()> {
    Ok(theseus::api::offline_skins::save_local_skin(profile_id, &png_base64).await?)
}

#[tauri::command]
pub async fn has_local_skin(profile_id: uuid::Uuid) -> Result<bool> {
    Ok(theseus::api::offline_skins::has_local_skin(profile_id).await?)
}
""")
    exactly(root/"apps/app/build.rs",
            '                        "create_offline_user",\n',
            '                        "create_offline_user",\n                        "save_local_skin",\n                        "has_local_skin",\n')

    launcher=root/"packages/app-lib/src/launcher/mod.rs"
    exactly(launcher,
            '    let instance_path =\n        get_instance_full_path(&instance.path, &state.directories)\n            .context("resolving instance path")?;\n\n    let (minecraft, version_index) =',
            '    let instance_path =\n        get_instance_full_path(&instance.path, &state.directories)\n            .context("resolving instance path")?;\n\n'
            '    // FRE LAUNCH: stage a local nickname PNG for optional client-side\n'
            '    // skin mods. No Mojang authentication or game-jar modification.\n'
            '    crate::api::offline_skins::prepare_instance_skin(\n'
            '        credentials, &instance_path, &state.directories.settings_dir,\n'
            '    ).await.context("preparing the local profile skin")?;\n\n'
            '    let (minecraft, version_index) =')

    skins=root/"apps/app-frontend/src/pages/Skins.vue"
    exactly(skins,"import { type DragDropEvent, getCurrentWebview } from '@tauri-apps/api/webview'\n",
            "import { type DragDropEvent, getCurrentWebview } from '@tauri-apps/api/webview'\n"
            "import { invoke } from '@tauri-apps/api/core'\n")
    exactly(skins,
            "const addSkinFileInput = useTemplateRef<HTMLInputElement>('addSkinFileInput')",
            "const addSkinFileInput = useTemplateRef<HTMLInputElement>('addSkinFileInput')\n"
            "const localSkinInput = useTemplateRef<HTMLInputElement>('localSkinInput')\n"
            "const localSkinSaved = ref(false)")
    exactly(skins,
            "function changeSkin(newSkin: Skin) {\n\tif (isSkinManagementReadOnly.value) return",
            "function changeSkin(newSkin: Skin) {\n\tif (isSkinManagementReadOnly.value && !isLocalAccount.value) return")
    exactly(skins,
            "\t\tisApplyingSkin.value ||\n\t\tisSkinManagementReadOnly.value\n",
            "\t\tisApplyingSkin.value ||\n\t\t(isSkinManagementReadOnly.value && !isLocalAccount.value)\n")
    exactly(skins,
            "\tisApplyingSkin.value = true\n\ttry {\n\t\tawait equip_skin(skinToApply)\n",
            "\tisApplyingSkin.value = true\n\ttry {\n"
            "\t\tif (isLocalAccount.value) {\n"
            "\t\t\tconst texture = await get_normalized_skin_texture(skinToApply)\n"
            "\t\t\tconst pngBase64 = texture.split(',')[1]\n"
            "\t\t\tif (!pngBase64 || !currentUserId.value) throw new Error('Select a local Minecraft profile')\n"
            "\t\t\tawait invoke('plugin:auth|save_local_skin', {\n"
            "\t\t\t\tprofileId: currentUserId.value,\n"
            "\t\t\t\tpngBase64,\n"
            "\t\t\t})\n"
            "\t\t\tlocalSkinSaved.value = true\n"
            "\t\t\tsetLocallyEquippedSkin(skinToApply)\n"
            "\t\t\treturn\n"
            "\t\t}\n"
            "\t\tawait equip_skin(skinToApply)\n")
    exactly(skins,
            "\t\tcurrentUser.value = allAccounts.find((acc) => acc.profile.id === defaultId)\n",
            "\t\tcurrentUser.value = allAccounts.find((acc) => acc.profile.id === defaultId)\n"
            "\t\tlocalSkinSaved.value = !!(currentUser.value?.access_token?.startsWith('fre-offline:') && defaultId &&\n"
            "\t\t\tawait invoke<boolean>('plugin:auth|has_local_skin', { profileId: defaultId }))\n")
    exactly(skins,
            "function openAddSkinFileBrowser() {\n\tif (isSkinManagementReadOnly.value) return\n",
            "function openAddSkinFileBrowser() {\n\tif (isLocalAccount.value) {\n"
            "\t\tlocalSkinInput.value?.click()\n"
            "\t\treturn\n"
            "\t}\n"
            "\tif (isSkinManagementReadOnly.value) return\n")
    exactly(skins,
            "function isSkinImagePath(path: string) {",
            """async function onLocalSkinFileSelected(e: Event) {
    const input = e.target as HTMLInputElement
    const file = input.files?.[0]
    input.value = ''
    if (!file || !isLocalAccount.value || !currentUserId.value) return
    if (file.size > 512 * 1024) {
        handleError(new Error('PNG must be no larger than 512 KiB'))
        return
    }
    try {
        // Chunk conversion so even a large valid PNG cannot overflow JS arg stack.
        const bytes = new Uint8Array(await file.arrayBuffer())
        let binary = ''
        for (let i = 0; i < bytes.length; i += 8192) {
            binary += String.fromCharCode(...bytes.subarray(i, i + 8192))
        }
        const pngBase64 = btoa(binary)
        await invoke('plugin:auth|save_local_skin', {
            profileId: currentUserId.value,
            pngBase64,
        })
        localSkinSaved.value = true
        addNotification({
            type: 'success',
            title: formatMessage(messages.localSkinSaved),
            text: formatMessage(messages.localSkinNeedsMod),
        })
    } catch (error) {
        handleError(error as Error)
    }
}

function isSkinImagePath(path: string) {""")
    exactly(skins,
            '\t\t:demo="!currentUser || isLocalAccount"\n',
            '\t\t:demo="!currentUser || isLocalAccount"\n')
    exactly(skins,
            '\t\tref="addSkinFileInput"\n',
            '\t\tref="addSkinFileInput"\n')  # Validate ordering of existing file input.
    exactly(skins,
            '\t<ConfirmModal\n\t\tref="deleteSkinModal"',
            '\t<input\n'
            '\t\tref="localSkinInput"\n'
            '\t\ttype="file"\n'
            '\t\taccept=".png,image/png"\n'
            '\t\tclass="hidden"\n'
            '\t\t@change="onLocalSkinFileSelected"\n'
            '\t/>\n'
            '\t<ConfirmModal\n\t\tref="deleteSkinModal"')
    exactly(skins,
            '\t\t\t<p v-if="isLocalAccount" class="mt-3 rounded-lg border border-solid border-surface-5 bg-button-bg p-3 text-sm leading-5 text-secondary">\n'
            '\t\t\t\t{{ formatMessage(messages.localNicknameSkinNote) }}\n'
            '\t\t\t</p>',
            '\t\t\t<div v-if="isLocalAccount" class="mt-3 rounded-lg border border-solid border-surface-5 bg-button-bg p-3 text-sm leading-5 text-secondary">\n'
            '\t\t\t\t<p class="m-0">{{ formatMessage(messages.localNicknameSkinNote) }}</p>\n'
            '\t\t\t\t<p v-if="localSkinSaved" class="mt-2 text-brand font-semibold">{{ formatMessage(messages.localSkinSaved) }}</p>\n'
            '\t\t\t\t<Button type="colored" color="brand" class="mt-3" @click="localSkinInput?.click()">\n'
            '\t\t\t\t\t{{ formatMessage(messages.localUpload) }}\n'
            '\t\t\t\t</Button>\n'
            '\t\t\t\t<p class="mt-2 mb-0">{{ formatMessage(messages.localSkinNeedsMod) }}</p>\n'
            '\t\t\t\t<a href="https://modrinth.com/mod/customskinloader" target="_blank" rel="noopener noreferrer" class="text-link">CustomSkinLoader (Modrinth)</a>\n'
            '\t\t\t</div>')
    exactly(skins,
            '\t\t\t\t\t:disabled="!currentUser || isApplyingSkin || isSkinManagementReadOnly"\n',
            '\t\t\t\t\t:disabled="!currentUser || isApplyingSkin || (isSkinManagementReadOnly && !isLocalAccount)"\n')
    exactly(skins,
            '\t\t\t:is-add-skin-button-drag-active="isAddSkinButtonDragActive"\n',
            '\t\t\t:is-add-skin-button-drag-active="isAddSkinButtonDragActive"\n'
            '\t\t\t:allow-offline-selection="isLocalAccount"\n')
    exactly(skins,
            "\tdemoTitle: {\n",
            "\tlocalUpload: { id: 'fre-launch.skins.local-upload', defaultMessage: 'Choose PNG skin' },\n"
            "\tlocalSkinSaved: { id: 'fre-launch.skins.local-saved', defaultMessage: 'Local skin saved for this nickname' },\n"
            "\tlocalSkinNeedsMod: { id: 'fre-launch.skins.local-mod', defaultMessage: 'For the skin to appear in Minecraft, install a compatible client-side skin mod (such as CustomSkinLoader) in this game instance. Vanilla Minecraft does not show unsigned offline skins. Other players may not see it.' },\n"
            "\tdemoTitle: {\n")
    virtual=root/"apps/app-frontend/src/components/ui/skin/VirtualSkinSectionList.vue"
    exactly(virtual,
            "\treadOnly?: boolean\n",
            "\treadOnly?: boolean\n\tallowOfflineSelection?: boolean\n")
    content=virtual.read_text(encoding="utf-8")
    count=content.count(':disabled="readOnly"')
    if count != 4:
        raise RuntimeError(f"VirtualSkinSectionList selection controls changed: {count}")
    virtual.write_text(content.replace(':disabled="readOnly"',':disabled="readOnly && !allowOfflineSelection"'),encoding="utf-8")
    print("Added per-local-nickname skin PNG saving and game-instance CustomSkinLoader/OfflineSkins integration")
