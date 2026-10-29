"""FRE LAUNCH startup branding and independent Discord activity identity.

The upstream Discord application ID and its brand assets MUST NOT be used by
this fork. Rich Presence is off unless a distinct FRE LAUNCH Discord application
ID is explicitly supplied at build time. Discord's automatic game detection is
controlled separately by the Discord desktop client.
"""
from pathlib import Path


def exactly(path: Path, before: str, after: str):
    data = path.read_text(encoding="utf-8")
    count = data.count(before)
    if count != 1:
        raise RuntimeError(f"FRE LAUNCH branding patch anchor changed ({count}): {path}: {before[:100]!r}")
    path.write_text(data.replace(before, after, 1), encoding="utf-8")


def apply(root: Path):
    splash = root / "apps/app-frontend/src/components/ui/SplashScreen.vue"
    text = splash.read_text(encoding="utf-8")
    begin = "\t\t\t\t<svg\n"
    end = "\t\t\t\t</svg>\n"
    if text.count(begin) != 1 or text.count(end) != 1:
        raise RuntimeError("Splash SVG layout changed; refusing to leave upstream logo in FRE LAUNCH")
    start = text.index(begin)
    finish = text.index(end, start) + len(end)
    text = (text[:start]
            + '\t\t\t\t<div class="app-logo" aria-label="FRE LAUNCH">FRE <span>LAUNCH</span></div>\n'
            + text[finish:])
    old_css = """.app-logo {
\theight: 2.25rem;
\twidth: fit-content;
}"""
    new_css = """.app-logo {
\tdisplay: flex;
\talign-items: center;
\tgap: 0.5rem;
\tfont-size: 2rem;
\tfont-weight: 800;
\tletter-spacing: 0.08em;
\tline-height: 1.2;
}

.app-logo span {
\tcolor: var(--color-brand);
}"""
    if text.count(old_css) != 1:
        raise RuntimeError("Splash logo CSS layout changed")
    text = text.replace(old_css, new_css, 1)
    for asset in (
        "url('@/assets/loading/cube.png')",
        "url('@/assets/loading/cube-light.webp')",
    ):
        if text.count(asset) != 1:
            raise RuntimeError(f"Unexpected branded startup background: {asset}")
        text = text.replace(asset, "none", 1)
    splash.write_text(text, encoding="utf-8")

    discord = root / "packages/app-lib/src/state/discord.rs"
    exactly(discord,
            "use std::sync::{Arc, atomic::AtomicBool};",
            """use std::sync::{Arc, atomic::AtomicBool};

// A separately registered FRE LAUNCH Discord app may be configured at build
// time. Without it, do not connect to Modrinth's Rich Presence application.
const FRE_DISCORD_APPLICATION_ID: Option<&str> = option_env!("FRE_LAUNCH_DISCORD_APP_ID");""")
    exactly(discord,
            "    activity::{Activity, Assets},",
            "    activity::Activity,")
    exactly(discord,
            'let dipc = DiscordIpcClient::new("1123683254248148992");',
            'let dipc = DiscordIpcClient::new(FRE_DISCORD_APPLICATION_ID.unwrap_or("0"));')
    exactly(discord,
            """    pub async fn retry_if_not_ready(&self) -> bool {
        let mut client = self.client.write().await;""",
            """    pub async fn retry_if_not_ready(&self) -> bool {
        // Never use the upstream Discord application identity, even when the
        // existing user settings still have Discord RPC enabled.
        if FRE_DISCORD_APPLICATION_ID.is_none() {
            return false;
        }
        let mut client = self.client.write().await;""")
    exactly(discord,
            """        let activity = Activity::new().state(msg).assets(
            Assets::new()
                .large_image("modrinth_simple")
                .large_text("Modrinth Logo"),
        );""",
            "        let activity = Activity::new().state(msg);")

    cargo = root / "apps/app/Cargo.toml"
    exactly(cargo,
            'description = "The Modrinth App is a desktop application for managing your Minecraft mods"',
            'description = "FRE LAUNCH is a desktop Minecraft launcher and mod manager"')
    print("FRE LAUNCH splash replaced; upstream Discord Rich Presence identity disabled by default.")
