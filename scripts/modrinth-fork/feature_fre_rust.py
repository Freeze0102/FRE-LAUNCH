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
    shutil.copyfile(here / "RustInstanceFiles.vue", frontend / "src/components/ui/RustInstanceFiles.vue")
    shutil.copyfile(here / "RustInstancePage.vue", frontend / "src/pages/RustInstancePage.vue")
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
                    .commands(&["rust_profiles", "rust_create_profile", "rust_rename_profile", "rust_delete_profile", "rust_runtime", "rust_import_resources", "rust_launch", "rust_logs", "rust_open_folder", "rust_list_files"])
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

    # Keep the EXACT normal Java creation UI. Rust is a loader chip in the
    # custom stage, not a parallel "New Rust profile" creation experience.
    exactly(app_vue, 'type="instance"\n\t\t\t:on-horizon="showFreRustWizard"',
            'type="instance"\n\t\t\t:available-loaders="[\'fabric\', \'neoforge\', \'forge\', \'quilt\', \'rust\']"\n\t\t\t:on-horizon="showFreRustWizard"')
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
        // Engine is bundled with FRE LAUNCH. An authenticated owner's private
        // Minecraft resources may still need to be prepared or imported.
        let status = await invoke('plugin:fre-rust|rust_runtime')
        if (status.installed && !status.prepared) {
            // Reuse an existing owner's prepared Horizon installation when
            // present. No unlicensed assets are fetched or shipped.
            try {
                await invoke('plugin:fre-rust|rust_import_resources', { source: '' })
                status = await invoke('plugin:fre-rust|rust_runtime')
            } catch {
                // No prepared local resources or ownership verification:
                // open the in-app importer with the exact new profile.
            }
        }
        // Keep the user in the ordinary Library after Create. The Rust
        // card's Setup action opens the importer if resources are still needed.
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

    # The earlier separate entry in setup-type is superseded by the regular
    # custom loader chips; existing Horizon management routes remain intact.
    setup_stage = root / 'packages/ui/src/components/flows/creation-flow-modal/components/SetupTypeStage.vue'
    exactly(setup_stage,
            '''\t\t\t\t<BigOptionButton
\t\t\t\t\tv-if="ctx.onHorizon"
\t\t\t\t\t:icon="BoxesIcon"
\t\t\t\t\t:title="formatMessage(messages.horizonTitle)"
\t\t\t\t\t:description="formatMessage(messages.horizonDescription)"
\t\t\t\t\t@click="ctx.onHorizon?.()"
\t\t\t\t/>\n''',
            '')


    # The same Java instance card now opens a real full-width route rather
    # than a bespoke modal. No native Rust operation reaches Java's loader.
    routes = frontend / "src/routes.js"
    exactly(routes,
            "\t\t{\n\t\t\tpath: '/instance/:id',",
            """\t\t{
            path: '/rust-instance/:kind(native|horizon)/:id',
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
    try {
        const exe = await invoke<string | null>('plugin:fre-horizon|horizon_detect_launcher')
        if (exe) {
            const state = await invoke<{ instances: { id: string; name: string }[] }>(
                'plugin:fre-horizon|horizon_status', { exePath: exe },
            )
            names.push(...state.instances
                .filter(profile => profile.id === 'horizon' || /^[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}$/.test(profile.id))
                .map(profile => profile.name))
        }
    } catch { /* No Horizon executable is not a Java library error. */ }
    if (generation === rustRefreshGeneration) rustProfileNames.value = names
}
const refreshRustLibrary = () => { void refreshRustNames() }
onMounted(() => {
    window.addEventListener('fre-rust-library-refresh', refreshRustLibrary)
    window.addEventListener('fre-horizon-library-refresh', refreshRustLibrary)
    void refreshRustNames()
})
onUnmounted(() => {
    rustRefreshGeneration++
    window.removeEventListener('fre-rust-library-refresh', refreshRustLibrary)
    window.removeEventListener('fre-horizon-library-refresh', refreshRustLibrary)
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
    # Don't append separate Horizon/Rust sections or an empty "Rust library".
    exactly(index,
            "import HorizonLibrary from '@/components/ui/HorizonLibrary.vue'\n",
            "")
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
    if (!found) {
        try {
            const exe = await invoke<string | null>('plugin:fre-horizon|horizon_detect_launcher')
            if (exe) {
                const state = await invoke<{ instances: { id: string }[] }>(
                    'plugin:fre-horizon|horizon_status', { exePath: exe },
                )
                found = state.instances.length > 0
            }
        } catch { /* A removed Horizon binary is not a reason to change Home. */ }
    }
    if (request === rustHomeGeneration) {
        hasRustProfiles.value = found
        rustHomeReady.value = true
    }
}
const onRustHomeRefresh = () => { void checkRustHome() }
onMounted(() => {
    window.addEventListener('fre-rust-library-refresh', onRustHomeRefresh)
    window.addEventListener('fre-horizon-library-refresh', onRustHomeRefresh)
    void checkRustHome()
})
onUnmounted(() => {
    rustHomeGeneration++
    window.removeEventListener('fre-rust-library-refresh', onRustHomeRefresh)
    window.removeEventListener('fre-horizon-library-refresh', onRustHomeRefresh)
})""")
    exactly(index,
            '\t<WelcomeScreen v-if="isReady && !hasCreatedInstance" />\n\t<div\n\t\tv-else-if="isReady"',
            '\t<WelcomeScreen v-if="isReady && !hasCreatedInstance && rustHomeReady && !hasRustProfiles" />\n\t<div\n\t\tv-else-if="isReady && (hasCreatedInstance || hasRustProfiles)"')
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
        rust_requirement = (
            'Rust Minecraft використовує окремий рушій та потребує підготовлених ресурсів Minecraft '
            'і Microsoft-акаунта з придбаною Java Edition. Rust — не Java-збірка Fabric.',
            'Rust Minecraft uses a separate native engine and needs your prepared Minecraft game resources '
            'and a Microsoft account that owns Java Edition. A Rust profile is not a Fabric modpack.',
        )
        messages['fre-rust.create.requirement'] = {'message': rust_requirement[position]}
        detail_messages = {
            'fre-rust.page.unavailable': ('Профіль Rust не знайдено.', 'Rust profile was not found.'),
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
                'Для автоматичної підготовки потрібна встановлена ліцензійна Minecraft Java Edition та активний Microsoft-акаунт.',
                'Automatic preparation requires an owned Minecraft Java Edition installation and an active Microsoft account.',
            ),
            'fre-rust.page.choose-resources': ('Обрати папку ресурсів', 'Choose prepared resources folder'),
            'fre-rust.page.resources': ('Файли гри', 'Game resources'),
            'fre-rust.page.preparing': ('Перевірка локальних ресурсів…', 'Checking locally available resources…'),
        }
        for key, pair in detail_messages.items():
            messages[key] = {'message': pair[position]}
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

    from feature_rust_polish import apply as apply_rust_polish
    apply_rust_polish(root)
    print("Integrated Rust instances into the existing Java quick switcher, selection and bulk delete.")
