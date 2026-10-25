"""Compact offline skin controls and nickname-first profile switching UI."""
from pathlib import Path


def exactly(path: Path, before: str, after: str):
    body = path.read_text(encoding="utf-8")
    count = body.count(before)
    if count != 1:
        raise RuntimeError(f"FRE LAUNCH UI patch anchor changed ({count}): {path}: {before[:90]!r}")
    path.write_text(body.replace(before, after, 1), encoding="utf-8")


def apply(root: Path):
    skins = root/"apps/app-frontend/src/pages/Skins.vue"
    exactly(skins,
            "const localSkinSaved = ref(false)",
            """const localSkinSaved = ref(false)
// Keep the upload control visible, but collapse the long compatibility guide.
const localSkinHelpExpanded = ref(localStorage.getItem('fre-launch-local-skin-help') === '1')
watch(localSkinHelpExpanded, (expanded) => {
    localStorage.setItem('fre-launch-local-skin-help', expanded ? '1' : '0')
})""")
    old = """\t\t\t<div v-if="isLocalAccount" class="mt-3 rounded-lg border border-solid border-surface-5 bg-button-bg p-3 text-sm leading-5 text-secondary">
\t\t\t\t<p class="m-0">{{ formatMessage(messages.localNicknameSkinNote) }}</p>
\t\t\t\t<p v-if="localSkinSaved" class="mt-2 text-brand font-semibold">{{ formatMessage(messages.localSkinSaved) }}</p>
\t\t\t\t<Button type="colored" color="brand" class="mt-3" @click="localSkinInput?.click()">
\t\t\t\t\t{{ formatMessage(messages.localUpload) }}
\t\t\t\t</Button>
\t\t\t\t<p class="mt-2 mb-0">{{ formatMessage(messages.localSkinNeedsMod) }}</p>
\t\t\t\t<a href="https://modrinth.com/mod/customskinloader" target="_blank" rel="noopener noreferrer" class="text-link">CustomSkinLoader (Modrinth)</a>
\t\t\t</div>"""
    new = """\t\t\t<div v-if="isLocalAccount" class="mt-3 rounded-lg border border-solid border-surface-5 bg-button-bg p-2 text-sm text-secondary">
\t\t\t\t<div class="flex flex-wrap items-center gap-2">
\t\t\t\t\t<Button type="colored" color="brand" @click="localSkinInput?.click()">
\t\t\t\t\t\t{{ formatMessage(messages.localUpload) }}
\t\t\t\t\t</Button>
\t\t\t\t\t<button
\t\t\t\t\t\ttype="button"
\t\t\t\t\t\tclass="rounded-md border-0 bg-transparent px-2 py-2 text-secondary hover:text-contrast cursor-pointer"
\t\t\t\t\t\t:aria-expanded="localSkinHelpExpanded"
\t\t\t\t\t\t@click="localSkinHelpExpanded = !localSkinHelpExpanded"
\t\t\t\t\t>
\t\t\t\t\t\t{{ localSkinHelpExpanded ? formatMessage(messages.localHideHelp) : formatMessage(messages.localShowHelp) }}
\t\t\t\t\t</button>
\t\t\t\t</div>
\t\t\t\t<p v-if="localSkinSaved" class="m-0 mt-2 text-brand font-semibold">
\t\t\t\t\t{{ formatMessage(messages.localSkinSaved) }}
\t\t\t\t</p>
\t\t\t\t<div v-if="localSkinHelpExpanded" class="mt-2 border-0 border-t border-solid border-surface-5 pt-2">
\t\t\t\t\t<p class="m-0">{{ formatMessage(messages.localNicknameSkinNote) }}</p>
\t\t\t\t\t<p class="mt-2 mb-1">{{ formatMessage(messages.localSkinNeedsMod) }}</p>
\t\t\t\t\t<a
\t\t\t\t\t\thref="https://modrinth.com/mod/customskinloader"
\t\t\t\t\t\ttarget="_blank"
\t\t\t\t\t\trel="noopener noreferrer"
\t\t\t\t\t\tclass="text-link"
\t\t\t\t\t>CustomSkinLoader (Modrinth)</a>
\t\t\t\t</div>
\t\t\t</div>"""
    exactly(skins,old,new)
    exactly(skins,
        "\tlocalUpload: { id: 'fre-launch.skins.local-upload', defaultMessage: 'Choose PNG skin' },",
        "\tlocalUpload: { id: 'fre-launch.skins.local-upload', defaultMessage: 'Choose PNG skin' },\n"
        "\tlocalShowHelp: { id: 'fre-launch.skins.show-help', defaultMessage: 'How does this work?' },\n"
        "\tlocalHideHelp: { id: 'fre-launch.skins.hide-help', defaultMessage: 'Hide help' },")

    accounts = root/"apps/app-frontend/src/features/minecraft-accounts/accounts-card.vue"
    # Let players type a nickname without expanding the account list.
    exactly(accounts,
        "\t\t\t\t<OfflineProfileCreator />\n",
        "")
    exactly(accounts,
        "\t</Accordion>\n</template>",
        "\t</Accordion>\n"
        "\t<OfflineProfileCreator v-if=\"accounts.length > 0\" class=\"mt-2\" />\n"
        "</template>")
    # Distinct and explicit account type badges, not only a generic title.
    exactly(accounts,
        "{{ account.profile.name }} <span v-if=\"account.access_token?.startsWith('fre-offline:')\" class=\"text-xs text-secondary\">({{ formatMessage(messages.offline) }})</span>",
        """{{ account.profile.name }}
\t\t\t\t\t\t\t<span class="ml-1 rounded bg-bg-raised px-1.5 py-0.5 text-xs text-secondary">
\t\t\t\t\t\t\t\t{{ account.access_token?.startsWith('fre-offline:') ? formatMessage(messages.offline) : formatMessage(messages.microsoft) }}
\t\t\t\t\t\t\t</span>""")
    exactly(accounts,
        "\toffline: { id: 'fre-launch.offline.short', defaultMessage: 'Local' },",
        "\toffline: { id: 'fre-launch.offline.short', defaultMessage: 'Local' },\n"
        "\tmicrosoft: { id: 'fre-launch.offline.microsoft', defaultMessage: 'Microsoft' },\n"
        "\tconfirmRemove: { id: 'fre-launch.offline.confirm-remove', defaultMessage: 'Remove Minecraft profile {nickname} from this launcher?' },")
    exactly(accounts,
        "async function logout(id: string) {\n\tawait remove_user(id).catch(handleError)",
        """async function logout(id: string) {
    const account = accounts.value.find((value) => value.profile.id === id)
    if (!account || !window.confirm(formatMessage(messages.confirmRemove, { nickname: account.profile.name }))) return
\tawait remove_user(id).catch(handleError)""")
    # Existing creator chooses existing saved nickname (Rust backend) on submit.
    creator = root/"apps/app-frontend/src/features/minecraft-accounts/offline-profile-creator.vue"
    exactly(creator,
        "defaultMessage: 'Create'",
        "defaultMessage: 'Use nickname'")
    exactly(creator,
        "const nickname = ref('')",
        "const nickname = ref(localStorage.getItem('fre-launch-last-nickname') ?? '')")
    exactly(creator,
        "        await create_offline_user(nickname.value)\n        nickname.value = ''",
        """        await create_offline_user(nickname.value)
        localStorage.setItem('fre-launch-last-nickname', nickname.value)
        nickname.value = ''""")
    print("FRE LAUNCH skin guide collapsible, local PNG upload always visible; nick accounts one-click switch, clearly marked and protected from accidental deletion")
