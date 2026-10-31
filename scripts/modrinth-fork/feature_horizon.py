"""Add Horizon (Rust) as a real, separate instance type in FRE LAUNCH.

Uses the official Horizon Windows launcher executable that the player chooses.
Horizon's CLI owns its versions, world folders, updates, and account checks.
Does not merge Horizon mods into Modrinth Java instance content.
"""
from pathlib import Path
import shutil

def exactly(path: Path, before: str, after: str):
    data = path.read_text(encoding="utf-8")
    matches = data.count(before)
    if matches != 1:
        raise RuntimeError(f"Horizon integration source anchor changed ({matches}): {path}: {before[:90]!r}")
    path.write_text(data.replace(before, after, 1), encoding="utf-8")

def apply(root: Path):
    here = Path(__file__).resolve().parent
    shutil.copyfile(here / "horizon_backend.rs", root / "apps/app/src/api/horizon.rs")
    shutil.copyfile(here / "HorizonRustPanel.vue", root / "apps/app-frontend/src/components/ui/HorizonRustPanel.vue")

    api = root / "apps/app/src/api/mod.rs"
    exactly(api, "pub mod files;\npub mod friends;", "pub mod files;\npub mod horizon;\npub mod friends;")

    app_main = root / "apps/app/src/main.rs"
    exactly(app_main, "        .plugin(api::instance::init())",
            "        .plugin(api::instance::init())\n        .plugin(api::horizon::init())")

    tauri_build = root / "apps/app/build.rs"
    exactly(tauri_build,
            '            .plugin(\n                "instance",',
            '''            .plugin(
                "fre-horizon",
                InlinedPlugin::new()
                    .commands(&["horizon_instances", "horizon_create", "horizon_play"])
                    .default_permission(DefaultPermissionRule::AllowAllCommands),
            )
            .plugin(
                "instance",''')

    permissions = root / "apps/app/capabilities/plugins.json"
    exactly(permissions, '"instance:default",',
            '"instance:default",\n\t\t"fre-horizon:default",')

    wizard = root / "packages/ui/src/components/flows/creation-flow-modal/index.vue"
    exactly(wizard,
        "\t\tfetchExistingInstanceNames?: () => Promise<string[]>",
        "\t\tfetchExistingInstanceNames?: () => Promise<string[]>\n\t\tonHorizon?: () => void")
    exactly(wizard,
        "\t\tonBack: null,",
        "\t\tonBack: null,\n\t\tonHorizon: undefined,")
    exactly(wizard,
        "\t\tonBack: props.onBack ?? undefined,",
        "\t\tonBack: props.onBack ?? undefined,\n\t\tonHorizon: props.onHorizon,")

    ctx = root / "packages/ui/src/components/flows/creation-flow-modal/creation-flow-context.ts"
    exactly(ctx,
        "\tonBack: (() => void) | null",
        "\tonBack: (() => void) | null\n\tonHorizon: (() => void) | null")
    exactly(ctx,
        "\tonBack?: () => void",
        "\tonBack?: () => void\n\tonHorizon?: () => void")
    exactly(ctx,
        "\tconst onBack = options.onBack ?? null",
        "\tconst onBack = options.onBack ?? null\n\tconst onHorizon = options.onHorizon ?? null")
    exactly(ctx,
        "\t\tonBack,\n\t\treset,",
        "\t\tonBack,\n\t\tonHorizon,\n\t\treset,")

    stage = root / "packages/ui/src/components/flows/creation-flow-modal/components/SetupTypeStage.vue"
    exactly(stage,
            '''\t\t\t\t<BigOptionButton
\t\t\t\t\t:icon="CompassIcon"
\t\t\t\t\t:title="formatMessage(messages.modpackBaseTitle)"''',
            '''\t\t\t\t<BigOptionButton
\t\t\t\t\tv-if="ctx.onHorizon"
\t\t\t\t\t:icon="BoxesIcon"
\t\t\t\t\t:title="formatMessage(messages.horizonTitle)"
\t\t\t\t\t:description="formatMessage(messages.horizonDescription)"
\t\t\t\t\t@click="ctx.onHorizon()"
\t\t\t\t/>
\t\t\t\t<BigOptionButton
\t\t\t\t\t:icon="CompassIcon"
\t\t\t\t\t:title="formatMessage(messages.modpackBaseTitle)"''')
    exactly(stage,
            "\tcustomSetupTitle: {",
            '''\thorizonTitle: {
\t\tid: 'fre-launch.horizon.option.title',
\t\tdefaultMessage: 'Horizon (Rust)',
\t},
\thorizonDescription: {
\t\tid: 'fre-launch.horizon.option.description',
\t\tdefaultMessage: 'Separate native Rust Minecraft client and versions; runs through the official Horizon launcher.',
\t},
\tcustomSetupTitle: {''')

    app = root / "apps/app-frontend/src/App.vue"
    exactly(app,
        "import IconEditorModal from '@/components/ui/instance_settings/icon-editor-modal/index.vue'",
        "import HorizonRustPanel from '@/components/ui/HorizonRustPanel.vue'\n"
        "import IconEditorModal from '@/components/ui/instance_settings/icon-editor-modal/index.vue'")
    exactly(app,
        "const creationIconEditorModal = ref(null)",
        "const horizonWizard = ref(null)\nconst creationIconEditorModal = ref(null)")
    exactly(app,
        "const creationIconTarget = ref('creation-flow')",
        """const creationIconTarget = ref('creation-flow')

function showHorizonWizard() {
    installationModal.value?.hide()
    horizonWizard.value?.show()
}""")
    exactly(app,
        '\t\t\tref="installationModal"\n\t\t\ttype="instance"',
        '\t\t\tref="installationModal"\n\t\t\ttype="instance"\n\t\t\t:on-horizon="showHorizonWizard"')
    exactly(app,
        "\t\t<IconEditorModal\n\t\t\tref=\"creationIconEditorModal\"",
        "\t\t<HorizonRustPanel ref=\"horizonWizard\" />\n\t\t<IconEditorModal\n\t\t\tref=\"creationIconEditorModal\"")
    print("FRE LAUNCH Horizon Rust instance integration added to the Create Instance wizard.")
