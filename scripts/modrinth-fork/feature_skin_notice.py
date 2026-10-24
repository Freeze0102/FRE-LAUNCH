"""Explain Microsoft-only skin sync for locally created nickname accounts.

Local nicknames are not authenticated Mojang users. Preserve the preview
experience while blocking remote skin write operations and misleading success.
"""
from pathlib import Path


def exactly(path, old, new):
    value=path.read_text(encoding="utf-8")
    hits=value.count(old)
    if hits!=1:
        raise RuntimeError(f"{path}: unexpected upstream skin UI ({hits} matches)")
    path.write_text(value.replace(old,new,1),encoding="utf-8")


def apply(root: Path):
    p=root/"apps/app-frontend/src/pages/Skins.vue"
    exactly(p,
        "const currentUserId = ref<string | undefined>(undefined)",
        "const currentUserId = ref<string | undefined>(undefined)\n"
        "const isLocalAccount = computed(() => Boolean(currentUser.value?.access_token?.startsWith('fre-offline:')))")
    exactly(p,
        "isSkinManagementReadOnly = computed(\n\t() =>\n\t\t!!currentUser.value &&\n\t\t(offline.value || (authServerQuery.isError.value && !authServerQuery.isLoading.value)),",
        "isSkinManagementReadOnly = computed(\n\t() =>\n\t\t!!currentUser.value &&\n"
        "\t\t(isLocalAccount.value || offline.value || (authServerQuery.isError.value && !authServerQuery.isLoading.value)),")
    exactly(p,
        "\t\t:demo=\"!currentUser\"",
        "\t\t:demo=\"!currentUser || isLocalAccount\"")
    exactly(p,
        "\t\t\t\t{{ formatMessage(appMessages.skinSelectorLabel) }}\n\t\t\t</h1>",
        "\t\t\t\t{{ formatMessage(appMessages.skinSelectorLabel) }}\n\t\t\t</h1>\n"
        "\t\t\t<p v-if=\"isLocalAccount\" class=\"mt-3 rounded-lg border border-solid border-surface-5 bg-button-bg p-3 text-sm leading-5 text-secondary\">\n"
        "\t\t\t\t{{ formatMessage(messages.localNicknameSkinNote) }}\n"
        "\t\t\t</p>")
    exactly(p,
        "\tdemoTitle: {\n",
        "\tlocalNicknameSkinNote: {\n"
        "\t\tid: 'fre-launch.skins.local-notice',\n"
        "\t\tdefaultMessage: 'This is a local nickname, not a Microsoft account. You can preview skins here, but applying an online skin requires a licensed Minecraft account.',\n"
        "\t},\n"
        "\tdemoTitle: {\n")
    print("Added offline-nickname skin notice and read-only Minecraft Services skin protection")
