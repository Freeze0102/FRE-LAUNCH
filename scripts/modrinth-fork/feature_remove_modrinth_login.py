"""Hide Modrinth platform-account sign-in UI in the FRE LAUNCH desktop fork.

Keep Modrinth's public API and download/catalog routes operational.
Keep optional upstream platform authorization internals intact for features
that inherently require an official Modrinth account.
"""
from pathlib import Path


def exactly(path: Path, before: str, after: str):
    data = path.read_text(encoding="utf-8")
    hits = data.count(before)
    if hits != 1:
        raise RuntimeError(f"Modrinth UI patch anchor drift ({hits}): {path}: {before[:95]!r}")
    path.write_text(data.replace(before, after, 1), encoding="utf-8")


def apply(root: Path):
    app = root / "apps/app-frontend/src/App.vue"
    # Remove Modrinth profile-switcher, plus upsell and signup controls from
    # bottom nav without disturbing Minecraft account selection on the sidebar.
    source = app.read_text(encoding="utf-8")
    start_anchor = '\t\t\t<IconButton\n\t\t\t\tv-if="credentials === undefined"'
    end_anchor = '\t\t</div>\n\t\t<AppTitleBar'
    if source.count(start_anchor) != 1 or source.count(end_anchor) != 1:
        raise RuntimeError("Could not locate upstream Modrinth account toolbar")
    start = source.index(start_anchor)
    end = source.index(end_anchor, start)
    source = source[:start] + source[end:]
    app.write_text(source, encoding="utf-8")

    sidebar = root / "apps/app-frontend/src/app/shell/sidebar/index.vue"
    # Friends list requires Modrinth sign-in and shouldn't show a login card.
    # Keep the component bindings compiled because the upstream app may still
    # expose its optional share/friend helpers elsewhere.
    exactly(sidebar,
        '\t\t\t\t\t\tv-show="showFriendsList"\n',
        '\t\t\t\t\t\tv-if="false && showFriendsList"\n')
    # Campaign/brand panels are not part of a Minecraft launcher.
    exactly(sidebar,
        '\t\t\t\t\t<PrideFundraiserBanner\n',
        '\t\t\t\t\t<PrideFundraiserBanner v-if="false"\n')
    exactly(sidebar,
        '\t\t\t\t\t<NewsSidebar :style="promotionPaddingStyle" />',
        '\t\t\t\t\t<NewsSidebar v-if="false" :style="promotionPaddingStyle" />')

    onboarding = root / "apps/app-frontend/src/features/onboarding/checklist.vue"
    exactly(onboarding,
        """\t{
\t\tid: 'login-minecraft',
\t\tlabel: formatMessage(messages.loginMinecraft),
\t\tcomplete: hasLoggedIntoMinecraft.value,
\t\taction: () => emit('login-minecraft'),
\t},
\t{
\t\tid: 'login-modrinth',
\t\tlabel: formatMessage(messages.loginModrinth),
\t\tcomplete: hasLoggedIntoModrinth.value,
\t\taction: () => emit('login-modrinth'),
\t},
""", "")
    exactly(onboarding,
        "\thasLoggedIntoMinecraft,\n\thasLoggedIntoModrinth,\n", "")
    # Don't keep asking for a Microsoft or Modrinth login. Show the quick
    # checklist only while the user has not created their first game instance.
    exactly(onboarding,
        'v-if="isReady && showChecklist"',
        'v-if="isReady && showChecklist && !hasCreatedInstance"')

    settings = root / "apps/app-frontend/src/features/settings/app-settings-modal/index.vue"
    # Upstream Account tabs are for the Modrinth online platform, not Minecraft.
    # Keep all display, language, Java, file and troubleshooting settings.
    exactly(settings,
        "\ttabs.filter((tab) => !tab.developerOnly || appSettings.devMode),",
        "\ttabs.filter((tab) => (!tab.developerOnly || appSettings.devMode) && tab.category !== tabCategories.account),")

    print("Hid Modrinth platform sign-in/profile upsell, social login prompts and account settings. Minecraft and public catalog retained.")


if __name__ == "__main__":
    raise SystemExit("Import through prepare.py")
