"""Replace the old external Horizon Launcher integration with FRE LAUNCH's native Rust game.

The game executable is built locally and bundled as a Tauri resource. It is
not downloaded, launched or updated through any foreign launcher executable.
Minecraft's private game assets come from the user's own installation.
"""
import json
import hashlib
import shutil
from pathlib import Path

def exactly(path: Path, before: str, after: str):
    source = path.read_text(encoding="utf-8")
    occurrences = source.count(before)
    if occurrences != 1:
        raise RuntimeError(f"FRE Rust integration anchor changed ({occurrences}): {path}: {before[:90]!r}")
    path.write_text(source.replace(before, after, 1), encoding="utf-8")

def apply(root: Path):
    here = Path(__file__).resolve().parent
    app = root / "apps/app"
    frontend = root / "apps/app-frontend"

    shutil.copyfile(here / "fre_rust_backend.rs", app / "src/api/fre_rust.rs")
    shutil.copyfile(here / "FreRustPanel.vue", frontend / "src/components/ui/FreRustPanel.vue")
    shutil.copyfile(here / "FreRustLibrary.vue", frontend / "src/components/ui/FreRustLibrary.vue")

    # Existing Horizon profiles already work for some players. Keep their
    # launcher, profile folders and library card instead of silently replacing
    # them with a new incompatible native profile format.
    exactly(app / "src/api/mod.rs",
            "pub mod horizon;",
            "pub mod horizon;\npub mod fre_rust;")
    exactly(app / "src/main.rs",
            "        .plugin(api::horizon::init())",
            "        .plugin(api::horizon::init())\n        .plugin(api::fre_rust::init())")
    exactly(app / "build.rs",
            '            .plugin(\n                "fre-horizon",',
            '''            .plugin(
                "fre-rust",
                InlinedPlugin::new()
                    .commands(&["rust_profiles", "rust_create_profile", "rust_runtime", "rust_import_resources", "rust_launch", "rust_logs"])
                    .default_permission(DefaultPermissionRule::AllowAllCommands),
            )
            .plugin(
                "fre-horizon",''')
    exactly(app / "capabilities/plugins.json",
            '"fre-horizon:default",',
            '"fre-horizon:default",\n\t\t"fre-rust:default",')

    app_vue = frontend / "src/App.vue"
    exactly(app_vue,
            "import HorizonRustPanel from '@/components/ui/HorizonRustPanel.vue'",
            "import HorizonRustPanel from '@/components/ui/HorizonRustPanel.vue'\nimport FreRustPanel from '@/components/ui/FreRustPanel.vue'")
    exactly(app_vue, "const horizonWizard = ref(null)",
            "const horizonWizard = ref(null)\nconst freRustWizard = ref(null)")
    exactly(app_vue, "provide('showHorizonWizard', showHorizonWizard)",
            """provide('showHorizonWizard', showHorizonWizard)
function showFreRustWizard() {
    installationModal.value?.hide()
    freRustWizard.value?.show()
}
provide('showFreRustWizard', showFreRustWizard)""")
    # The new Create flow selects the built-in game, while existing Horizon
    # cards still open their original wizard through showHorizonWizard.
    exactly(app_vue, ':on-horizon="showHorizonWizard"',
            ':on-horizon="showFreRustWizard"')
    exactly(app_vue, '<HorizonRustPanel ref="horizonWizard" />',
            '<HorizonRustPanel ref="horizonWizard" />\n\t\t<FreRustPanel ref="freRustWizard" />')

    index = frontend / "src/pages/Index.vue"
    exactly(index,
            "import HorizonLibrary from '@/components/ui/HorizonLibrary.vue'",
            "import HorizonLibrary from '@/components/ui/HorizonLibrary.vue'\nimport FreRustLibrary from '@/components/ui/FreRustLibrary.vue'")
    exactly(index,
            '\t\t<HorizonLibrary />',
            '\t\t<HorizonLibrary />\n\t\t<FreRustLibrary />')
    exactly(index,
            '\t<HorizonLibrary v-if="isReady && !hasCreatedInstance" class="px-6 pb-6" />',
            '\t<HorizonLibrary v-if="isReady && !hasCreatedInstance" class="px-6 pb-6" />\n\t<FreRustLibrary v-if="isReady && !hasCreatedInstance" class="px-6 pb-6" />')

    stage = root / "packages/ui/src/components/flows/creation-flow-modal/components/SetupTypeStage.vue"
    exactly(stage, "defaultMessage: 'Horizon (Rust)'",
            "defaultMessage: 'Minecraft (Rust)'")
    exactly(stage,
            "defaultMessage: 'Separate native Rust Minecraft client and versions; FRE LAUNCH launches the game directly.'",
            "defaultMessage: 'An independent Rust game, installed and played inside FRE LAUNCH.'")

    # Preserve the older Horizon adapter and its UI so an update does not
    # strand already installed Rust profiles. No automatic world migration.
    # Bundle the native game only when the local build has prepared real
    # artifacts. Missing assets are an explicit Runtime: Not installed state
    # in the UI, NEVER a fake Play button.
    runtime = here / "fre-rust-runtime"
    binaries = ("fre-rust-client.exe", "research-lock.json", "LICENSE.txt")
    valid = all((runtime / f).is_file() for f in binaries) and (runtime / "BUILD-INFO.json").is_file()
    if valid:
        # These may be private Minecraft resources. Prevent symlink escapes
        # and detect incomplete, edited or mismatched native builds before
        # copying anything into the Tauri installer.
        for path in runtime.rglob("*"):
            if path.is_symlink():
                raise RuntimeError(f"Native Rust game runtime contains a link: {path}")
        info = json.loads((runtime / "BUILD-INFO.json").read_text(encoding="utf-8-sig"))
        if info.get("source_commit") != "975c07d9151847e6af13afc4b5ab37b6bbe78ef5":
            raise RuntimeError("Native Rust engine source revision is not approved.")
        hasher = hashlib.sha256()
        with (runtime / "fre-rust-client.exe").open("rb") as executable:
            for block in iter(lambda: executable.read(1024 * 1024), b""):
                hasher.update(block)
        checksum = hasher.hexdigest()
        if checksum.lower() != str(info.get("game_exe_sha256", "")).lower():
            raise RuntimeError("Native Rust game SHA-256 is wrong: rebuild; do not bundle.")
        if info.get("engine_license") != "MIT":
            raise RuntimeError("Native Rust game license metadata changed.")
        if not (runtime / "LICENSE.txt").read_text(encoding="utf-8").startswith("MIT License"):
            raise RuntimeError("Native Rust game attribution is missing.")
        staged = app / "fre-rust-runtime"
        if staged.exists():
            raise RuntimeError("Native Rust staging folder already exists; use a clean upstream checkout.")
        shutil.copytree(runtime, staged)
        conf = app / "tauri.conf.json"
        settings = json.loads(conf.read_text(encoding="utf-8"))
        resources = settings["bundle"].setdefault("resources", {})
        if not isinstance(resources, dict):
            raise RuntimeError("Unexpected Tauri bundle.resources configuration.")
        for file in binaries + ("BUILD-INFO.json",):
            resources[f"fre-rust-runtime/{file}"] = f"fre-rust/{file}"
        # PUBLIC releases carry the MIT engine only. Mojang resources are
        # imported locally by the authenticated owner after installation.
        if (runtime / "resources/horizon-resources.json").is_file():
            resources["fre-rust-runtime/resources/"] = "fre-rust/resources/"
        elif (runtime / "resources").exists():
            raise RuntimeError("Incomplete private Minecraft resource set.")
        conf.write_text(json.dumps(settings, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        print("Bundled native FRE LAUNCH Rust engine; private resources stay on the user's PC.")
    else:
        if runtime.exists():
            raise RuntimeError("Incomplete fre-rust-runtime staging; run BUILD-FRE-RUST.ps1 again.")
        print("Native FRE Rust engine is not yet built; Play will be disabled with an explicit explanation.")
