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
    '@modrinth/assets': { PlayIcon: {}, StopCircleIcon: {}, CheckIcon: {}, SettingsIcon: {}, TrashIcon: {}, FolderOpenIcon: {} },
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
      provideConsoleManager: value => options.consoleManager?.(value),
    },
    'vue-router': { useRouter: () => ({ push: options.push || (async () => {}) }) },
    '@/components/ui/library/instance-group/instance-card-view.vue': {},
    '@/components/ui/RustModCatalog.vue': {},
    '@/components/ui/world/WorldItem.vue': {},
    '@/helpers/worlds': { get_server_status: options.getServerStatus || (async () => ({ players: { online: 1, max: 20 } })) },
    vue: { ...vue, onMounted: f => mounted.push(f), onUnmounted: f => unmounted.push(f) },
  }
  const names = file === 'RustLogs.vue' ? 'logLines,names,selected,loading,sources,activeName,parse,refresh'
    : file === 'RustWorlds.vue' ? 'worlds,query,filter,busy,states,shown,load,ping,play,save,serverName,serverAddress,removing'
    : file === 'RustModCatalog.vue'
    ? 'projects,query,kind,loading,pending,error,notice,stale,expanded,reports,filtered,shown,load,verify,install,details,source,safeSource,image,isInstalled,version,report,page,pageSize,pageCount,order,layout,detailTab,selectedProject,switchPage,selectDetailTab,justInstalled,descriptionForFre'
    : file === 'RustInstanceGrid.vue'
    ? 'cards,nativeProfiles,runtime,refresh,refreshRunning,runningIds,isRunning,openCard,play,stop,cardStyle,viewInstance,detailUrl,busy,selectCard,isSelected,activateCard'
    : file === 'RustNativeMods.vue'
    ? 'installed,busy,error,notice,load,importFile,removeMod,openFolder,settings,saving,setMod,setZoomFactor,setZoomKey,saveSettings,blocked,rows,filtered,selected,removable,changeEnabled,bulkToggle,askRemove,confirmRemove,deleting,query,filter,catalogVisited,openBuiltin'
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

test('Running native Rust instance shows Stop, and rejects duplicate Play', async () => {
  const calls = []
  let running = false
  const c = component('RustInstanceGrid.vue', async (command, args) => {
    calls.push({ command, args })
    if (command.endsWith('|rust_profiles')) return [{ id: '77', name: 'Native Rust', created: 1 }]
    if (command.endsWith('|rust_runtime')) return { installed: true, prepared: true, version: '26.3', reason: null }
    if (command.endsWith('|rust_running')) return running ? ['77'] : []
    if (command.endsWith('|rust_launch')) {
      if (running) throw Error('Backend must reject a duplicate launch')
      running = true
      return 'started'
    }
    if (command.endsWith('|rust_stop')) {
      running = false
      return 'stop requested'
    }
    throw Error('Unexpected command: ' + command)
  }, [], {
    props: { startIndex: 0, columnCount: 1, cardWidth: 140, cardHeight: 180, gap: 12, compact: false, search: '' },
  })
  await c.state.refresh()
  const card = c.state.cards.value[0]
  assert.equal(c.state.isRunning(card), false)
  await c.state.play(card)
  assert.equal(c.state.isRunning(card), true)
  await c.state.play(card)
  assert.equal(calls.filter(x => x.command.endsWith('|rust_launch')).length, 1)
  await c.state.stop(card)
  assert.equal(c.state.isRunning(card), false)
  assert.equal(calls.filter(x => x.command.endsWith('|rust_stop')).length, 1)
  const grid = fs.readFileSync(path.join(__dirname, 'RustInstanceGrid.vue'), 'utf8')
  const page = fs.readFileSync(path.join(__dirname, 'RustInstancePage.vue'), 'utf8')
  const backend = fs.readFileSync(path.join(__dirname, 'fre_rust_backend.rs'), 'utf8')
  const polish = fs.readFileSync(path.join(__dirname, 'feature_rust_polish.py'), 'utf8')
  assert.match(grid, /fre-rust-process-changed/)
  assert.match(page, /fre-rust-process-changed/)
  assert.match(polish, /fre-rust-process-changed/)
  assert.match(polish, /plugin:fre-rust\|rust_stop/)
  assert.match(backend, /async fn rust_running\(/)
  assert.match(backend, /async fn rust_stop\(/)
  assert.match(backend, /RUNNING_PROFILES\.lock\(\)\.await\.retain/)
  assert.match(backend, /STOP_SIGNALS\.lock\(\)\.await\.remove/)
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
  assert.match(page, /running \? t\(m.stop\) : t\(m.play\)/)
  assert.match(page, /<NavTabs/)
  assert.match(page, /mode="local" :active-index=/)
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
    if (command.endsWith('|rust_client_mods')) return { zoom: true, zoomFactor: 4, zoomKey: 'c', fpsCounter: false, coordinates: false }
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
  for (const file of ['RustNativeMods.vue', 'RustModCatalog.vue', 'RustInstancePage.vue', 'FreRustPanel.vue']) {
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


test('Rust client mods save toggles, zoom strength and key for the selected profile', async () => {
  const defaults = () => ({ zoom: true, zoomFactor: 4, zoomKey: 'c', fpsCounter: false, coordinates: false })
  const saved = new Map()
  const calls = []
  const invoke = async (command, args) => {
    calls.push({ command, args })
    if (command.endsWith('|rust_mods')) return '[]'
    assert.equal(command, 'plugin:fre-rust|rust_client_mods')
    if (args.settings) saved.set(args.profileId, { ...args.settings })
    return { ...(saved.get(args.profileId) || defaults()) }
  }
  const c = component('RustNativeMods.vue', invoke, [], { props: { profileId: '123' } })
  await c.state.load()
  assert.equal(c.state.settings.value.zoom, true)
  await c.state.setMod('fpsCounter')
  await c.state.setMod('coordinates')
  await c.state.setZoomFactor({ target: { value: '8' } })
  await c.state.setZoomKey({ target: { value: 'v' } })
  await c.state.setMod('zoom')
  assert.deepEqual(saved.get('123'), { zoom: false, zoomFactor: 8, zoomKey: 'v', fpsCounter: true, coordinates: true })
  const count = calls.length
  await c.state.setZoomFactor({ target: { value: '99' } })
  await c.state.setZoomKey({ target: { value: 'invalid' } })
  assert.equal(calls.length, count)
  const second = component('RustNativeMods.vue', invoke, [], { props: { profileId: '456' } })
  await second.state.load()
  assert.equal(second.state.settings.value.fpsCounter, false)
  await c.state.load()
  assert.equal(c.state.settings.value.zoomFactor, 8)
})

test('A rejected native mod change retains the saved state and clears the busy indicator', async () => {
  const c = component('RustNativeMods.vue', async (command, args) => {
    if (command.endsWith('|rust_mods')) return '[]'
    if (args.settings) throw Error('Close the Rust game before changing client mods.')
    return { zoom: true, zoomFactor: 4, zoomKey: 'c', fpsCounter: false, coordinates: false }
  }, [], { props: { profileId: '123' } })
  await c.state.load()
  await c.state.setMod('zoom')
  assert.equal(c.state.settings.value.zoom, true)
  assert.equal(c.state.saving.value, false)
  assert.match(c.state.error.value, /Close the Rust game/)
})

test('A late Rust mod response cannot replace the newly selected profile settings', async () => {
  const old = deferred()
  const props = { profileId: '123' }
  const c = component('RustNativeMods.vue', async (command, args) => {
    if (command.endsWith('|rust_mods')) return '[]'
    if (args.profileId === '123') return old.promise
    return { zoom: false, zoomFactor: 8, zoomKey: 'z', fpsCounter: true, coordinates: true }
  }, [], { props })
  const first = c.state.load()
  props.profileId = '456'
  await c.state.load()
  old.resolve({ zoom: true, zoomFactor: 2, zoomKey: 'c', fpsCounter: false, coordinates: false })
  await first
  assert.equal(c.state.settings.value.zoomFactor, 8)
  assert.equal(c.state.settings.value.zoom, false)
  assert.equal(c.state.busy.value, false)
})


function catalogFixture(kind = 'native-mod') {
  return { id: 'catalog_demo', name: 'Catalog Demo', kind, summary: 'Camera tool', description: 'Source package', authors: ['ExampleAuthor'], license: 'MIT', source: 'https://github.com/example/demo', categories: ['utility'], tags: ['zoom'], downloads: 50, versions: [{ version: '1', game: '26.3', api: kind === 'mod' ? '0.2' : 'native-1/pins', published: '2026-11-05', file: { name: kind === 'mod' ? 'demo.hmod' : 'demo.hnmod', url: 'https://github.com/example/demo/releases/download/v1/demo.hnmod', size: 100, sha256: '1'.repeat(64) } }] }
}
test('Rust catalog searches authors and tags and restricts external source/image links', async () => {
  const native = catalogFixture()
  const pack = { ...catalogFixture(), id: 'pack', name: 'Pack', kind: 'shader' }
  const c = component('RustModCatalog.vue', async () => ({ projects: [native, pack], generated: '', fetchedAt: 1, stale: false }), [], { props: { profileId: '123', installedFiles: [] } })
  await c.state.load()
  assert.equal(c.state.filtered.value.length, 1)
  c.state.query.value = 'exampleauthor zoom'
  assert.equal(c.state.filtered.value.length, 1)
  c.state.query.value = 'not-found'
  assert.equal(c.state.filtered.value.length, 0)
  c.state.query.value = ''
  c.state.kind.value = 'all'
  assert.equal(c.state.filtered.value.length, 2)
  assert.equal(c.state.safeSource('javascript:alert(1)'), null)
  assert.equal(c.state.safeSource('https://github.com.evil/repo'), null)
  assert.equal(c.state.image('http://localhost/private'), undefined)
  assert.equal(c.state.image('https://evil.test/track.png'), undefined)
})
test('Native compatibility failures stay unavailable after inspection', async () => {
  const project = catalogFixture()
  const calls = []
  const c = component('RustModCatalog.vue', async (command, args) => {
    calls.push({ command, args })
    if (command.endsWith('|rust_catalog')) return { projects: [project], generated: '', fetchedAt: 1, stale: false }
    assert.equal(command, 'plugin:fre-rust|rust_catalog_check')
    assert.equal(args.expectedSha256, '1'.repeat(64))
    return { projectId: project.id, version: '1', sha256: '1'.repeat(64), installable: false, checks: [{ code: 'execution', state: 'warning', detail: 'Unsupported hook API' }] }
  }, [], { props: { profileId: '123', installedFiles: [] } })
  await c.state.load()
  await c.state.verify(project)
  await c.state.install(project)
  assert.equal(c.state.report(project).installable, false)
  assert.equal(c.state.pending.value, '')
  assert.equal(calls.filter(c => c.command.endsWith('|rust_catalog_install')).length, 0)
})
test('Catalog data installation binds the current profile and exact inspected hash', async () => {
  const project = catalogFixture('mod')
  const calls = [], events = []
  const checked = { projectId: project.id, version: '1', sha256: '1'.repeat(64), installable: true, checks: [] }
  const c = component('RustModCatalog.vue', async (command, args) => {
    calls.push({ command, args })
    if (command.endsWith('|rust_catalog')) return { projects: [project], generated: '', fetchedAt: 1, stale: false }
    return checked
  }, [], { props: { profileId: '456', installedFiles: [] }, emit: event => events.push(event) })
  await c.state.load()
  c.state.stale.value = true
  const initial = calls.length
  await c.state.install(project)
  assert.equal(calls.length, initial)
  c.state.stale.value = false
  await c.state.install(project)
  assert.equal(calls.filter(c => c.command.endsWith('|rust_catalog_check')).length, 1)
  assert.equal(calls.filter(c => c.command.endsWith('|rust_catalog_install')).length, 1)
  assert.equal(c.state.isInstalled(project), true)
  const install = calls.find(c => c.command.endsWith('|rust_catalog_install'))
  assert.deepEqual(install.args, { profileId: '456', projectId: project.id, expectedSha256: '1'.repeat(64) })
  assert.deepEqual(events, ['installed'])
  c.state.stale.value = true
  const count = calls.length
  await c.state.install(project)
  assert.equal(calls.length, count)
})
test('Late catalog replies cannot expose another selected profile state', async () => {
  const previous = deferred()
  const props = { profileId: '123' }
  const c = component('RustModCatalog.vue', async (command,args) => args.profileId === '123' ? previous.promise : { projects: [], generated: '', fetchedAt: 1, stale: false }, [], { props })
  const first = c.state.load()
  props.profileId = '456'
  await c.state.load()
  previous.resolve({ projects: [catalogFixture()], generated: '', fetchedAt: 1, stale: false })
  await first
  assert.equal(c.state.projects.value.length, 0)
  assert.equal(c.state.loading.value, false)
})


test('Native instance mod rows search real installed metadata and protect included mods from deletion', async () => {
  const local = { project: 'demo.hmod', file: 'demo.hmod', slug: 'demo', title: 'Demo camera', number: '2', authors: ['Builder'], enabled: false }
  const c = component('RustNativeMods.vue', async command => command.endsWith('|rust_mods') ? JSON.stringify([local]) : ({ zoom: true, zoomFactor: 4, zoomKey: 'c', fpsCounter: false, coordinates: false }), [], { props: { profileId: '123' } })
  await c.state.load()
  assert.equal(c.state.rows.value.length, 4)
  assert.equal(c.state.rows.value.find(row => row.id === 'local:demo.hmod').enabled, false)
  assert.ok(c.state.rows.value.filter(row => row.id.startsWith('builtin:')).every(row => row.hideDelete))
  c.state.query.value = 'builder demo.hmod'
  assert.deepEqual(c.state.filtered.value.map(row => row.id), ['local:demo.hmod'])
  c.state.selected.value = ['builtin:zoom', 'local:demo.hmod']
  assert.deepEqual(c.state.removable.value, ['local:demo.hmod'])
})

test('Local mod switches use the selected profile and reload the actual game state', async () => {
  const calls = [], local = { project: 'demo.hmod', file: 'demo.hmod', slug: 'demo', title: 'Demo', number: '1', enabled: false }
  const c = component('RustNativeMods.vue', async (command, args) => {
    calls.push({ command, args })
    if (command.endsWith('|rust_client_mods')) return { zoom: true, zoomFactor: 4, zoomKey: 'c', fpsCounter: false, coordinates: false }
    if (args.action === 'list') return JSON.stringify([local])
    local.enabled = args.action === 'enable'; return 'Changed'
  }, [], { props: { profileId: '123' } })
  await c.state.load()
  await c.state.changeEnabled('local:demo.hmod', true)
  assert.equal(c.state.rows.value.find(row => row.id === 'local:demo.hmod').enabled, true)
  await c.state.changeEnabled('local:demo.hmod', false)
  assert.equal(c.state.rows.value.find(row => row.id === 'local:demo.hmod').enabled, false)
  assert.deepEqual(calls.filter(call => ['enable','disable'].includes(call.args.action)).map(call => call.args), [
    { profileId: '123', action: 'enable', project: 'demo.hmod' }, { profileId: '123', action: 'disable', project: 'demo.hmod' },
  ])
})

test('Builtin mod settings can change during play while downloaded content stays locked', async () => {
  const saves = [], props = { profileId: '123', running: false }
  const c = component('RustNativeMods.vue', async (command, args) => {
    if (command.endsWith('|rust_mods')) return '[]'
    if (args.settings) saves.push(args.settings)
    return args.settings || { zoom: true, zoomFactor: 8, zoomKey: 'v', fpsCounter: false, coordinates: false }
  }, [], { props })
  await c.state.load(); c.state.selected.value = ['builtin:fpsCounter','builtin:coordinates']
  await c.state.bulkToggle(true)
  assert.deepEqual(saves.at(-1), { zoom: true, zoomFactor: 8, zoomKey: 'v', fpsCounter: true, coordinates: true })
  props.running = true
  const count = saves.length
  await c.state.setMod('zoom')
  assert.equal(saves.length, count + 1)
  assert.equal(saves.at(-1).zoom, false)
  await c.state.bulkToggle(false)
  assert.equal(saves.length, count + 1)
})

test('Mod deletion waits for confirmation and cancels after a profile change', async () => {
  const removals = [], props = { profileId: '123' }
  const c = component('RustNativeMods.vue', async (command, args) => {
    if (command.endsWith('|rust_client_mods')) return { zoom: true, zoomFactor: 4, zoomKey: 'c', fpsCounter: false, coordinates: false }
    if (args.action === 'list') return JSON.stringify([{ project:'demo.hmod',file:'demo.hmod',slug:'demo',title:'Demo',number:'1' }])
    removals.push(args);return 'Removed'
  }, [], { props })
  await c.state.load()
  c.state.askRemove(['builtin:zoom','local:demo.hmod'])
  assert.equal(removals.length, 0)
  assert.deepEqual(c.state.deleting.value, ['local:demo.hmod'])
  props.profileId = '456'; await c.state.confirmRemove()
  assert.equal(removals.length, 0)
  props.profileId = '123'; c.state.askRemove(['local:demo.hmod']); await c.state.confirmRemove()
  assert.deepEqual(removals, [{ profileId:'123',action:'remove',project:'demo.hmod' }])
})

test('Multi-file mod import preserves a visible scan failure after refreshing installed files', async () => {
  const imported = []
  const c = component('RustNativeMods.vue', async (command,args) => {
    if (command.endsWith('|rust_client_mods')) return { zoom:true,zoomFactor:4,zoomKey:'c',fpsCounter:false,coordinates:false }
    if (command.endsWith('|rust_mods')) return '[]'
    imported.push(args.source)
    if (args.source.endsWith('bad.hmod')) throw Error('File check failed')
    return 'Imported'
  }, [], { props: { profileId:'123' }, open: async()=>['D:/good.hmod','D:/bad.hmod'] })
  await c.state.load(); await c.state.importFile()
  assert.deepEqual(imported,['D:/good.hmod','D:/bad.hmod'])
  assert.match(c.state.error.value,/File check failed/)
  assert.equal(c.state.busy.value,false)
})

test('One-click installation displays a failed automatic audit and never activates the file', async () => {
  const project=catalogFixture('mod'), calls=[]
  const c=component('RustModCatalog.vue',async(command,args)=>{
    calls.push(command)
    if(command.endsWith('|rust_catalog'))return {projects:[project],stale:false}
    return {projectId:project.id,sha256:'1'.repeat(64),installable:false,checks:[{code:'archive',state:'fail',detail:'Unsupported file'}]}
  },[],{props:{profileId:'123',installedFiles:[]}})
  await c.state.load();await c.state.install(project)
  assert.equal(c.state.selectedProject.value.id,project.id)
  assert.equal(c.state.detailTab.value,2)
  assert.equal(calls.filter(command=>command.endsWith('|rust_catalog_install')).length,0)
})

test('Catalog pagination and sorting retain the chosen search when opening a project', async () => {
  const projects=Array.from({length:43},(_,i)=>({...catalogFixture(),id:'demo_'+i,name:'Project '+String(i).padStart(2,'0'),downloads:i}))
  const c=component('RustModCatalog.vue',async()=>({projects,stale:false}),[],{props:{profileId:'123',installedFiles:[]}})
  await c.state.load(); assert.equal(c.state.pageCount.value,3)
  assert.equal(c.state.shown.value[0].downloads,42)
  c.state.switchPage(2);assert.equal(c.state.shown.value.length,20)
  const chosen=c.state.shown.value[0]
  c.state.details(chosen);assert.equal(c.state.selectedProject.value.id,chosen.id)
  c.state.expanded.value='';assert.equal(c.state.page.value,2)
  c.state.order.value='name';await vue.nextTick()
  assert.equal(c.state.page.value,1)
  assert.equal(c.state.shown.value[0].name,'Project 00')
})


test('Native worlds and servers launch the selected destination through native events', async () => {
  const events = [], calls = []
  const worlds = [{ type: 'singleplayer', name: 'Home', path: 'World One', display_status: 'normal', hardcore: false, locked: false, game_mode: 'survival' }, { type: 'server', name: 'Friends', address: 'localhost:25565', index: 0, pack_status: 'prompt', display_status: 'normal' }]
  const c = component('RustWorlds.vue', async (command, args) => { calls.push([command,args]); return worlds }, [], { props: { profileId: '123' }, emit: (...args) => events.push(args) })
  c.state.play(worlds[0]); c.state.play(worlds[1])
  assert.deepEqual(events, [['play', {world:'World One'}], ['play',{server:'localhost:25565'}]])
  await c.state.load()
  c.state.query.value = 'friends'; assert.equal(c.state.shown.value.length, 1)
  c.state.serverName.value = 'New'; c.state.serverAddress.value = 'example.org'
  await c.state.save('add')
  assert.ok(calls.some(([command,args]) => command==='plugin:fre-rust|rust_server_edit' && args.action==='add' && args.address==='example.org'))
  assert.ok(calls.every(([command]) => command.startsWith('plugin:fre-rust|')))
  c.unmount()
})

test('Native console parses log levels, clears CRLF live output and selects saved logs', async () => {
  let manager, content = '[12:00:00] [Render thread/INFO]: Ready\r\n', calls = []
  const c = component('RustLogs.vue', async (command,args) => { calls.push([command,args]); return command.endsWith('|rust_log_sources') ? ['latest.log','2026-11-05-1.log.gz'] : content }, [], { props: { profileId:'123', running:true }, consoleManager: value => {manager=value} })
  await c.state.refresh(true)
  assert.equal(c.state.logLines.value[0].level,'info')
  manager.onClear(); content += '[12:00:01] [worker/WARN]: Warning\r\n'
  await c.state.refresh()
  assert.equal(c.state.logLines.value.length,1)
  assert.equal(c.state.logLines.value[0].level,'warn')
  c.state.selected.value=1; await vue.nextTick(); await new Promise(resolve=>setImmediate(resolve))
  assert.equal(calls.at(-1)[1].filename,'2026-11-05-1.log.gz')
  assert.equal(manager.showCommandInput,false)
  assert.equal(manager.shareDisabled.value,true)
  c.unmount()
})

test('Native catalog source mods can be installed and built directly after automatic checks', async () => {
  const project=catalogFixture(), commands=[], events=[]
  const report={projectId:project.id,version:'1',filename:'demo.hnmod',sha256:'1'.repeat(64),installable:true,checks:[]}
  const c=component('RustModCatalog.vue',async(command,args)=>{commands.push(command);return command.endsWith('|rust_catalog') ? {projects:[project],stale:false} : report},[],{props:{profileId:'123',installedFiles:[]},emit:event=>events.push(event)})
  await c.state.load(); await c.state.install(project)
  assert.ok(commands.includes('plugin:fre-rust|rust_catalog_check'))
  assert.ok(commands.includes('plugin:fre-rust|rust_catalog_install'))
  assert.deepEqual(events,['installed'])
  assert.equal(c.state.isInstalled(project),true)
  c.unmount()
})


test('Catalog instructions use FRE Play while preserving author metadata and feature commands', () => {
  const c=component('RustModCatalog.vue', async()=>({projects:[],stale:false}), [], {props:{profileId:'123'}})
  for (const prefix of ['A native Horizon mod:', '**A native Horizon mod**:']) {
    const project={...catalogFixture(), description:prefix+" the launcher builds it into the game (install it on the launcher's Mods page, then press Build there).\n\nUse /gamma 700% to adjust brightness.\n\nOriginal author ExampleAuthor; MIT."}
    const original=JSON.stringify(project)
    assert.equal(c.state.descriptionForFre(project), 'Use /gamma 700% to adjust brightness.\n\nOriginal author ExampleAuthor; MIT.')
    assert.equal(JSON.stringify(project), original)
  }
  assert.equal(c.state.descriptionForFre(catalogFixture()), 'Source package')
})


test('AppleSkin installs the compatible release while skipping a newer unsupported hook API', async () => {
  const base=catalogFixture(), latest={...base.versions[0],version:'0.1.3',hooks:139,supported:false}, older={...base.versions[0],version:'0.1.2',hooks:1,supported:true,file:{...base.versions[0].file,name:'appleskin-0.1.2.hnmod',sha256:'2'.repeat(64)}}
  const project={...base,id:'appleskin',name:'AppleSkin',versions:[latest,older]},calls=[]
  const report={projectId:project.id,version:older.version,sha256:older.file.sha256,installable:true,checks:[]}
  const c=component('RustModCatalog.vue',async(command,args)=>{calls.push({command,args});return command.endsWith('|rust_catalog')?{projects:[project],stale:false}:report},[],{props:{profileId:'123',installedFiles:[]}})
  await c.state.load();assert.equal(c.state.version(project),older)
  await c.state.install(project)
  const install=calls.find(c=>c.command.endsWith('|rust_catalog_install'))
  assert.equal(install.args.expectedSha256,older.file.sha256)
  assert.ok(c.state.isInstalled(project));assert.equal(c.state.error.value,'')
})
test('Unsupported mods and failed checks produce an explicit outcome without an installation request', async () => {
  const base=catalogFixture(), calls=[]
  const project={...base,versions:[{...base.versions[0],supported:false}]}
  const c=component('RustModCatalog.vue',async(command)=>{calls.push(command);return {installable:false,sha256:base.versions[0].file.sha256,checks:[{code:'compatibility',state:'fail',detail:'Newer API'}]}},[],{props:{profileId:'123'}})
  assert.equal(c.state.version(project),undefined)
  await c.state.install(project);assert.equal(calls.length,0)
  await c.state.install(base)
  assert.equal(c.state.detailTab.value,2);assert.ok(c.state.error.value.includes('blocked'))
  assert.ok(!calls.some(command=>command.endsWith('|rust_catalog_install')))
})
