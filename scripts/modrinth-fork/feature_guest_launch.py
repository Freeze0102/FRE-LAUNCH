"""FRE LAUNCH TLauncher-style unchecked account mode: ephemeral nickname launches.

No Minecraft account is created, persisted, refreshed or deleted. The guest
credentials are passed only to the currently requested Minecraft launch.
Online-mode/authenticated servers still require real Microsoft credentials.
"""
from pathlib import Path


def exactly(path: Path, before: str, after: str):
    text = path.read_text(encoding="utf-8")
    count = text.count(before)
    if count != 1:
        raise RuntimeError(f"Guest launch patch anchor changed ({count}): {path}: {before[:90]!r}")
    path.write_text(text.replace(before, after, 1), encoding="utf-8")


def apply(root: Path):
    auth = root/"packages/app-lib/src/api/minecraft_auth.rs"
    with auth.open("a", encoding="utf-8") as file:
        file.write(r"""

/// Ephemeral nickname-only credentials. Never upserted into the account DB.
/// The UUID uses Minecraft's established OfflinePlayer:<name> algorithm.
pub fn guest_credentials(name: &str) -> crate::Result<Credentials> {
    use anyhow::bail;
    let name = name.trim();
    if name.is_empty() || name.len() > 16 ||
        !name.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        bail!("Guest nickname must use 1-16 ASCII letters, digits, or underscores");
    }
    let mut bytes = md5::compute(format!("OfflinePlayer:{name}").as_bytes()).0;
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let id = uuid::Uuid::from_bytes(bytes);
    Ok(Credentials {
        offline_profile: crate::state::MinecraftProfile {
            id, name: name.to_owned(), ..Default::default()
        },
        access_token: format!("fre-offline:{}", id.simple()),
        refresh_token: "fre-offline".to_owned(),
        expires: chrono::DateTime::<chrono::Utc>::from_timestamp(4_102_444_800, 0)
            .expect("Year 2100 UTC should be representable"),
        active: false,
    })
}
""")

    runner = root/"packages/app-lib/src/api/instance/run.rs"
    exactly(runner,
        """#[tracing::instrument]
pub async fn run(
    instance_id: &str,
    quick_play_type: QuickPlayType,
) -> crate::Result<ProcessMetadata> {
    let state = State::get().await?;""",
        """#[tracing::instrument]
pub async fn run(
    instance_id: &str,
    quick_play_type: QuickPlayType,
) -> crate::Result<ProcessMetadata> {
    run_internal(instance_id, quick_play_type, None).await
}

/// Launches as an unsaved Minecraft nickname, preserving the selected account.
/// No record is added to the user's saved accounts.
#[tracing::instrument(skip(nickname))]
pub async fn run_guest(
    instance_id: &str,
    quick_play_type: QuickPlayType,
    nickname: &str,
) -> crate::Result<ProcessMetadata> {
    let guest = crate::minecraft_auth::guest_credentials(nickname)?;
    run_internal(instance_id, quick_play_type, Some(guest)).await
}

async fn run_internal(
    instance_id: &str,
    quick_play_type: QuickPlayType,
    guest_credentials: Option<Credentials>,
) -> crate::Result<ProcessMetadata> {
    let state = State::get().await?;""")
    exactly(runner,
        """    let default_account = Credentials::get_default_credential(&state.pool)
        .await?
        .ok_or(ErrorKind::NotLoggedIn)?;

    run_credentials(instance_id, &default_account, quick_play_type).await""",
        """    let credentials = match guest_credentials {
        Some(guest) => guest,
        None => Credentials::get_default_credential(&state.pool)
            .await?
            .ok_or(ErrorKind::NotLoggedIn)?,
    };

    run_credentials(instance_id, &credentials, quick_play_type).await""")
    exactly(runner,
        """    if let Some(project_id) = server_play_project_id(&context.link)
        && !project_id.trim().is_empty()
    {""",
        """    if let Some(project_id) = server_play_project_id(&context.link)
        && !project_id.trim().is_empty()
        && !credentials.is_offline_profile()
    {""")
    exactly(runner,
        """    crate::minecraft_skins::flush_pending_skin_change().await?;""",
        """    // Never send an unrelated saved Microsoft user's pending skin change
    // as part of an anonymous nickname-only game launch.
    if !credentials.is_offline_profile() {
        crate::minecraft_skins::flush_pending_skin_change().await?;
    }""")

    api = root/"packages/app-lib/src/api/instance.rs"
    exactly(api, "QuickPlayType, kill, run, try_update_playtime_by_instance_id,",
           "QuickPlayType, kill, run, run_guest, try_update_playtime_by_instance_id,")

    ui = root/"apps/app/src/api/instance.rs"
    exactly(ui, "            instance_run,\n",
            "            instance_run,\n            instance_run_guest,\n")
    exactly(ui,
        """#[tauri::command]
pub async fn instance_kill(instance_id: &str) -> Result<()> {""",
        """/// Run Minecraft under a one-off offline nickname without saving an account.
#[tauri::command]
pub async fn instance_run_guest(
    instance_id: &str,
    nickname: String,
    server_address: Option<String>,
) -> Result<ProcessMetadata> {
    let quick_play = match server_address {
        Some(addr) => QuickPlayType::Server(ServerAddress::Unresolved(addr)),
        None => QuickPlayType::None,
    };
    Ok(theseus::instance::run_guest(instance_id, quick_play, &nickname).await?)
}

#[tauri::command]
pub async fn instance_kill(instance_id: &str) -> Result<()> {""")
    exactly(root/"apps/app/build.rs",
        '                        "instance_run",\n',
        '                        "instance_run",\n                        "instance_run_guest",\n')

    frontend = root/"apps/app-frontend/src/helpers/instance.ts"
    exactly(frontend,
        """\treturn await invoke('plugin:instance|instance_run', { instanceId, serverAddress })""",
        """\tif (localStorage.getItem('fre-launch-account-mode') === 'nickname') {
        const nickname = (localStorage.getItem('fre-launch-guest-nickname') ?? '').trim()
        if (!/^[A-Za-z0-9_]{1,16}$/.test(nickname)) {
            throw new Error('Enter a nickname (1-16 English letters, digits or underscores)')
        }
        const result = await invoke('plugin:instance|instance_run_guest', {
            instanceId, serverAddress, nickname,
        })
        // Remember the text of recent nicks, not auth records or tokens.
        let recent: string[] = []
        try {
            const parsed = JSON.parse(localStorage.getItem('fre-launch-guest-history') ?? '[]')
            if (Array.isArray(parsed)) recent = parsed.filter((name): name is string =>
                typeof name === 'string' && /^[A-Za-z0-9_]{1,16}$/.test(name))
        } catch { /* optional history */ }
        localStorage.setItem('fre-launch-guest-history',
            JSON.stringify([nickname, ...recent.filter((name) => name !== nickname)].slice(0, 12)))
        return result
    }
\treturn await invoke('plugin:instance|instance_run', { instanceId, serverAddress })""")

    card = root/"apps/app-frontend/src/features/minecraft-accounts/accounts-card.vue"
    # Account-mode switch is visible even if there are zero saved accounts.
    exactly(card, '<template>\n', """<template>
    <div class="mt-2 rounded-xl border border-solid border-surface-5 bg-button-bg p-3">
        <label class="flex items-center gap-2 text-sm text-primary cursor-pointer">
            <input v-model="useSavedAccount" type="checkbox" class="accent-brand" />
            <span>{{ formatMessage(messages.useAccount) }}</span>
        </label>
        <div v-if="!useSavedAccount" class="mt-2 flex flex-col gap-1">
            <label :for="guestNicknameId" class="text-xs text-secondary">
                {{ formatMessage(messages.guestNickname) }}
            </label>
            <input
                :id="guestNicknameId"
                v-model.trim="guestNickname"
                :list="guestNickListId"
                type="text"
                maxlength="16"
                pattern="[A-Za-z0-9_]{1,16}"
                autocomplete="off"
                spellcheck="false"
                :placeholder="formatMessage(messages.guestPlaceholder)"
                class="w-full min-w-0 rounded-lg border border-solid border-surface-5 bg-bg-raised px-3 py-2 text-sm text-contrast outline-none focus:border-brand"
            />
            <datalist :id="guestNickListId">
                <option v-for="name in guestNickSuggestions" :key="name" :value="name" />
            </datalist>
            <p class="m-0 text-xs text-secondary">
                {{ formatMessage(messages.guestNotice) }}
            </p>
            <p v-if="guestNickname && !/^[A-Za-z0-9_]{1,16}$/.test(guestNickname)" class="m-0 text-xs text-red">
                {{ formatMessage(messages.guestInvalid) }}
            </p>
        </div>
    </div>
""")
    exactly(card, '\t\tv-if="accounts.length === 0"', '\t\tv-if="useSavedAccount && accounts.length === 0"')
    exactly(card, "\t\tv-else\n\t\tclass=\"w-full mt-2 bg-button-bg border",
           "\t\tv-else-if=\"useSavedAccount\"\n\t\tclass=\"w-full mt-2 bg-button-bg border")
    exactly(card,
        '<OfflineProfileCreator v-if="accounts.length > 0" class="mt-2" />',
        '<OfflineProfileCreator v-if="useSavedAccount && accounts.length > 0" class="mt-2" />')
    exactly(card, "\t\t<OfflineProfileCreator />\n",
                 "\t\t<OfflineProfileCreator />\n") # validate existing no-accounts form
    exactly(card, "import { computed, onMounted, onUnmounted, ref } from 'vue'",
                 "import { computed, onMounted, onUnmounted, ref, useId, watch } from 'vue'")
    exactly(card,
        "const accounts: Ref<MinecraftCredential[]> = ref([])",
        """const accounts: Ref<MinecraftCredential[]> = ref([])
const guestNickSuggestions = computed(() => {
    let recent: string[] = []
    try {
        const saved = JSON.parse(localStorage.getItem('fre-launch-guest-history') ?? '[]')
        if (Array.isArray(saved)) recent = saved.filter((name): name is string =>
            typeof name === 'string' && /^[A-Za-z0-9_]{1,16}$/.test(name))
    } catch {
        // Ignore damaged nickname history.
    }
    const offline = accounts.value
        .filter((account) => account.access_token?.startsWith('fre-offline:'))
        .map((account) => account.profile.name)
    return Array.from(new Set([...recent, ...offline])).slice(0, 12)
})""")
    exactly(card,
        "const loginDisabled = ref(false)",
        """const loginDisabled = ref(false)
const guestNicknameId = useId()
const guestNickListId = useId()
const useSavedAccount = ref(localStorage.getItem('fre-launch-account-mode') !== 'nickname')
const guestNickname = ref(localStorage.getItem('fre-launch-guest-nickname') ?? '')
watch(useSavedAccount, (enabled) =>
    localStorage.setItem('fre-launch-account-mode', enabled ? 'account' : 'nickname'))
watch(guestNickname, (name) => localStorage.setItem('fre-launch-guest-nickname', name))""")
    exactly(card,
        "async function setAccount(account: MinecraftCredential) {\n",
        "async function setAccount(account: MinecraftCredential) {\n    useSavedAccount.value = true\n")
    exactly(card,
        "async function login() {\n\tloginDisabled.value = true",
        "async function login() {\n    useSavedAccount.value = true\n\tloginDisabled.value = true")
    exactly(card,
        "\tnotSignedIn: {\n",
        """    useAccount: { id: 'fre-launch.guest.use-account', defaultMessage: 'Use saved account' },
    guestNickname: { id: 'fre-launch.guest.nickname', defaultMessage: 'Nickname (without account)' },
    guestPlaceholder: { id: 'fre-launch.guest.placeholder', defaultMessage: 'Enter nickname' },
    guestNotice: { id: 'fre-launch.guest.notice', defaultMessage: 'No Microsoft login needed. For servers allowing non-premium players, use the same nickname you registered there and enter your server password when prompted.' },
    guestInvalid: { id: 'fre-launch.guest.invalid', defaultMessage: 'Use 1-16 English letters, numbers, or underscores.' },
\tnotSignedIn: {
""")
    print("Guest nickname mode added: unchecked account box launches directly without DB account row or Microsoft token")
