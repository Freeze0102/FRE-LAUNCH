// Uses the pinned workspace's Vue compiler, TypeScript and Vue reactivity.
const { test } = require('node:test')
const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const { createRequire } = require('node:module')
const source = path.resolve(process.argv[2] || 'FRE-Modrinth-Source')
const workspace = createRequire(path.join(source, 'apps/app-frontend/package.json'))
const rootRequire = createRequire(path.join(source, 'package.json'))
const ts = workspace('typescript')
const vue = workspace('vue')
const compiler = rootRequire('@vue/compiler-sfc')

function component(file, invoke, instances = [], options = {}) {
  const filePath = file === 'RustInstanceGrid.vue'
    ? path.join(source, 'apps/app-frontend/src/components/ui/library/instance-group/RustInstanceGrid.vue')
    : path.join(__dirname, file)
  const text = fs.readFileSync(filePath, 'utf8')
  const { descriptor, errors } = compiler.parse(text, { filename: file })
  assert.deepEqual(errors, [])
  compiler.compileScript(descriptor, { id: file })
  const template = compiler.compileTemplate({ source: descriptor.template.content, filename: file, id: file })
  assert.deepEqual(template.errors, [])
  const output = ts.transpileModule(descriptor.scriptSetup.content, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS },
  }).outputText
  const mounted = []
  const unmounted = []
  const mocks = {
    '@tauri-apps/api/core': { invoke },
    '@tauri-apps/api/event': { listen: options.listen || (async () => () => {}) },
    '@tauri-apps/plugin-opener': { openUrl: options.openUrl || (async () => {}) },
    '@tauri-apps/plugin-dialog': { open: options.open || (async () => null) },
    '@/helpers/instance': { list: async () => instances, add_project_from_path: options.addProjectFromPath || (async () => {}) },
    '@modrinth/assets': { PlayIcon: {}, CheckIcon: {}, SettingsIcon: {}, TrashIcon: {}, FolderOpenIcon: {} },
    '@/components/ui/library/use-library': {
      getLibraryInstanceSelectionKey: ({ instanceId, groupId }) => JSON.stringify([groupId, instanceId]),
      useLibrary: () => {
        const selectedLibraryInstances = options.selection || vue.ref(new Map())
        return {
          selectedLibraryInstances,
          isLibraryInstanceSelectionActive: vue.computed(() => selectedLibraryInstances.value.size > 0),
          toggleLibraryInstanceSelection: ({ instanceId, groupId }) => {
            const key = JSON.stringify([groupId, instanceId])
            const next = new Map(selectedLibraryInstances.value)
            if (next.has(key)) next.delete(key)
            else next.set(key, { instanceId, groupId })
            selectedLibraryInstances.value = next
          },
        }
      },
    },
    '@modrinth/ui': {
      defineMessages: x => x,
      useVIntl: () => ({ formatMessage: (m, args = {}) =>
        m.defaultMessage.replace(/\{(\w+)\}/g, (_, k) => String(args[k])) }),
      injectNotificationManager: () => ({ handleError: options.handleError || (() => {}) }),
      IconButton: {},
    },
    'vue-router': { useRouter: () => ({ push: options.push || (async () => {}) }) },
    '@/components/ui/library/instance-group/instance-card-view.vue': {},
    vue: { ...vue, onMounted: f => mounted.push(f), onUnmounted: f => unmounted.push(f) },
  }
  const names = file === 'RustInstanceGrid.vue'
    ? 'cards,nativeProfiles,runtime,refresh,openCard,play,cardStyle,viewInstance,detailUrl,busy,selectCard,isSelected,activateCard'
    : file === 'RustNativeMods.vue'
    ? 'installed,busy,error,notice,load,importFile,removeMod,openFolder'
    : file === 'CurseForgeBrowser.vue'
    ? 'instanceId,searchText,apiKey,results,searched,loading,notice,error,installing,doSearch,install'
    : file === 'FreRustPanel.vue'
      ? 'source,name,selectedId,profiles,runtime,pending,error,notice,chooseSourceFolder,importResources,openProfileFiles,create,refresh,show,hide,visible'
      : file === 'FreRustLibrary.vue'
        ? 'profiles,runtime,error,refresh,play,openFiles,openFolder,openRustWizard,loadFiles,closeFiles,fileEntries,fileCategory,fileProfile'
        : file === 'HorizonLibrary.vue'
          ? 'profiles,status,executable,error,refresh,openFiles,openHorizonWizard,refreshFiles,closeFiles,openExplorer,fileEntries,fileCategory,fileProfile'
          : 'minecraftPath,executable,instanceName,launchName,pending,error,feedback,status,prepared,canPlay,activeVersion,releaseTag,downloadProgress,manageHorizon,createInstance,playInstance,downloadLauncher,show,hide,visible'
  const setup = new Function('require', 'defineProps', 'defineExpose', 'defineEmits', 'localStorage', 'window', 'Event', 'exports',
    `${output}\nreturn {${names}}`)
  const listeners = new Map()
  const browser = {
    addEventListener: (type, callback) => listeners.set(type, callback),
    removeEventListener: type => listeners.delete(type),
    dispatchEvent: event => { listeners.get(event.type)?.(event); return true },
  }
  const state = setup(name => {
    assert.ok(name in mocks, `Unexpected dependency: ${name}`)
    return mocks[name]
  }, () => options.props || ({}), () => {}, () => (event, count) => options.emit?.(event, count), { getItem: () => null, setItem: () => {} }, browser, Event, {})
  return { state, mount: async () => { for (const f of mounted) await f() },
    unmount: () => { for (const f of unmounted) f() } }
}

const instances = [
  { id: 'a', name: 'Fabric', loader: 'fabric', game_version: '1.21.1' },
  { id: 'b', name: 'Forge', loader: 'forge', game_version: '1.20.1' },
]
const deferred = () => {
  let resolve, reject
  const promise = new Promise((a, b) => { resolve = a; reject = b })
  return { promise, resolve, reject }
}

test('CurseForge uses the in-app API and never redirects a keyless search to a website', async () => {
  const calls = []
  const c = component('CurseForgeBrowser.vue', async (command, args) => {
    calls.push({ command, args })
    throw new Error('CurseForge needs an approved API key')
  }, instances)
  await c.mount()
  c.state.searchText.value = 'Sodium & shaders'
  await c.state.doSearch()
  assert.deepEqual(calls, [{
    command: 'plugin:fre-curseforge|cf_search',
    args: {
      query: 'Sodium & shaders',
      gameVersion: '1.21.1',
      modLoader: 'fabric',
      contentType: 'mod',
      apiKey: null,
    },
  }])
  assert.match(c.state.error.value, /approved API key/)
  assert.equal(c.state.searched.value, false)
  const template = fs.readFileSync(path.join(__dirname, 'CurseForgeBrowser.vue'), 'utf8')
  assert.doesNotMatch(template, /curseForgeWebsiteSearchUrl|browseWithoutKey|importDownloadedMod|openUrl\(/)
})

test('CurseForge discards an old search after switching instance', async () => {
  const request = deferred()
  const c = component('CurseForgeBrowser.vue', () => request.promise, instances)
  await c.mount()
  const searching = c.state.doSearch()
  c.state.instanceId.value = 'b'
  request.resolve([{ id: 1, name: 'Wrong version' }])
  await searching
  assert.deepEqual(c.state.results.value, [])
  assert.equal(c.state.loading.value, false)
})

test('CurseForge latest query wins even when responses arrive out of order', async () => {
  const old = deferred(), recent = deferred()
  let calls = 0
  const c = component('CurseForgeBrowser.vue', () => (++calls === 1 ? old : recent).promise, instances)
  await c.mount()
  const first = c.state.doSearch()
  c.state.searchText.value = 'new'
  const second = c.state.doSearch()
  recent.resolve([{ id: 2 }])
  await second
  old.resolve([{ id: 1 }])
  await first
  assert.equal(c.state.results.value[0].id, 2)
})

test('CurseForge refuses unavailable distribution and keeps install destination stable', async () => {
  const request = deferred(), calls = []
  const c = component('CurseForgeBrowser.vue', (command, args) => {
    calls.push({ command, args }); return request.promise
  }, instances)
  await c.mount()
  await c.state.install({ id: 1, allowModDistribution: false })
  await c.state.install({ id: 1, allowModDistribution: true, isAvailable: false })
  assert.equal(calls.length, 0)
  const install = c.state.install({ id: 1, name: 'Example', allowModDistribution: true })
  c.state.instanceId.value = 'b'
  request.resolve('done')
  await install
  assert.equal(calls[0].args.instanceId, 'a')
  assert.match(c.state.notice.value, /^Fabric:/)
})

test('CurseForge modpack search uses the approved modpack category, not Java mod installer', async () => {
  const calls = []
  const c = component('CurseForgeBrowser.vue', async (command, args) => {
    calls.push({ command, args })
    if (command.endsWith('|cf_search')) return [{ id: 47, slug: 'test-pack', name: 'Test', allowModDistribution: true }]
    throw new Error(command)
  }, instances, { props: { contentType: 'modpack' } })
  await c.mount()
  await c.state.doSearch()
  assert.equal(calls.at(-1).args.contentType, 'modpack')
  assert.equal(calls.at(-1).args.gameVersion, null)
  assert.equal(calls.at(-1).args.modLoader, null)
  await c.state.install({ id: 47, allowModDistribution: true })
  assert.equal(calls.length, 1, 'Modpack search must never invoke the Java jar installer')
  const template = fs.readFileSync(path.join(__dirname, 'CurseForgeBrowser.vue'), 'utf8')
  assert.match(template, /v-if="!isModpack"/)
})

test('Rust library stays accessible when no profile exists yet', async () => {
  const source = fs.readFileSync(path.join(__dirname, 'FreRustLibrary.vue'), 'utf8')
  assert.match(source, /<section class="flex flex-col gap-3"/)
  assert.match(source, /v-if="!profiles.length && !error"/)
  const c = component('FreRustLibrary.vue', async command => {
    if (command.endsWith('|rust_profiles')) return []
    if (command.endsWith('|rust_runtime')) return { installed: true, prepared: false, version: '26.3', reason: 'Missing assets' }
    throw new Error(command)
  })
  await c.state.refresh()
  assert.deepEqual(c.state.profiles.value, [])
  assert.equal(c.state.error.value, '')
})

test('Rust profile creation survives refresh and becomes visible in library', async () => {
  const profile = { id: '12345', name: 'Survival Rust', created: 1 }
  let created = false
  const calls = []
  const invoke = async (command, args) => {
    calls.push({ command, args })
    if (command.endsWith('|rust_create_profile')) { created = true; return profile }
    if (command.endsWith('|rust_profiles')) return created ? [profile] : []
    if (command.endsWith('|rust_runtime')) return { installed: true, prepared: true, version: '26.3', reason: null }
    throw new Error(command)
  }
  const panel = component('FreRustPanel.vue', invoke)
  const library = component('FreRustLibrary.vue', invoke)
  await library.mount()
  await panel.mount()
  panel.state.name.value = profile.name
  await panel.state.create()
  await library.state.refresh()
  assert.equal(panel.state.selectedId.value, profile.id)
  assert.equal(library.state.profiles.value[0].name, profile.name)
  assert.ok(calls.some(c => c.command === 'plugin:fre-rust|rust_create_profile' && c.args.name === profile.name))
  library.unmount()
})

test('Rust resource folder picker passes selected directory to import command', async () => {
  const chosen = 'D:\\Minecraft Resources'
  const calls = []
  const c = component('FreRustPanel.vue', async (command, args) => {
    calls.push({ command, args })
    if (command.endsWith('|rust_profiles')) return []
    if (command.endsWith('|rust_runtime')) return { installed: true, prepared: false, version: '26.3', reason: null }
    if (command.endsWith('|rust_import_resources')) return 'Imported files'
    throw new Error(command)
  }, [], { open: async opts => {
    assert.equal(opts.directory, true)
    assert.equal(opts.multiple, false)
    return chosen
  } })
  await c.state.refresh()
  await c.state.chooseSourceFolder()
  assert.equal(c.state.source.value, chosen)
  await c.state.importResources()
  assert.ok(calls.some(({ command, args }) =>
    command === 'plugin:fre-rust|rust_import_resources' && args.source === chosen))
  assert.equal(c.state.pending.value, null)
})

test('Native Rust Files uses the chosen profile ID, never the active Java instance', async () => {
  const calls = []
  const profile = { id: '999888', name: 'Rust survival', created: 1 }
  const c = component('FreRustLibrary.vue', async (command, args) => {
    calls.push({ command, args })
    if (command.endsWith('|rust_profiles')) return [profile]
    if (command.endsWith('|rust_runtime')) return { installed: true, prepared: true, version: '26.3', reason: null }
    if (command.endsWith('|rust_list_files')) return []
    if (command.endsWith('|rust_open_folder')) return undefined
    throw new Error(command)
  })
  await c.state.refresh()
  await c.state.openFiles(profile)
  assert.deepEqual(calls.at(-1), { command: 'plugin:fre-rust|rust_list_files', args: { profileId: profile.id, folder: 'mods' } })
  await c.state.openFolder()
  assert.deepEqual(calls.at(-1), { command: 'plugin:fre-rust|rust_open_folder', args: { profileId: profile.id, folder: 'mods' } })
})

test('Native Rust manager folder shortcut targets the selected profile', async () => {
  const calls = []
  const profile = { id: '112233', name: 'Another Rust game', created: 1 }
  const c = component('FreRustPanel.vue', async (command, args) => {
    calls.push({ command, args })
    if (command.endsWith('|rust_profiles')) return [profile]
    if (command.endsWith('|rust_runtime')) return { installed: true, prepared: true, version: '26.3', reason: null }
    if (command.endsWith('|rust_open_folder')) return undefined
    throw new Error(command)
  })
  await c.state.show(profile.id)
  assert.equal(c.state.selectedId.value, profile.id)
  await c.state.openProfileFiles('mods')
  assert.deepEqual(calls.at(-1), { command: 'plugin:fre-rust|rust_open_folder', args: { profileId: profile.id, folder: 'mods' } })
})

test('Native Rust Files stay inside selected profile in the launcher', async () => {
  const calls = []
  const profile = { id: '12345678', name: 'Rust', created: 1 }
  const c = component('FreRustLibrary.vue', async (command, args) => {
    calls.push({ command, args })
    if (command.endsWith('|rust_list_files')) return [{ name: 'mod.wasm', bytes: 12, isDirectory: false }]
    throw new Error(command)
  })
  await c.state.openFiles(profile)
  assert.equal(c.state.fileProfile.value.id, profile.id)
  assert.deepEqual(calls.at(-1), {
    command: 'plugin:fre-rust|rust_list_files',
    args: { profileId: profile.id, folder: 'mods' },
  })
  assert.equal(c.state.fileEntries.value[0].name, 'mod.wasm')
})

test('Leaving an instance catalog resets context without stale breadcrumb handles', () => {
  const file = path.join(source, 'apps/app-frontend/src/pages/Browse.vue')
  const { descriptor } = compiler.parse(fs.readFileSync(file, 'utf8'))
  const ast = ts.createSourceFile(file, descriptor.scriptSetup.content, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS)
  const reset = ast.statements.find(s => ts.isFunctionDeclaration(s) && s.name.text === 'resetInstanceContext')
  assert.ok(reset)
  const installedProjectIds = vue.ref(['old'])
  const instanceHideInstalled = vue.ref(true)
  const newlyInstalled = vue.ref(['old'])
  const hiddenInstanceProjectIds = vue.ref(new Set(['old']))
  const hiddenInstanceProjectIdsInitialized = vue.ref(true)
  let refreshed = 0
  const bindings = { debugLog: () => {}, installedProjectIds, instanceHideInstalled, newlyInstalled,
    hiddenInstanceProjectIds, hiddenInstanceProjectIdsInitialized,
    refreshInstalledProjectIds: () => { refreshed++ } }
  const code = ts.transpileModule(reset.getText(ast), { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText
  new Function(...Object.keys(bindings), `${code}\nresetInstanceContext()`)(...Object.values(bindings))
  assert.equal(installedProjectIds.value, null)
  assert.equal(instanceHideInstalled.value, false)
  assert.deepEqual(newlyInstalled.value, [])
  assert.equal(refreshed, 1)
})


test('FRE Rust generator does not install the Horizon launcher or its account UI', () => {
  const generator = fs.readFileSync(path.join(__dirname, 'feature_fre_rust.py'), 'utf8')
  const prepare = fs.readFileSync(path.join(__dirname, 'prepare.py'), 'utf8')
  const backend = fs.readFileSync(path.join(__dirname, 'fre_rust_backend.rs'), 'utf8')
  assert.doesNotMatch(prepare, /add_horizon_client\(root\)/)
  assert.doesNotMatch(generator, /HorizonRustPanel|showHorizonWizard|"fre-horizon"/)
  assert.doesNotMatch(backend, /horizon_install_launcher|horizon_detect_launcher|native_mods_for_profile/)
  assert.match(generator, /plugin:fre-rust\|rust_create_profile/)
  assert.match(generator, /available-loaders/)
})

test('FRE LAUNCH prepares with a private verified JDK 25, not another launcher', () => {
  const backend = fs.readFileSync(path.join(__dirname, 'fre_rust_backend.rs'), 'utf8')
  const setup = fs.readFileSync(path.join(__dirname, 'fre_rust_setup.rs'), 'utf8')
  const jdk = fs.readFileSync(path.join(__dirname, 'fre_rust_jdk.rs'), 'utf8')
  const generator = fs.readFileSync(path.join(__dirname, 'feature_fre_rust.py'), 'utf8')
  const page = fs.readFileSync(path.join(__dirname, 'RustInstancePage.vue'), 'utf8')
  assert.match(backend, /mod managed_jdk;/)
  assert.match(setup, /managed_jdk::ensure\(data\)\.await/)
  assert.match(setup, /"--java-home"/)
  assert.match(generator, /fre_rust_jdk\.rs/)
  assert.match(fs.readFileSync(path.join(source, 'apps/app/Cargo.toml'), 'utf8'), /zip = \{ workspace = true \}/)
  assert.match(jdk, /api\.adoptium\.net\/v3\/assets\/latest\/25\/hotspot/)
  assert.match(jdk, /valid_digest\(expected\)/)
  assert.match(jdk, /hasher\.finalize\(\)/)
  assert.match(jdk, /item\.enclosed_name\(\)/)
  assert.match(jdk, /Refusing symbolic links inside a downloaded JDK/)
  assert.match(page, /java: 'prepJava'/)
  assert.doesNotMatch(jdk, /horizon_install_launcher|horizon_detect_launcher/)
})

test('Native Rust profiles use Java InstanceCardView and navigate to the real page', async () => {
  const profile = { id: '12345678', name: 'Survival Rust', created: 1, modified: 98765 }
  const emitted = [], navigated = []
  const c = component('RustInstanceGrid.vue', async command => {
    if (command.endsWith('|rust_profiles')) return [profile]
    if (command.endsWith('|rust_runtime')) return { installed: true, prepared: true, version: '26.3', reason: null }
    if (command.endsWith('|horizon_detect_launcher')) return null
    throw Error(command)
  }, [], {
    props: { startIndex: 1, columnCount: 4, cardWidth: 140, cardHeight: 180, gap: 12, compact: false, search: '' },
    emit: (event, count) => emitted.push({ event, count }),
    push: async url => navigated.push(url),
  })
  await c.state.refresh()
  assert.equal(c.state.cards.value.length, 1)
  assert.deepEqual(c.state.cards.value.map(x => [x.kind, x.name, x.ready]), [['native', 'Survival Rust', true]])
  assert.equal(c.state.viewInstance(c.state.cards.value[0]).loader, 'Rust')
  assert.equal(c.state.viewInstance(c.state.cards.value[0]).modified, 98765)
  assert.deepEqual(c.state.cardStyle(0), { position: 'absolute', width: '140px', left: '152px', top: '0px', height: '180px' })
  c.state.openCard(c.state.cards.value[0])
  assert.deepEqual(navigated, ['/rust-instance/native/12345678'])
  assert.ok(emitted.some(e => e.event === 'count-change' && e.count === 1))
  const grid = fs.readFileSync(path.join(__dirname, 'RustInstanceGrid.vue'), 'utf8')
  assert.match(grid, /<InstanceCardView/)
  assert.match(grid, /<ContextMenu/)
  assert.match(grid, /<ConfirmDeleteInstanceModal/)
  assert.match(grid, /plugin:fre-rust\|rust_delete_profile/)
  assert.doesNotMatch(grid, /<Teleport|role="dialog"|<article/)
})

test('Missing Horizon executable never obscures native Rust or invents a second library', async () => {
  const navigated = []
  const c = component('RustInstanceGrid.vue', async command => {
    if (command.endsWith('|rust_profiles')) return [{ id: '22', name: 'My Rust', created: 0 }]
    if (command.endsWith('|rust_runtime')) return { installed: true, prepared: false, version: '26.3', reason: 'Not prepared' }
    if (command.endsWith('|horizon_detect_launcher')) throw Error('os error 3')
    throw Error(command)
  }, [], {
    props: { startIndex: 0, columnCount: 1, cardWidth: 140, cardHeight: 180, gap: 12, compact: false, search: '' },
    push: async url => navigated.push(url),
  })
  await c.state.refresh()
  assert.equal(c.state.cards.value.length, 1)
  assert.equal(c.state.cards.value[0].ready, false)
  await c.state.play(c.state.cards.value[0])
  assert.deepEqual(navigated, ['/rust-instance/native/22'])
})

test('Native Rust uses native game launch, page-specific Files and settings, never Java instance API', async () => {
  const calls = []
  const c = component('RustInstanceGrid.vue', async (command, args) => {
    calls.push({ command, args })
    if (command.endsWith('|rust_profiles')) return [{ id: '77', name: 'Rust', created: 0 }]
    if (command.endsWith('|rust_runtime')) return { installed: true, prepared: true, version: '26.3', reason: null }
    if (command.endsWith('|horizon_detect_launcher')) return null
    if (command.endsWith('|rust_launch')) return 'started'
    throw Error('Unexpected command ' + command)
  }, [], { props: { startIndex: 0, columnCount: 1, cardWidth: 140, cardHeight: 180, gap: 12, compact: false, search: '' } })
  await c.state.refresh()
  await c.state.play(c.state.cards.value[0])
  assert.ok(calls.some(x => x.command === 'plugin:fre-rust|rust_launch' && x.args.profileId === '77'))
  assert.ok(!calls.some(x => x.command.includes('instance|') || x.command.includes('plugin:profile')))
  const page = fs.readFileSync(path.join(__dirname, 'RustInstancePage.vue'), 'utf8')
  const files = fs.readFileSync(path.join(__dirname, 'RustInstanceFiles.vue'), 'utf8')
  assert.match(page, /<PageHeader/)
  assert.match(page, /<PageHeaderActions/)
  assert.match(page, /<ConfirmDeleteInstanceModal/)
  assert.match(page, /<TabbedModal/)
  assert.match(page, /plugin:fre-rust\|rust_rename_profile/)
  assert.match(page, /plugin:fre-rust\|rust_delete_profile/)
  assert.match(page, /plugin:fre-rust\|rust_import_resources/)
  assert.match(page, /launching \? t\(m.starting\) : t\(m.play\)/)
  assert.match(page, /<NavTabs/)
  assert.match(page, /query="tab"/)
  assert.match(page, /<Transition name="fre-rust-tab"/)
  assert.match(page, /<RustInstanceFiles/)
  assert.match(page, /plugin:fre-rust\|rust_launch/)
  assert.doesNotMatch(page, /showFreRustWizard/)
  assert.match(page, /<ConfirmDeleteInstanceModal/)
  assert.match(page, /<TabbedModal/)
  assert.match(files, /<FilePageLayout/)
  assert.match(files, /plugin:fre-rust\|rust_list_files/)
  assert.match(files, /plugin:fre-rust\|rust_open_folder/)
})

test('Old Horizon records never appear as FRE Rust profiles or invoke a foreign launcher', async () => {
  const calls = []
  const c = component('RustInstanceGrid.vue', async (command) => {
    calls.push(command)
    if (command.endsWith('|rust_profiles')) return []
    if (command.endsWith('|rust_runtime')) return { installed: true, prepared: true, version: '26.3', reason: null }
    throw Error('Foreign launcher command invoked: ' + command)
  }, [], {
    props: { startIndex: 0, columnCount: 2, cardWidth: 140, cardHeight: 180, gap: 12, compact: false, search: '' },
  })
  await c.state.refresh()
  assert.deepEqual(c.state.cards.value, [])
  assert.ok(calls.every(call => call.startsWith('plugin:fre-rust|')))
  const grid = fs.readFileSync(path.join(__dirname, 'RustInstanceGrid.vue'), 'utf8')
  const page = fs.readFileSync(path.join(__dirname, 'RustInstancePage.vue'), 'utf8')
  const files = fs.readFileSync(path.join(__dirname, 'RustInstanceFiles.vue'), 'utf8')
  assert.doesNotMatch(grid + page + files, /plugin:fre-horizon\|/)
})

test('Content mod file import and removal use only the FRE-owned Rust profile', async () => {
  const calls = []
  let installed = []
  const c = component('RustNativeMods.vue', async (command, args) => {
    calls.push({ command, args })
    if (command.endsWith('|rust_mods') && args.action === 'list') return JSON.stringify(installed)
    if (command.endsWith('|rust_import_mod')) {
      installed = [{ project: 'test.hmod', slug: 'test', title: 'test', number: 'local', file: 'test.hmod', source: 'FRE LAUNCH' }]
      return 'Imported. Enable in-game.'
    }
    if (command.endsWith('|rust_mods') && args.action === 'remove') {
      installed = []
      return 'Removed'
    }
    throw Error('Unexpected call: ' + command)
  }, [], {
    props: { profileId: '123' },
    open: async () => 'D:\\Mods\\test.hmod',
  })
  await c.mount()
  await new Promise(resolve => setImmediate(resolve))
  assert.equal(c.state.installed.value.length, 0)
  await c.state.importFile()
  assert.equal(c.state.installed.value.length, 1)
  assert.ok(calls.some(x => x.command === 'plugin:fre-rust|rust_import_mod'
    && x.args.profileId === '123' && x.args.source === 'D:\\Mods\\test.hmod'))
  await c.state.removeMod('test.hmod')
  assert.equal(c.state.installed.value.length, 0)
})

test('Rust appears as native loader in the ordinary Create instance dialog', () => {
  const creation = fs.readFileSync(path.join(source, 'packages/ui/src/components/flows/creation-flow-modal/components/CustomSetupStage.vue'), 'utf8')
  const context = fs.readFileSync(path.join(source, 'packages/ui/src/components/flows/creation-flow-modal/creation-flow-context.ts'), 'utf8')
  const app = fs.readFileSync(path.join(source, 'apps/app-frontend/src/App.vue'), 'utf8')
  const stage = fs.readFileSync(path.join(source, 'packages/ui/src/components/flows/creation-flow-modal/components/SetupTypeStage.vue'), 'utf8')
  assert.match(app, /:available-loaders="\['fabric', 'neoforge', 'forge', 'quilt', 'rust'\]"/)
  assert.match(app, /@create="handleFreRustOrJavaCreate"/)
  assert.match(app, /if \(config\.selectedLoader\.value !== 'rust'\)\s*\{\s*return handleCreate\(config\)/)
  assert.match(app, /plugin:fre-rust\|rust_create_profile/)
  assert.match(app, /iconBytes/)
  assert.match(creation, /if \(selectedLoader\.value === 'rust'\) return \[\{ value: '26\.3', label: '26\.3' \}\]/)
  assert.match(creation, /fre-rust\.create\.requirement/)
  assert.match(context, /selectedLoader\.value === 'rust' \|\|\s*projectInstall\.value !== null/)
  assert.doesNotMatch(stage, /@click="ctx\.onHorizon\?\.\(\)"/)
})

test('Rust profile icons preserve ordinary creation dialog selection', () => {
  const backend = fs.readFileSync(path.join(source, 'apps/app/src/api/fre_rust.rs'), 'utf8')
  const tiles = fs.readFileSync(path.join(__dirname, 'RustInstanceGrid.vue'), 'utf8')
  assert.match(backend, /icon_path: Option<String>/)
  assert.match(backend, /icon_bytes: Option<Vec<u8>>/)
  assert.match(backend, /fn valid_icon\(/)
  assert.match(backend, /profile\.icon_path = \["icon\.png", "icon\.jpg", "icon\.webp"\]/)
  assert.match(tiles, /icon_path: card.iconPath/)
  assert.match(tiles, /<InstanceCardView/)
  assert.match(tiles, /:instance="viewInstance\(card\)"/)
})

test('Modrinth/CurseForge chooser no longer overlaps the negative-margin sticky Browse header', () => {
  const browser = fs.readFileSync(path.join(source, 'apps/app-frontend/src/pages/Browse.vue'), 'utf8')
  const layout = fs.readFileSync(path.join(source, 'packages/ui/src/layouts/shared/browse-tab/layout.vue'), 'utf8')
  assert.match(browser, /relative flex min-h-14 w-full shrink-0 flex-wrap/)
  assert.match(browser, /Modrinth<\/button>/)
  assert.match(browser, /CurseForge<\/button>/)
  assert.doesNotMatch(layout, /sticky top-0 z-20 -mx-6 -mt-6/)
  assert.match(layout, /sticky top-0 z-20 -mx-6 mb-4/)
});


test('Rust cards follow standard Java library grouping and remain visible without Java profiles', () => {
  const pathToGroups = path.join(source, 'apps/app-frontend/src/components/ui/library/rust-library-group.ts')
  const tsSource = fs.readFileSync(pathToGroups, 'utf8')
  const compiled = ts.transpileModule(tsSource, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS },
  }).outputText
  const exported = {}
  new Function('exports', compiled)(exported)
  for (const [mode, id] of [
    ['Group', 'group:none'],
    ['None', 'None:None'],
    ['Loader', 'Loader:Rust'],
    ['Game version', 'Game version:26.3'],
    ['Instance type', 'Instance type:Rust'],
  ]) {
    assert.equal(exported.rustLibraryGroupId(mode), id)
  }
  const library = fs.readFileSync(path.join(source, 'apps/app-frontend/src/components/ui/library/index.vue'), 'utf8')
  const group = fs.readFileSync(path.join(source, 'apps/app-frontend/src/components/ui/library/instance-group/index.vue'), 'utf8')
  assert.match(library, /rustMatchesSearch/)
  assert.match(library, /rustLibraryGroupId\(displayState.value.group\)/)
  assert.match(library, /visible.push\(\{/)
  assert.match(group, /const isRustGridGroup = computed/)
  assert.match(group, /v-if="isRustGridGroup"/)
});


test('Home preserves original Welcome behavior while Rust-only profiles open the normal library', () => {
  const home = fs.readFileSync(path.join(source, 'apps/app-frontend/src/pages/Index.vue'), 'utf8')
  assert.match(home, /import WelcomeScreen from '@\/components\/ui\/WelcomeScreen.vue'/)
  assert.match(home, /<WelcomeScreen v-if="isReady && !hasCreatedInstance && rustHomeReady && !hasRustProfiles"/)
  assert.match(home, /v-else-if="isReady && \(hasCreatedInstance \|\| hasRustProfiles\)"/)
  assert.match(home, /<LibrarySection :instances="instances" \/>/)
  assert.doesNotMatch(home, /<HorizonLibrary|<FreRustLibrary/)
})

test('Native Rust profile has safe real Rename/Delete permissions and same Java confirmation', () => {
  const backend = fs.readFileSync(path.join(source, 'apps/app/src/api/fre_rust.rs'), 'utf8')
  const build = fs.readFileSync(path.join(source, 'apps/app/build.rs'), 'utf8')
  const page = fs.readFileSync(path.join(__dirname, 'RustInstancePage.vue'), 'utf8')
  const grid = fs.readFileSync(path.join(__dirname, 'RustInstanceGrid.vue'), 'utf8')
  assert.match(backend, /async fn rust_rename_profile/)
  assert.match(backend, /async fn rust_delete_profile/)
  assert.match(backend, /profile_id_valid\(id: &str\)/)
  assert.match(backend, /RUNNING_PROFILES\.lock\(\)\.await\.contains\(&profile_id\)/)
  assert.match(backend, /fs::remove_dir_all\(&dir\)/)
  assert.match(build, /"rust_rename_profile"/)
  assert.match(build, /"rust_delete_profile"/)
  assert.match(page, /<ConfirmDeleteInstanceModal/)
  assert.match(grid, /<ConfirmDeleteInstanceModal/)
  assert.match(page, /@delete="deleteProfile"/)
  assert.match(grid, /@delete="confirmedDelete"/)
});


test('Rust instance participates in the real Java library selection without routing its ID to Java', async () => {
  const selection = vue.ref(new Map())
  const paths = []
  const c = component('RustInstanceGrid.vue', async command => {
    if (command.endsWith('|rust_profiles')) return [{ id: '55', name: 'Rust saved world', created: 1 }]
    if (command.endsWith('|rust_runtime')) return { installed: true, prepared: true, version: '26.3', reason: null }
    if (command.endsWith('|horizon_detect_launcher')) return null
    throw Error(command)
  }, [], {
    props: { startIndex: 0, groupId: 'group:none', columnCount: 2, cardWidth: 160, cardHeight: 200, gap: 12, compact: false, search: '' },
    selection,
    push: async url => paths.push(url),
  })
  await c.state.refresh()
  const card = c.state.cards.value[0]
  assert.equal(c.state.isSelected(card), false)
  c.state.selectCard(card)
  assert.equal(c.state.isSelected(card), true)
  assert.deepEqual([...selection.value.values()], [{ instanceId: 'rust:55', groupId: 'group:none' }])
  c.state.selectCard(card)
  assert.equal(c.state.isSelected(card), false)
  c.state.activateCard({ shiftKey: true }, card)
  assert.equal(c.state.isSelected(card), true)
  assert.deepEqual(paths, [])
  c.state.activateCard({ shiftKey: false }, card)
  assert.deepEqual(paths, [])
  const source = fs.readFileSync(path.join(__dirname, 'feature_rust_polish.py'), 'utf8')
  assert.match(source, /instanceId\.startsWith\('rust:'\)/)
  assert.match(source, /plugin:fre-rust\|rust_delete_profile/)
  assert.match(source, /selection => !deletedInstanceIds\.has\(selection\.instanceId\)/)
  assert.match(source, /<ConfirmDeleteInstanceModal/)
});

test('Native Rust reuses Java quick switcher with real game route and avoids Java run/remove', () => {
  const mixin = fs.readFileSync(path.join(__dirname, 'feature_rust_polish.py'), 'utf8')
  assert.match(mixin, /QuickInstanceSwitcher\.vue/)
  assert.match(mixin, /freRustInstances\.value/)
  assert.match(mixin, /freRustDetailUrl\(instance\)/)
  assert.match(mixin, /plugin:fre-rust\|rust_launch/)
  assert.match(mixin, /plugin:fre-rust\|rust_import_resources/)
  assert.match(mixin, /plugin:fre-rust\|rust_delete_profile/)
  const quick = fs.readFileSync(path.join(source, 'apps/app-frontend/src/components/ui/QuickInstanceSwitcher.vue'), 'utf8')
  assert.match(quick, /getInstanceIconUrl\(instance\.icon_path\)/)
  assert.match(quick, /freRustDetailUrl\(instance\)/)
});


test('Native Rust-only onboarding uses the original checklist but counts a Rust profile', () => {
  const provider = fs.readFileSync(path.join(source, 'apps/app-frontend/src/providers/onboarding-checklist.ts'), 'utf8')
  assert.match(provider, /hasNativeRustProfile/)
  assert.match(provider, /plugin:fre-rust\|rust_profiles/)
  assert.match(provider, /has_created_instance \?\? false\) \|\| hasNativeRustProfile\.value/)
  assert.match(provider, /fre-rust-library-refresh/)
  assert.doesNotMatch(provider, /showChecklist: computed\(\(\) => false/)
});

test('Rust profile icon settings save the selected image, support reset, and ignore cancellation', async () => {
  const file = path.join(__dirname, 'RustInstancePage.vue')
  const { descriptor } = compiler.parse(fs.readFileSync(file, 'utf8'))
  const ast = ts.createSourceFile(file, descriptor.scriptSetup.content, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS)
  const fn = ast.statements.find(s => ts.isFunctionDeclaration(s) && s.name.text === 'changeIcon')
  const code = ts.transpileModule(fn.getText(ast), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
  }).outputText
  let chosen = 'C:/Pictures/my-icon.webp'
  const calls = [], events = [], errors = []
  let refreshed = 0
  const savingIcon = vue.ref(false)
  const bindings = {
    require: name => { assert.equal(name, '@tauri-apps/plugin-dialog'); return { open: async () => chosen } },
    savingIcon, profile: vue.ref({ id: '123' }), id: vue.ref('123'),
    invoke: async (command, args) => calls.push({ command, args }),
    refresh: async () => { refreshed++ },
    handleError: err => errors.push(err),
    window: { dispatchEvent: e => events.push(e.type) }, Event,
  }
  const change = new Function(...Object.keys(bindings), code + '\nreturn changeIcon')(...Object.values(bindings))
  await change()
  await change(true)
  chosen = null
  await change()
  assert.deepEqual(calls, [
    { command: 'plugin:fre-rust|rust_set_profile_icon', args: { profileId: '123', source: 'C:/Pictures/my-icon.webp' } },
    { command: 'plugin:fre-rust|rust_set_profile_icon', args: { profileId: '123', source: null } },
  ])
  assert.equal(refreshed, 2)
  assert.deepEqual(events, ['fre-rust-library-refresh', 'fre-rust-library-refresh'])
  assert.equal(savingIcon.value, false)
  assert.deepEqual(errors, [])
})

test('Rust resource setup and native mod browser compile without an external launcher', () => {
  const compiler = rootRequire('@vue/compiler-sfc')
  for (const file of ['RustNativeMods.vue', 'RustInstancePage.vue', 'FreRustPanel.vue']) {
    const sourceText = fs.readFileSync(path.join(__dirname, file), 'utf8')
    const { descriptor, errors } = compiler.parse(sourceText, { filename: file })
    assert.deepEqual(errors, [], file + ' parse failed')
    assert.ok(descriptor.scriptSetup, file + ' missing script')
    assert.ok(descriptor.template, file + ' missing template')
    assert.doesNotThrow(() => compiler.compileScript(descriptor, { id: file }), file + ' script failed')
    const result = compiler.compileTemplate({
      source: descriptor.template.content, filename: file, id: file,
    })
    assert.deepEqual(result.errors, [], file + ' template failed')
  }
  const api = fs.readFileSync(path.join(source, 'apps/app/src/api/fre_rust.rs'), 'utf8')
  const setup = fs.readFileSync(path.join(source, 'apps/app/src/api/fre_rust_setup.rs'), 'utf8')
  const nativeMods = fs.readFileSync(path.join(source, 'apps/app-frontend/src/components/ui/RustNativeMods.vue'), 'utf8')
  const page = fs.readFileSync(path.join(source, 'apps/app-frontend/src/pages/RustInstancePage.vue'), 'utf8')
  assert.match(api, /fn resources_valid\(/)
  assert.match(api, /async fn rust_mods/)
  assert.match(api, /preserve_incomplete\(&destination\)/)
  assert.match(api, /standalone_setup::prepare/)
  assert.match(setup, /verified_preparer\(bundle\)/)
  assert.equal(fs.existsSync(path.join(source, 'apps/app/src/api/horizon.rs')), false)
  assert.doesNotMatch(api + setup, /horizon::prepare_for_native_rust|horizon_install_launcher|native_mods_for_profile/)
  assert.match(nativeMods, /plugin:fre-rust\|rust_mods/)
  assert.match(page, /<RustNativeMods v-if="kind === 'native'"/)
})
