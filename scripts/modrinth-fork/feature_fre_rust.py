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
    shutil.copyfile(here / "RustInstanceGrid.vue", frontend / "src/components/ui/library/instance-group/RustInstanceGrid.vue")

    # Theseus exports its minecraft_auth API but keeps State.pool private.
    # Bridge the existing refreshed default-credentials lookup INSIDE Theseus,
    # rather than accessing private state internals from the Tauri app.
    auth_api = root / "packages/app-lib/src/api/minecraft_auth.rs"
    exactly(
        auth_api,
        "#[tracing::instrument]\npub async fn get_default_user()",
        """/// Return the active Minecraft account after Theseus's normal token
/// refresh, without exposing the private database pool to the Tauri app.
pub async fn fre_rust_default_credentials() -> crate::Result<Option<Credentials>> {
    let state = State::get().await?;
    Credentials::get_default_credential(&state.pool).await
}

#[tracing::instrument]
pub async fn get_default_user()""",
    )

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
                    .commands(&["rust_profiles", "rust_create_profile", "rust_runtime", "rust_import_resources", "rust_launch", "rust_logs", "rust_open_folder", "rust_list_files"])
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
function showFreRustWizard(profileId) {
    installationModal.value?.hide()
    freRustWizard.value?.show(profileId)
}
provide('showFreRustWizard', showFreRustWizard)""")
    # The new Create flow selects the built-in game, while existing Horizon
    # cards still open their original wizard through showHorizonWizard.
    exactly(app_vue, ':on-horizon="showHorizonWizard"',
            ':on-horizon="showFreRustWizard"')
    exactly(app_vue, '<HorizonRustPanel ref="horizonWizard" />',
            '<HorizonRustPanel ref="horizonWizard" />\n\t\t<FreRustPanel ref="freRustWizard" />')

    # Keep independent Rust backend data, but show each profile in the SAME
    # virtualized card grid as Java instances. Never masquerade as GameInstance:
    # that would run a Rust profile through the Java loader and risk user data.
    group = frontend / "src/components/ui/library/instance-group/index.vue"
    exactly(group,
            "import InstanceCard from '@/components/ui/library/instance-group/instance-card.vue'",
            "import InstanceCard from '@/components/ui/library/instance-group/instance-card.vue'\nimport RustInstanceGrid from '@/components/ui/library/instance-group/RustInstanceGrid.vue'")
    exactly(group,
            "\tsetSectionCollapsed,\n\tisSearching,",
            "\tsetSectionCollapsed,\n\tisSearching,\n\tsearch,")
    exactly(group,
            "const focusedInstanceId = ref<string | null>(null)",
            "const focusedInstanceId = ref<string | null>(null)\nconst rustCardCount = ref(0)")
    exactly(group,
            "Math.ceil(props.instanceGroup.instances.length / columnCount.value) *",
            "Math.ceil((props.instanceGroup.instances.length + (isUngrouped.value ? rustCardCount.value : 0)) / columnCount.value) *")
    exactly(group,
            "v-if=\"instanceGroup.instances.length\"",
            "v-if=\"instanceGroup.instances.length || (isUngrouped && rustCardCount)\"")
    exactly(group,
            "v-if=\"instanceGroup.instances.length === 0\"",
            "v-if=\"instanceGroup.instances.length === 0 && (!isUngrouped || rustCardCount === 0)\"")
    exactly(group,
            '<div ref="instanceGridContent" :style="{ height: `${gridHeight}px` }">',
            '<div ref="instanceGridContent" class="relative" :style="{ height: `${gridHeight}px` }">')
    exactly(group,
            """\t\t\t\t\t</TransitionGroup>
\t\t\t\t</div>
\t\t\t</div>
\t\t</Accordion>""",
            """\t\t\t\t\t</TransitionGroup>
\t\t\t\t\t<RustInstanceGrid
\t\t\t\t\t\tv-if="isUngrouped"
\t\t\t\t\t\t:start-index="instanceGroup.instances.length"
\t\t\t\t\t\t:column-count="columnCount"
\t\t\t\t\t\t:card-width="cardWidth"
\t\t\t\t\t\t:card-height="cardHeight"
\t\t\t\t\t\t:gap="gap"
\t\t\t\t\t\t:compact="compactMode"
\t\t\t\t\t\t:search="search"
\t\t\t\t\t\t@count-change="rustCardCount = $event"
\t\t\t\t\t/>
\t\t\t\t</div>
\t\t\t</div>
\t\t</Accordion>""")
    index = frontend / "src/pages/Index.vue"
    # Don't append separate Horizon/Rust sections or an empty "Rust library".
    exactly(index,
            "import HorizonLibrary from '@/components/ui/HorizonLibrary.vue'\n",
            "")
    exactly(index, "import WelcomeScreen from '@/components/ui/WelcomeScreen.vue'\n", "")
    exactly(index,
            '\t<WelcomeScreen v-if="isReady && !hasCreatedInstance" />\n\t<div\n\t\tv-else-if="isReady"',
            '\t<div\n\t\tv-if="isReady"')
    exactly(index,
            '\t\t<LibrarySection :instances="instances" />\n\t\t<HorizonLibrary />',
            '\t\t<LibrarySection :instances="instances" />')
    exactly(index,
            '\t<HorizonLibrary v-if="isReady && !hasCreatedInstance" class="px-6 pb-6" />',
            '')

    stage = root / "packages/ui/src/components/flows/creation-flow-modal/components/SetupTypeStage.vue"
    exactly(stage, "defaultMessage: 'Horizon (Rust)'",
            "defaultMessage: 'Minecraft (Rust)'")
    exactly(stage,
            "defaultMessage: 'Separate native Rust Minecraft client and versions; FRE LAUNCH launches the game directly.'",
            "defaultMessage: 'An independent Rust game, installed and played inside FRE LAUNCH.'")

    # The Rust instance details share the launcher's UA/EN locale system.
    # Keep all UI strings localizable rather than painting Ukrainian over EN.
    grid_translations = {
        'open': ('Відкрити {name}', 'Open {name}'),
        'start': ('Запустити {name}', 'Play {name}'),
        'close': ('Закрити', 'Close'),
        'starting': ('Запуск…', 'Starting…'),
        'play': ('Грати', 'Play'),
        'setup': ('Налаштувати гру', 'Set up game'),
        'settings': ('Налаштування', 'Settings'),
        'open-folder': ('Відкрити папку', 'Open folder'),
        'files': ('Файли збірки', 'Instance files'),
        'search-files': ('Пошук файлів…', 'Search files…'),
        'refresh': ('Оновити', 'Refresh'),
        'reading': ('Зчитування файлів…', 'Reading files…'),
        'no-files': ('У цій папці немає файлів.', 'No files in this folder.'),
        'folder': ('Папка', 'Folder'),
        'native-required': ('Перед запуском імпортуй власні ресурси Minecraft у налаштуваннях Rust.',
                            'Import your own Minecraft resources in Rust settings before playing.'),
        'game-required': ('Потрібні твої встановлені файли Minecraft.',
                          'Your own installed Minecraft resources are required.'),
        'horizon-required': ('Спочатку підготуй версію Horizon у налаштуваннях.',
                             'Prepare this Horizon version in settings before playing.'),
        'mods-warning': ('Java JAR-моди не підтримуються нативним Rust-клієнтом. '
                         'Світи та файли кожної збірки зберігаються окремо.',
                         'Java JAR mods are not supported by the native Rust client. '
                         'Each profile keeps its own worlds and files.'),
        'mods': ('Моди', 'Mods'),
        'resourcepacks': ('Ресурс-паки', 'Resource packs'),
        'shaderpacks': ('Шейдери', 'Shaders'),
        'saves': ('Світи', 'Worlds'),
        'logs': ('Журнали', 'Logs'),
        'root': ('Усі файли', 'All files'),
    }
    for language, position in (('uk-UA', 0), ('en-US', 1)):
        locale = frontend / 'src/locales' / language / 'index.json'
        messages = json.loads(locale.read_text(encoding='utf-8'))
        for key, pair in grid_translations.items():
            messages['fre-rust.grid.' + key] = {'message': pair[position]}
        locale.write_text(json.dumps(messages, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')

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
