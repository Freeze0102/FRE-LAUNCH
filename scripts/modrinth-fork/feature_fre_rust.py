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

    # Patch in place after the original pinned Modrinth -> Horizon adapter so
    # the old adapter's rigid upstream anchors are kept stable. Only this new
    # native module remains referenced in the final Tauri binary.
    exactly(app / "src/api/mod.rs", "pub mod horizon;", "pub mod fre_rust;")
    exactly(app / "src/main.rs",
            ".plugin(api::horizon::init())",
            ".plugin(api::fre_rust::init())")
    build = app / "build.rs"
    old = '''            .plugin(
                "fre-horizon",
                InlinedPlugin::new()
                    .commands(&["horizon_instances", "horizon_create", "horizon_play", "horizon_logs", "horizon_versions", "horizon_install_version", "horizon_set_version", "horizon_prepare", "horizon_install_launcher", "horizon_detect_launcher", "horizon_status", "horizon_open_launcher", "horizon_direct_play"])
                    .default_permission(DefaultPermissionRule::AllowAllCommands),
            )'''
    new = '''            .plugin(
                "fre-rust",
                InlinedPlugin::new()
                    .commands(&["rust_profiles", "rust_create_profile", "rust_runtime", "rust_launch", "rust_logs"])
                    .default_permission(DefaultPermissionRule::AllowAllCommands),
            )'''
    exactly(build, old, new)
    exactly(app / "capabilities/plugins.json",
            '"fre-horizon:default"', '"fre-rust:default"')

    app_vue = frontend / "src/App.vue"
    replacements = {
        "import HorizonRustPanel from '@/components/ui/HorizonRustPanel.vue'":
            "import FreRustPanel from '@/components/ui/FreRustPanel.vue'",
        "const horizonWizard = ref(null)": "const freRustWizard = ref(null)",
        "function showHorizonWizard() {": "function showFreRustWizard() {",
        "horizonWizard.value?.show()": "freRustWizard.value?.show()",
        "provide('showHorizonWizard', showHorizonWizard)":
            "provide('showFreRustWizard', showFreRustWizard)",
        ':on-horizon="showHorizonWizard"': ':on-horizon="showFreRustWizard"',
        '<HorizonRustPanel ref="horizonWizard" />': '<FreRustPanel ref="freRustWizard" />',
    }
    for before, after in replacements.items():
        exactly(app_vue, before, after)

    index = frontend / "src/pages/Index.vue"
    for before, after in {
        "import HorizonLibrary from '@/components/ui/HorizonLibrary.vue'":
            "import FreRustLibrary from '@/components/ui/FreRustLibrary.vue'",
        "<HorizonLibrary": "<FreRustLibrary",
    }.items():
        if before == "<HorizonLibrary":
            old = index.read_text(encoding="utf-8")
            if old.count(before) != 2:
                raise RuntimeError("Expected two Rust library mounts on Index.vue")
            index.write_text(old.replace(before, after), encoding="utf-8")
        else:
            exactly(index, before, after)

    stage = root / "packages/ui/src/components/flows/creation-flow-modal/components/SetupTypeStage.vue"
    exactly(stage, "defaultMessage: 'Horizon (Rust)'",
            "defaultMessage: 'Minecraft (Rust)'")
    exactly(stage,
            "defaultMessage: 'Separate native Rust Minecraft client and versions; FRE LAUNCH launches the game directly.'",
            "defaultMessage: 'An independent Rust game, installed and played inside FRE LAUNCH.'")

    # Drop unused legacy source templates from the GENERATED build tree.
    # They are not deleted from this repository, which keeps historical work.
    for path in (
        app / "src/api/horizon.rs",
        app / "src/api/horizon_bootstrap.rs",
        frontend / "src/components/ui/HorizonRustPanel.vue",
        frontend / "src/components/ui/HorizonLibrary.vue",
    ):
        if path.exists():
            path.unlink()

    # Bundle the native game only when the local build has prepared real
    # artifacts. Missing assets are an explicit Runtime: Not installed state
    # in the UI, NEVER a fake Play button.
    runtime = here / "fre-rust-runtime"
    binaries = ("fre-rust-client.exe", "research-lock.json", "LICENSE.txt")
    valid = all((runtime / f).is_file() for f in binaries) and (
        runtime / "resources/horizon-resources.json"
    ).is_file() and (runtime / "BUILD-INFO.json").is_file()
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
        resources["fre-rust-runtime/resources/"] = "fre-rust/resources/"
        conf.write_text(json.dumps(settings, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        print("Bundled native FRE LAUNCH Rust engine and local Minecraft resources.")
    else:
        if runtime.exists():
            raise RuntimeError("Incomplete fre-rust-runtime staging; run BUILD-FRE-RUST.ps1 again.")
        print("Native FRE Rust engine is not yet built; Play will be disabled with an explicit explanation.")
