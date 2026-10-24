"""Patch the pinned Modrinth source with functional local Minecraft nick profiles."""
from pathlib import Path

def exactly(path, old, new):
    text=path.read_text(encoding="utf-8")
    count=text.count(old)
    if count!=1:
        raise RuntimeError(f"Upstream changed in {path}: count={count}, anchor={old[:85]!r}")
    path.write_text(text.replace(old,new,1),encoding="utf-8")

def apply(root):
    p=root/"packages/app-lib/src/api/minecraft_auth.rs"
    text=p.read_text(encoding="utf-8")
    if "pub async fn create_offline_user" in text:
        raise RuntimeError("Already contains local profiles")
    text+=r"""

/// A local player nickname only; never a Mojang/Microsoft authorization.
#[tracing::instrument]
pub async fn create_offline_user(name: &str) -> crate::Result<Credentials> {
    use anyhow::bail;
    let name = name.trim();
    if name.is_empty() || name.len() > 16 ||
        !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
    {
        bail!("Nickname must contain 1–16 English letters, digits or underscores");
    }

    let state = State::get().await?;
    let users = Credentials::get_all(&state.pool).await?;
    // Re-select a saved local account rather than producing a duplicate.
    let existing_id = users.iter()
        .find(|entry| entry.value().is_offline_profile() &&
                     entry.value().offline_profile.name.eq_ignore_ascii_case(name))
        .map(|entry| *entry.key());
    if let Some(id) = existing_id {
        if let Some((_, mut user)) = users.remove(&id) {
            user.active = true;
            user.upsert(&state.pool).await?;
            // A local player profile also completes the account setup step.
            // This does not claim that Microsoft authentication succeeded.
            if let Err(error) = crate::onboarding_checklist::mark_logged_into_minecraft().await {
                tracing::warn!("Could not update account setup checklist: {error}");
            }
            return Ok(user);
        }
    }

    // Minecraft offline UUID = UUID.nameUUIDFromBytes("OfflinePlayer:" + name).
    let mut bytes = md5::compute(format!("OfflinePlayer:{name}").as_bytes()).0;
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let id = uuid::Uuid::from_bytes(bytes);
    if users.contains_key(&id) {
        bail!("Nickname UUID is already used by another profile");
    }

    let user = Credentials {
        offline_profile: crate::state::MinecraftProfile {
            id,
            name: name.to_owned(),
            ..Default::default()
        },
        access_token: format!("fre-offline:{}", id.simple()),
        refresh_token: "fre-offline".to_owned(),
        expires: chrono::DateTime::<chrono::Utc>::from_timestamp(4_102_444_800, 0)
            .expect("Year 2100 UTC should be representable"),
        active: true,
    };
    user.upsert(&state.pool).await?;
    if let Err(error) = crate::onboarding_checklist::mark_logged_into_minecraft().await {
        tracing::warn!("Could not update account setup checklist: {error}");
    }
    Ok(user)
}
"""
    p.write_text(text,encoding="utf-8")
    exactly(root/"packages/app-lib/Cargo.toml",
        'uuid = { workspace = true, features = ["serde", "v4"] }',
        'uuid = { workspace = true, features = ["serde", "v4"] }\nmd5 = "0.8.0"')

    p=root/"packages/app-lib/src/state/minecraft_auth.rs"
    exactly(p,"impl Credentials {\n    /// Refreshes the authentication tokens",
        "impl Credentials {\n    /// Local nickname profiles have no Microsoft authorization tokens.\n"
        "    pub fn is_offline_profile(&self) -> bool {\n"
        "        self.access_token.starts_with(\"fre-offline:\") && self.refresh_token == \"fre-offline\"\n"
        "    }\n\n    /// Refreshes the authentication tokens")
    exactly(p,"    ) -> crate::Result<()> {\n        // Use a margin of 5 minutes",
        "    ) -> crate::Result<()> {\n        if self.is_offline_profile() { return Ok(()); }\n"
        "        // Use a margin of 5 minutes")
    exactly(p,"    ) -> Option<Arc<MinecraftProfile>> {\n        let max_age = cache_intent.max_age();",
        "    ) -> Option<Arc<MinecraftProfile>> {\n        if self.is_offline_profile() { return None; }\n"
        "        let max_age = cache_intent.max_age();")

    p=root/"apps/app/src/api/auth.rs"
    exactly(p,"            get_users,\n","            get_users,\n            create_offline_user,\n")
    p.write_text(p.read_text(encoding="utf-8")+"""

/// Creates or selects a nickname-only account in the existing SQLite store.
#[tauri::command]
pub async fn create_offline_user(name: String) -> Result<Credentials> {
    Ok(minecraft_auth::create_offline_user(&name).await?)
}
""",encoding="utf-8")
    exactly(p,'.title("Sign into Modrinth")','.title("Sign in to Minecraft with Microsoft")')
    exactly(root/"apps/app/build.rs",'                        "get_users",\n',
        '                        "get_users",\n                        "create_offline_user",\n')

    p=root/"apps/app-frontend/src/helpers/auth.js"
    p.write_text(p.read_text(encoding="utf-8")+"""

/** Create or select an offline-only Minecraft nickname account. */
export async function create_offline_user(name) {
    return await invoke('plugin:auth|create_offline_user', { name })
}
""",encoding="utf-8")
    p=root/"packages/app-lib/src/launcher/args.rs"
    marker = '"'+chr(36)+'{user_type}"'
    exactly(p, '.replace('+marker+', "msa")',
        '.replace('+marker+', if access_token.starts_with("fre-offline:") { "legacy" } else { "msa" })')

    p=root/"apps/app-frontend/src/features/minecraft-accounts/offline-profile-creator.vue"
    p.write_text("""<template>
    <form
        class="flex flex-col gap-2 rounded-xl border border-solid border-surface-5 bg-button-bg p-3"
        @submit.prevent="createProfile"
    >
        <label for="fre-offline-nickname" class="text-sm font-semibold text-primary">
            {{ formatMessage(messages.title) }}
        </label>
        <div class="flex items-center gap-2">
            <input
                id="fre-offline-nickname"
                v-model.trim="nickname"
                type="text"
                autocomplete="off"
                autocapitalize="none"
                spellcheck="false"
                maxlength="16"
                pattern="[A-Za-z0-9_]{1,16}"
                :placeholder="formatMessage(messages.placeholder)"
                class="w-full min-w-0 rounded-lg border border-solid border-surface-5 bg-bg-raised px-3 py-2 text-sm text-contrast outline-none focus:border-brand"
                :disabled="busy"
            />
            <Button type="colored" color="brand" :disabled="busy || !valid" @click.prevent="createProfile">
                <SpinnerIcon v-if="busy" class="animate-spin" />
                <PlusIcon v-else />
                {{ formatMessage(messages.create) }}
            </Button>
        </div>
        <p class="m-0 text-xs leading-5 text-secondary">
            {{ formatMessage(messages.disclaimer) }}
        </p>
    </form>
</template>

<script setup lang="ts">
import { PlusIcon, SpinnerIcon } from '@modrinth/assets'
import { Button, defineMessages, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { computed, ref } from 'vue'
import { create_offline_user } from '@/helpers/auth'

const emit = defineEmits<{ created: [] }>()
const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()
const nickname = ref('')
const busy = ref(false)
const valid = computed(() => /^[A-Za-z0-9_]{1,16}$/.test(nickname.value))
async function createProfile() {
    if (!valid.value || busy.value) return
    busy.value = true
    try {
        await create_offline_user(nickname.value)
        nickname.value = ''
        emit('created')
    } catch (error) {
        handleError(error)
    } finally {
        busy.value = false
    }
}
const messages = defineMessages({
    title: { id: 'fre-launch.offline.title', defaultMessage: 'Local nickname' },
    placeholder: { id: 'fre-launch.offline.placeholder', defaultMessage: 'Your nickname' },
    create: { id: 'fre-launch.offline.create', defaultMessage: 'Create' },
    disclaimer: {
        id: 'fre-launch.offline.disclaimer',
        defaultMessage: 'For single-player and offline-mode servers only. Not a Microsoft account.',
    },
})
</script>
""",encoding="utf-8")
    p=root/"apps/app-frontend/src/features/minecraft-accounts/accounts-card.vue"
    exactly(p,'\t\t</Button>\n\t</div>\n\t<Accordion',
        '\t\t</Button>\n\t\t<OfflineProfileCreator @created="onOfflineCreated" />\n\t</div>\n\t<Accordion')
    exactly(p, '\t\t\t\t\t{{ formatMessage(messages.addAccount) }}\n\t\t\t\t</Button>\n',
        '\t\t\t\t\t{{ formatMessage(messages.addAccount) }}\n\t\t\t\t</Button>\n'
        '\t\t\t\t<OfflineProfileCreator @created="onOfflineCreated" />\n')
    exactly(p, "import type { Ref } from 'vue'",
        "import OfflineProfileCreator from './offline-profile-creator.vue'\nimport type { Ref } from 'vue'")
    exactly(p, "async function login() {\n",
        "async function onOfflineCreated() {\n\tawait refreshValues()\n\temit('change')\n}\n\nasync function login() {\n")
    p=root/"apps/app-frontend/src/app/shell/sidebar/index.vue"
    exactly(p,'\t\t\t\t\t\tv-show="hasLoggedIntoMinecraft"\n',"")

    # Explicitly mark local vs Microsoft accounts using a durable marker
    # already carried by Credentials; do not include auth token in the UI.
    p=root/"apps/app-frontend/src/features/minecraft-accounts/accounts-card.vue"
    exactly(p, "\tprofile: {\n\t\tid: string\n\t\tname: string\n\t}\n}",
        "\tprofile: {\n\t\tid: string\n\t\tname: string\n\t}\n\taccess_token?: string\n}")
    exactly(p, "{{ formatMessage(messages.minecraftAccount) }}",
        "{{ selectedAccount?.access_token?.startsWith('fre-offline:') ? formatMessage(messages.offline) : formatMessage(messages.minecraftAccount) }}")
    exactly(p, '{{ account.profile.name }}',
        '{{ account.profile.name }} <span v-if="account.access_token?.startsWith(\'fre-offline:\')" class="text-xs text-secondary">({{ formatMessage(messages.offline) }})</span>')
    exactly(p, "\tminecraftAccount: {\n",
        "\toffline: { id: 'fre-launch.offline.short', defaultMessage: 'Local' },\n\tminecraftAccount: {\n")

    with (root/"FRE_LAUNCH_FORK.md").open("a",encoding="utf-8") as f:
        f.write("\nFRE LAUNCH supports saved nickname-only profiles with Minecraft-compatible\n"
                "offline UUIDs. Such profiles do not authenticate to Minecraft Services,\n"
                "online-mode servers, or Microsoft. Official Microsoft login remains separate.\n")

if __name__=="__main__":
    raise SystemExit("This module must be imported from prepare.py")
