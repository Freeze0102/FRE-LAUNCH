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
    config_path = app / "tauri.conf.json"
    config = json.loads(config_path.read_text(encoding="utf-8"))
    icon_scope = config["app"]["security"]["assetProtocol"]["scope"]
    # Native icons live outside Theseus's Java profile/cache directories.
    # Permit only the three validated raster filenames, never worlds or tokens.
    for extension in ("png", "jpg", "webp"):
        icon_scope.append(f"$APPDATA/fre-rust-minecraft/profiles/*/icon.{extension}")
    config_path.write_text(json.dumps(config, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")

    shutil.copyfile(here / "fre_rust_backend.rs", app / "src/api/fre_rust.rs")
    shutil.copyfile(here / "fre_rust_setup.rs", app / "src/api/fre_rust_setup.rs")
    shutil.copyfile(here / "fre_rust_jdk.rs", app / "src/api/fre_rust_jdk.rs")
    # The independent first-run downloader uses Mojang HTTPS and pinned SHA-1,
    # not a second launcher, its CLI, or a separate Microsoft account.
    exactly(app / "Cargo.toml", "sha2 = { workspace = true }",
            'reqwest = { workspace = true, features = ["rustls-tls", "stream"] }\n'
            'sha2 = { workspace = true }')
    exactly(app / "Cargo.toml", 'tokio = { workspace = true, features = ["time"] }',
            'tokio = { workspace = true, features = ["time", "fs", "io-util", "process", "rt", "sync"] }')
    shutil.copyfile(here / "FreRustPanel.vue", frontend / "src/components/ui/FreRustPanel.vue")
    shutil.copyfile(here / "FreRustLibrary.vue", frontend / "src/components/ui/FreRustLibrary.vue")
    shutil.copyfile(here / "RustInstanceGrid.vue", frontend / "src/components/ui/library/instance-group/RustInstanceGrid.vue")
    shutil.copyfile(here / "RustInstanceFiles.vue", frontend / "src/components/ui/RustInstanceFiles.vue")
    shutil.copyfile(here / "RustInstancePage.vue", frontend / "src/pages/RustInstancePage.vue")
    shutil.copyfile(here / "RustNativeMods.vue", frontend / "src/components/ui/RustNativeMods.vue")
    shutil.copyfile(here / "rust-library-group.ts", frontend / "src/components/ui/library/rust-library-group.ts")

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

    # FRE Rust is registered directly in Tauri, with no Horizon Launcher
    # plugin or foreign account workflow; old data folders remain untouched.
    exactly(app / "src/api/mod.rs",
            "pub mod files;\npub mod friends;",
            "pub mod files;\npub mod fre_rust;\npub mod friends;")
    exactly(app / "src/main.rs",
            "        .plugin(api::instance::init())",
            "        .plugin(api::instance::init())\n        .plugin(api::fre_rust::init())")
    exactly(app / "build.rs",
            '            .plugin(\n                "instance",',
            '''            .plugin(
                "fre-rust",
                InlinedPlugin::new()
                    .commands(&["rust_profiles", "rust_create_profile", "rust_rename_profile", "rust_set_profile_icon", "rust_delete_profile", "rust_runtime", "rust_import_resources", "rust_mods", "rust_import_mod", "rust_launch", "rust_running", "rust_stop", "rust_logs", "rust_open_folder", "rust_list_files"])
                    .default_permission(DefaultPermissionRule::AllowAllCommands),
            )
            .plugin(
                "instance",''')
    exactly(app / "capabilities/plugins.json",
            '"instance:default",',
            '"instance:default",\n\t\t"fre-rust:default",')

    app_vue = frontend / "src/App.vue"
    exactly(app_vue,
            "import IconEditorModal from '@/components/ui/instance_settings/icon-editor-modal/index.vue'",
            "import FreRustPanel from '@/components/ui/FreRustPanel.vue'\n"
            "import IconEditorModal from '@/components/ui/instance_settings/icon-editor-modal/index.vue'")
    exactly(app_vue, "const creationIconEditorModal = ref(null)",
            "const freRustWizard = ref(null)\nconst creationIconEditorModal = ref(null)")
    exactly(app_vue, "const creationIconTarget = ref('creation-flow')",
            """const creationIconTarget = ref('creation-flow')

function showFreRustWizard(profileId) {
    installationModal.value?.hide()
    freRustWizard.value?.show(profileId)
}
provide('showFreRustWizard', showFreRustWizard)""")
    exactly(app_vue, "import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'",
            "import { computed, nextTick, onMounted, onUnmounted, provide, ref, watch } from 'vue'")
    exactly(app_vue,
            '<IconEditorModal\n\t\t\tref="creationIconEditorModal"',
            '<FreRustPanel ref="freRustWizard" />\n\t\t<IconEditorModal\n\t\t\tref="creationIconEditorModal"')

    # Create Rust the same way as Fabric/Forge: select the Rust loader chip.
    exactly(app_vue,
            'type="instance"',
            'type="instance"\n\t\t\t:available-loaders="[\'fabric\', \'neoforge\', \'forge\', \'quilt\', \'rust\']"')

    exactly(app_vue, '@create="handleCreate"', '@create="handleFreRustOrJavaCreate"')
    exactly(app_vue,
            "provide('showFreRustWizard', showFreRustWizard)",
            """provide('showFreRustWizard', showFreRustWizard)

// A Rust profile has its own native game format. Do not pass it into the
// Java/Theseus instance creator even though it uses the same creation UI.
async function handleFreRustOrJavaCreate(config) {
    if (config.selectedLoader.value !== 'rust') {
        return handleCreate(config)
    }
    try {
        const version = config.selectedGameVersion.value
        if (version !== '26.3') throw new Error('This Rust game build only supports Minecraft 26.3.')
        const name = (config.instanceName.value.trim() || config.autoInstanceName.value).trim()
        if (!name) throw new Error('Enter a name for the Rust profile.')
        const icon = config.instanceIcon.value
        const iconBytes = icon ? Array.from(new Uint8Array(await icon.arrayBuffer())) : null
        const profile = await invoke('plugin:fre-rust|rust_create_profile', {
            name,
            iconPath: icon ? null : config.instanceIconPath.value,
            iconBytes,
        })
        installationModal.value?.hide()
        window.dispatchEvent(new Event('fre-rust-library-refresh'))
        // Open the normal Java-style instance details immediately, even when
        // its verified native assets still need installing.
        await router.push('/rust-instance/native/' + encodeURIComponent(profile.id))
        const status = await invoke('plugin:fre-rust|rust_runtime')
        if (!status.installed) {
            handleError(new Error(status.reason || 'The native Rust game is not bundled with this FRE LAUNCH build.'))
            return
        }
        if (!status.prepared) {
            // First run downloads only the player's authorized Mojang files.
            // Surface real errors instead of silently leaving an empty profile.
            try {
                await invoke('plugin:fre-rust|rust_import_resources', { source: '' })
            } catch (error) {
                handleError(error instanceof Error ? error : new Error(String(error)))
            }
        }
        window.dispatchEvent(new Event('fre-rust-library-refresh'))
    } catch (error) {
        config.loading.value = false
        handleError(error)
    }
}""")

    # The cross-platform creation flow must never request a fake Rust version
    # from Modrinth's Java-loader API. This binary currently supports 26.3.
    flow_context = root / 'packages/ui/src/components/flows/creation-flow-modal/creation-flow-context.ts'
    exactly(flow_context,
            "selectedLoader.value === 'vanilla' ||\n\t\t\tprojectInstall.value !== null,",
            "selectedLoader.value === 'vanilla' ||\n\t\t\tselectedLoader.value === 'rust' ||\n\t\t\tprojectInstall.value !== null,")
    exactly(flow_context,
            "if (!loader || loader === 'vanilla') return",
            "if (!loader || loader === 'vanilla' || loader === 'rust') return")
    custom_stage = root / 'packages/ui/src/components/flows/creation-flow-modal/components/CustomSetupStage.vue'
    exactly(custom_stage,
            "if (!loader || loader === 'vanilla') return false",
            "if (!loader || loader === 'vanilla' || loader === 'rust') return false")
    exactly(custom_stage,
            "// For loaders with per-version data, only show game versions that have builds",
            """// Native Rust Minecraft is not a Java mod loader. Only advertise the
    // version actually supported by the bundled native runtime.
    if (selectedLoader.value === 'rust') return [{ value: '26.3', label: '26.3' }]

    // For loaders with per-version data, only show game versions that have builds""")
    exactly(custom_stage,
            "if (!loader || !gameVersion || loader === 'vanilla') return",
            "if (!loader || !gameVersion || loader === 'vanilla' || loader === 'rust') return")
    exactly(custom_stage,
            "\t\t<!-- Game version -->",
            """\t\t<p v-if="ctx.flowType === 'instance' && selectedLoader === 'rust'" class="m-0 text-sm text-secondary">
            {{ formatMessage(messages.nativeRustRequirement) }}
        </p>
\t\t<!-- Game version -->""")
    exactly(custom_stage,
            "\tuploadIcon: {",
            """\tnativeRustRequirement: {
        id: 'fre-rust.create.requirement',
        defaultMessage: 'Rust Minecraft uses a separate native engine and needs your prepared Minecraft game resources and a Microsoft account that owns Java Edition. A Rust profile is not a Fabric modpack.',
    },
\tuploadIcon: {""")

    # Keep the standard setup-type UI; no separate Horizon shortcut.
    # The same Java instance card now opens a real full-width route rather
    # than a bespoke modal. No native Rust operation reaches Java's loader.
    routes = frontend / "src/routes.js"
    exactly(routes,
            "\t\t{\n\t\t\tpath: '/instance/:id',",
            """\t\t{
            path: '/rust-instance/:kind(native)/:id',
            name: 'RustInstance',
            component: () => import('@/pages/RustInstancePage.vue'),
        },
\t\t{
\t\t\tpath: '/instance/:id',""")

    # Keep independent Rust backend data, but show each profile in the SAME
    # virtualized card grid as Java instances. Never masquerade as GameInstance:
    # that would run a Rust profile through the Java loader and risk user data.
    group = frontend / "src/components/ui/library/instance-group/index.vue"
    exactly(group,
            "import InstanceCard from '@/components/ui/library/instance-group/instance-card.vue'",
            "import InstanceCard from '@/components/ui/library/instance-group/instance-card.vue'\nimport RustInstanceGrid from '@/components/ui/library/instance-group/RustInstanceGrid.vue'\nimport { rustLibraryGroupId } from '@/components/ui/library/rust-library-group'")
    exactly(group,
            "\tsetSectionCollapsed,\n\tisSearching,",
            "\tsetSectionCollapsed,\n\tisSearching,\n\tsearch,")
    exactly(group,
            "const focusedInstanceId = ref<string | null>(null)",
            "const focusedInstanceId = ref<string | null>(null)\nconst rustCardCount = ref(0)\nconst isRustGridGroup = computed(() => props.instanceGroup.id === rustLibraryGroupId(displayState.value.group))")
    exactly(group,
            "Math.ceil(props.instanceGroup.instances.length / columnCount.value) *",
            "Math.ceil((props.instanceGroup.instances.length + (isRustGridGroup.value ? rustCardCount.value : 0)) / columnCount.value) *")
    exactly(group,
            "v-if=\"instanceGroup.instances.length\"",
            "v-if=\"instanceGroup.instances.length || (isRustGridGroup && rustCardCount)\"")
    exactly(group,
            "v-if=\"instanceGroup.instances.length === 0\"",
            "v-if=\"instanceGroup.instances.length === 0 && (!isRustGridGroup || rustCardCount === 0)\"")
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
\t\t\t\t\t\tv-if="isRustGridGroup"
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
    # The native cards must still be visible when the Java library is empty
    # and when the user switches between None/Loader/Game-version grouping.
    # Query only profile names (never reclassify them as Java GameInstances).
    library = frontend / "src/components/ui/library/index.vue"
    exactly(library,
            "import { StarIcon } from '@modrinth/assets'",
            "import { invoke } from '@tauri-apps/api/core'\nimport { StarIcon } from '@modrinth/assets'")
    exactly(library,
            "computed, nextTick, onDeactivated, onUnmounted, ref, toRef, watch",
            "computed, nextTick, onDeactivated, onMounted, onUnmounted, ref, toRef, watch")
    exactly(library,
            "import InstanceGroup from '@/components/ui/library/instance-group/index.vue'",
            "import InstanceGroup from '@/components/ui/library/instance-group/index.vue'\nimport { rustLibraryGroupId, rustLibraryGroupLabel } from '@/components/ui/library/rust-library-group'")
    exactly(library,
            "\tinstanceGroups,\n\tlibraryGroupsLoaded,",
            "\tinstanceGroups,\n\tlibraryGroupsLoaded,\n\tsearch,")
    exactly(library,
            "const visibleInstanceGroups = computed(() =>\n\tinstanceGroups.value.filter((instanceGroup) =>\n\t\tinstanceGroup.id === FAVORITES_GROUP_ID\n\t\t\t? instanceGroup.instances.length > 0\n\t\t\t: instanceGroup.instances.length > 0 ||\n\t\t\t\t(!isSearching.value && !hasActiveFilters.value && instanceGroup.key !== 'None'),\n\t),\n)",
            """const rustProfileNames = ref<string[]>([])
let rustRefreshGeneration = 0
async function refreshRustNames() {
    const generation = ++rustRefreshGeneration
    const names: string[] = []
    try {
        const profiles = await invoke<{ name: string }[]>('plugin:fre-rust|rust_profiles')
        names.push(...profiles.map(profile => profile.name))
    } catch { /* A missing native plugin must not hide Java instances. */ }
    if (generation === rustRefreshGeneration) rustProfileNames.value = names
}
const refreshRustLibrary = () => { void refreshRustNames() }
onMounted(() => {
    window.addEventListener('fre-rust-library-refresh', refreshRustLibrary)
    void refreshRustNames()
})
onUnmounted(() => {
    rustRefreshGeneration++
    window.removeEventListener('fre-rust-library-refresh', refreshRustLibrary)
})
const rustMatchesSearch = computed(() => {
    if (!rustProfileNames.value.length) return false
    const query = search.value.trim().toLocaleLowerCase()
    return !query || 'rust'.includes(query) || 'раст'.includes(query) ||
        rustProfileNames.value.some(name => name.toLocaleLowerCase().includes(query))
})
const visibleInstanceGroups = computed(() => {
    const nativeGroupId = rustLibraryGroupId(displayState.value.group)
    const visible = instanceGroups.value.filter((instanceGroup) =>
        instanceGroup.id === FAVORITES_GROUP_ID
            ? instanceGroup.instances.length > 0
            : instanceGroup.instances.length > 0 ||
                (rustMatchesSearch.value && instanceGroup.id === nativeGroupId) ||
                (!isSearching.value && !hasActiveFilters.value && instanceGroup.key !== 'None'),
    )
    if (rustMatchesSearch.value && !visible.some(group => group.id === nativeGroupId)) {
        visible.push({
            id: nativeGroupId,
            key: rustLibraryGroupLabel(displayState.value.group),
            instances: [],
        })
    }
    return visible
})""")

    index = frontend / "src/pages/Index.vue"
    # Preserve the ORIGINAL Modrinth Home/Welcome behavior. The previous
    # patch removed WelcomeScreen for everyone, which changed the main menu.
    # Only supplement onboarding's Java-only flag when native Rust exists.
    exactly(index,
            "import { computed, inject, ref } from 'vue'",
            "import { computed, inject, onMounted, onUnmounted, ref } from 'vue'")
    exactly(index,
            "import { PlayIcon, PlusIcon } from '@modrinth/assets'",
            "import { invoke } from '@tauri-apps/api/core'\nimport { PlayIcon, PlusIcon } from '@modrinth/assets'")
    exactly(index,
            "const showCreationModal = inject<() => void>('showCreationModal')",
            """const showCreationModal = inject<() => void>('showCreationModal')
const hasRustProfiles = ref(false)
const rustHomeReady = ref(false)
let rustHomeGeneration = 0
async function checkRustHome() {
    const request = ++rustHomeGeneration
    let found = false
    try {
        const profiles = await invoke<{ id: string }[]>('plugin:fre-rust|rust_profiles')
        found = profiles.length > 0
    } catch { /* Keep the standard Java Home when native Rust is not available. */ }
    if (request === rustHomeGeneration) {
        hasRustProfiles.value = found
        rustHomeReady.value = true
    }
}
const onRustHomeRefresh = () => { void checkRustHome() }
onMounted(() => {
    window.addEventListener('fre-rust-library-refresh', onRustHomeRefresh)
    void checkRustHome()
})
onUnmounted(() => {
    rustHomeGeneration++
    window.removeEventListener('fre-rust-library-refresh', onRustHomeRefresh)
})""")
    exactly(index,
            '\t<WelcomeScreen v-if="isReady && !hasCreatedInstance" />\n\t<div\n\t\tv-else-if="isReady"',
            '\t<WelcomeScreen v-if="isReady && !hasCreatedInstance && rustHomeReady && !hasRustProfiles" />\n\t<div\n\t\tv-else-if="isReady && (hasCreatedInstance || hasRustProfiles)"')
    # No second library section or Horizon setup option is ever inserted.
    # The original Java Home and Create layouts remain unchanged.

    # The Rust instance details share the launcher's UA/EN locale system.
    # Keep all UI strings localizable rather than painting Ukrainian over EN.
    grid_translations = {
        'open': ('Відкрити {name}', 'Open {name}'),
        'start': ('Запустити {name}', 'Play {name}'),
        'stop': ('Зупинити {name}', 'Stop {name}'),
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
        'horizon-required': ('Спочатку встанови ресурси Minecraft на Rust у FRE LAUNCH.',
                             'Prepare the native Minecraft resources in FRE LAUNCH before playing.'),
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
        rust_requirement = (
            'Rust Minecraft використовує окремий рушій та потребує підготовлених ресурсів Minecraft '
            'і Microsoft-акаунта з придбаною Java Edition. Rust — не Java-збірка Fabric.',
            'Rust Minecraft uses a separate native engine and needs your prepared Minecraft game resources '
            'and a Microsoft account that owns Java Edition. A Rust profile is not a Fabric modpack.',
        )
        messages['fre-rust.create.requirement'] = {'message': rust_requirement[position]}
        detail_messages = {
            'fre-rust.page.unavailable': ('Профіль Rust не знайдено.', 'Rust profile was not found.'),
            'fre-rust.page.stop': ('Зупинити', 'Stop'),
            'fre-rust.page.stopping': ('Зупинення…', 'Stopping…'),
            'fre-rust.page.content-note': (
                'Нативні Rust-моди зберігаються у файлах гри. Java-моди Fabric/Forge несумісні.',
                'Native Rust mods are managed in the game files. Java Fabric/Forge mods are not compatible.',
            ),
            'fre-rust.page.no-logs': ('Журналів гри ще немає.', 'No local game logs yet.'),
            'fre-rust.page.retry': ('Оновити', 'Refresh'),
            'fre-rust.files.read-only': (
                'Файли Rust наразі змінюються через провідник Windows.',
                'Native Rust files are currently managed using your file explorer.',
            ),
            'fre-rust.page.rename': ('Назва збірки', 'Instance name'),
            'fre-rust.page.save-name': ('Зберегти назву', 'Save name'),
            'fre-rust.page.delete': ('Видалити збірку', 'Delete instance'),
            'fre-rust.page.install': ('Підготувати ресурси', 'Prepare game resources'),
            'fre-rust.page.installing': ('Підготовка…', 'Preparing…'),
            'fre-rust.page.install-failed': (
                'Для автоматичної підготовки потрібен Microsoft-акаунт із придбаною Minecraft Java Edition у FRE LAUNCH. Для підготовлювача ресурсів може знадобитися Java 25.',
                'Automatic preparation requires a Minecraft Java Edition account in FRE LAUNCH. The native resource preparer may require Java 25.',
            ),
            'fre-rust.page.choose-resources': ('Обрати папку ресурсів', 'Choose prepared resources folder'),
            'fre-rust.page.sign-in-horizon': ('Вхід через FRE LAUNCH', 'Sign in with FRE LAUNCH'),
            'fre-rust.page.prep-helper': ('Перевіряємо вбудований Rust-рушій…', 'Checking bundled Rust engine…'),
            'fre-rust.page.prep-version': ('Перевіряємо версію Rust-гри…', 'Checking Rust game version…'),
            'fre-rust.page.prep-local': ('Перевіряємо встановлені файли Minecraft…', 'Checking installed Minecraft files…'),
            'fre-rust.page.prep-download': ('Завантажуємо перевірені файли Minecraft із Mojang…', 'Downloading missing verified Minecraft files from Mojang…'),
            'fre-rust.page.prep-copy': ('Зберігаємо ресурси гри в кеш FRE LAUNCH…', 'Saving game resources in the FRE LAUNCH cache…'),
            'fre-rust.page.prep-java': ('Перевіряємо Java 25 і, якщо потрібно, встановлюємо її у FRE LAUNCH…', 'Checking Java 25 and installing it privately in FRE LAUNCH if needed…'),
            'fre-rust.page.prep-prepare': ('Створюємо ресурси для нативної гри…', 'Preparing verified native Minecraft resources…'),
            'fre-rust.page.prep-ready': ('Ресурси Minecraft підготовлено.', 'Minecraft resources ready.'),
            'fre-rust.page.first-run-hint': (
                'Перший запуск: FRE LAUNCH використає наявні ресурси Minecraft 26.3 або завантажить перевірені файли з Mojang у власний кеш після перевірки Minecraft-акаунта у FRE LAUNCH. Жоден інший лаунчер встановлювати не потрібно.',
                'First run: FRE LAUNCH reuses existing Minecraft 26.3 resources or downloads verified Mojang game files to its own cache after Minecraft account verification in FRE LAUNCH. No separate launcher is required.',
            ),
            'fre-rust.page.resources': ('Файли гри', 'Game resources'),
            'fre-rust.page.change-icon': ('Обрати іконку', 'Choose icon'),
            'fre-rust.page.reset-icon': ('Відновити стандартну іконку', 'Restore default icon'),
            'fre-rust.page.repair': ('Перевірити й відновити файли гри', 'Check and repair game resources'),
            'fre-rust.page.preparing': ('Перевірка локальних ресурсів…', 'Checking locally available resources…'),
            'fre-rust.page.tab.general': ('Загальні', 'General'),
            'fre-rust.page.tab.installation': ('Встановлення', 'Installation'),
            'fre-rust.page.section.instance': ('Збірка', 'Instance'),
            'fre-rust.page.section.game': ('Гра', 'Game'),
            'fre-rust.page.delete-notice': (
                'Остаточно видаляє цю Rust-збірку разом зі світами. Спільні ресурси гри залишаться.',
                'Permanently deletes this Rust instance and its worlds. Shared game resources are kept.',
            ),
            'fre-rust.page.not-downloaded': (
                'FRE LAUNCH завантажує перевірені ресурси Minecraft з Mojang для власника Java Edition без іншого лаунчера.',
                'FRE LAUNCH downloads verified Minecraft resources from Mojang for a Java Edition owner, without another launcher.',
            ),
            'fre-rust.mods.title': ('Моди Rust', 'Rust content mods'),
            'fre-rust.mods.explanation': (
                'Імпортуй .hmod-файли безпосередньо в цю Rust-збірку. Увімкни їх у налаштуваннях «Моди» самої гри. Java JAR-моди не підтримуються; .hnmod потребують окремого компілювання.',
                'Import .hmod content files directly into this Rust instance. Enable them in the game Mods settings. Fabric/Forge JAR files are incompatible; .hnmod source mods need a separate build.',
            ),
            'fre-rust.mods.search': ('Пошук', 'Search'),
            'fre-rust.mods.placeholder': ('Пошук локальних Rust-модів…', 'Search local Rust mods…'),
            'fre-rust.mods.installed': ('Файли цієї збірки', 'Files in this instance'),
            'fre-rust.mods.empty': ('У цій збірці ще немає .hmod-файлів.', 'No .hmod content files yet.'),
            'fre-rust.mods.install': ('Встановити', 'Install'),
            'fre-rust.mods.remove': ('Видалити файл', 'Remove file'),
            'fre-rust.mods.refresh': ('Оновити', 'Refresh'),
            'fre-rust.mods.import-file': ('Імпортувати .hmod', 'Import .hmod'),
            'fre-rust.mods.open-folder': ('Відкрити папку модів', 'Open mods folder'),
            'fre-rust.mods.none': ('Моди не знайдено.', 'No matching native mods found.'),
        }
        for key, pair in detail_messages.items():
            messages[key] = {'message': pair[position]}
        locale.write_text(json.dumps(messages, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')

    # Legacy data stays on disk but is no longer shown as a playable FRE
    # profile. No automatic deletion or unsafe world migration.
    # Bundle the native game only when the local build has prepared real
    # artifacts. Missing assets are an explicit Runtime: Not installed state
    # in the UI, NEVER a fake Play button.
    runtime = here / "fre-rust-runtime"
    if (runtime / "resources").exists():
        raise RuntimeError("Minecraft resources must stay in the owner's private app cache, never in the installer.")
    binaries = ("fre-rust-client.exe", "fre-rust-prepare.exe", "research-lock.json", "LICENSE.txt")
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
        preparer_checksum = hashlib.sha256((runtime / "fre-rust-prepare.exe").read_bytes()).hexdigest()
        if preparer_checksum.lower() != str(info.get("prepare_exe_sha256", "")).lower():
            raise RuntimeError("Native Rust preparer SHA-256 mismatch: rebuild; do not bundle.")
        parity = runtime / "mods/horizon/src/test/java/dev/horizonlab/parity/VanillaHeightmapOracle.java"
        prepare_source = runtime / "crates/horizon-prepare"
        source_files = list((prepare_source / "src").glob("*.rs"))
        if not parity.is_file() or not (prepare_source / "Cargo.toml").is_file() or not (prepare_source / "build.rs").is_file() or len(source_files) < 10:
            raise RuntimeError("Bundled Rust preparer lacks its complete MIT source and Java exporters.")
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
        # These are original MIT exporter sources, not Mojang game files.
        resources["fre-rust-runtime/mods/horizon/src/test/java/dev/horizonlab/parity/"] = "fre-rust/mods/horizon/src/test/java/dev/horizonlab/parity/"
        resources["fre-rust-runtime/crates/horizon-prepare/"] = "fre-rust/crates/horizon-prepare/"
        # PUBLIC releases carry the MIT engine only. Mojang resources are
        # imported locally by the authenticated owner after installation.
        conf.write_text(json.dumps(settings, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        print("Bundled independent native FRE Rust engine + resource preparer; private Minecraft assets stay on the owner's PC.")
    else:
        if runtime.exists():
            raise RuntimeError("Incomplete fre-rust-runtime staging; run BUILD-FRE-RUST.ps1 again.")
        print("Native FRE Rust engine is not yet built; Play will be disabled with an explicit explanation.")

    from feature_rust_polish import apply as apply_rust_polish
    apply_rust_polish(root)
    print("Integrated Rust instances into the existing Java quick switcher, selection and bulk delete.")
