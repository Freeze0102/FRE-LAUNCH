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
  const text = fs.readFileSync(path.join(__dirname, file), 'utf8')
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
    '@modrinth/ui': { defineMessages: x => x, useVIntl: () => ({ formatMessage: (m, args = {}) =>
      m.defaultMessage.replace(/\{(\w+)\}/g, (_, k) => String(args[k])) }) },
    vue: { ...vue, onMounted: f => mounted.push(f), onUnmounted: f => unmounted.push(f) },
  }
  const names = file === 'CurseForgeBrowser.vue'
    ? 'instanceId,searchText,apiKey,results,loading,notice,error,installing,doSearch,install'
    : file === 'FreRustPanel.vue'
      ? 'source,name,selectedId,profiles,runtime,pending,error,notice,chooseSourceFolder,importResources,openProfileFiles,create,refresh,show,hide,visible'
      : file === 'FreRustLibrary.vue'
        ? 'profiles,runtime,error,refresh,play,openFiles,openRustWizard,loadFiles,closeFiles,fileEntries,fileCategory,fileProfile'
        : file === 'HorizonLibrary.vue'
          ? 'profiles,status,executable,error,refresh,openFiles,openHorizonWizard,refreshFiles,closeFiles,openExplorer,fileEntries,fileCategory,fileProfile'
          : 'minecraftPath,executable,instanceName,launchName,pending,error,feedback,status,prepared,canPlay,activeVersion,releaseTag,downloadProgress,manageHorizon,createInstance,playInstance,downloadLauncher,show,hide,visible'
  const setup = new Function('require', 'defineProps', 'defineExpose', 'localStorage', 'window', 'Event', 'exports',
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
  }, () => options.props || ({}), () => {}, { getItem: () => null, setItem: () => {} }, browser, Event, {})
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
    if (command.endsWith('|rust_open_folder')) return undefined
    throw new Error(command)
  })
  await c.state.refresh()
  await c.state.openFiles(profile)
  assert.deepEqual(calls.at(-1), { command: 'plugin:fre-rust|rust_open_folder', args: { profileId: profile.id, folder: 'root' } })
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

test('Existing Horizon Files are listed inside the app and Explorer remains optional', async () => {
  const calls = []
  const profile = { id: 'horizon', name: 'Old Rust game', version: null, selected: true }
  const c = component('HorizonLibrary.vue', async (command, args) => {
    calls.push({ command, args })
    if (command.endsWith('|horizon_detect_launcher')) return 'C:\\Horizon\\horizon-launcher.exe'
    if (command.endsWith('|horizon_status')) return { instances: [profile], installedVersions: ['nightly'], preparedVersions: ['nightly'], selectedVersion: 'nightly' }
    if (command.endsWith('|horizon_list_files')) return [{ name: 'native-mod.bin', bytes: 1024, isDirectory: false }]
    if (command.endsWith('|horizon_open_folder')) return undefined
    throw new Error(command)
  })
  await c.state.refresh()
  await c.state.openFiles(profile)
  assert.deepEqual(calls.at(-1), {
    command: 'plugin:fre-horizon|horizon_list_files',
    args: { exePath: 'C:\\Horizon\\horizon-launcher.exe', name: profile.id, folder: 'mods' },
  })
  assert.equal(c.state.fileEntries.value[0].name, 'native-mod.bin')
  await c.state.openExplorer()
  assert.equal(calls.at(-1).command, 'plugin:fre-horizon|horizon_open_folder')
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

test('Horizon forwards the optional Minecraft folder using the official prepare flag input', async () => {
  const calls = []
  const c = component('HorizonRustPanel.vue', async (command, args) => {
    calls.push({ command, args }); return command.endsWith('|horizon_status') ? horizonStatus : 'prepared'
  })
  c.state.executable.value = 'C:\\Horizon\\horizon-launcher.exe'
  c.state.minecraftPath.value = 'D:\\My Minecraft'
  await c.state.manageHorizon('prepare')
  assert.equal(calls[0].command, 'plugin:fre-horizon|horizon_prepare')
  assert.equal(calls[0].args.minecraftPath, 'D:\\My Minecraft')
  assert.equal(c.state.pending.value, null)
})

test('Horizon prevents duplicate launch requests and shows CLI failures', async () => {
  const request = deferred()
  let calls = 0
  const c = component('HorizonRustPanel.vue', () => { calls++; return request.promise })
  c.state.executable.value = 'C:\\Horizon\\horizon-launcher.exe'
  c.state.launchName.value = 'Survival'
  c.state.status.value = { ...horizonStatus, instances: [{ id: 'Survival', name: 'Survival', version: null }] }
  const play = c.state.playInstance()
  await c.state.playInstance()
  assert.equal(calls, 1)
  request.reject(new Error('Missing installed version'))
  await play
  assert.match(c.state.error.value, /Missing installed version/)
  assert.equal(c.state.pending.value, null)
})

const horizonStatus = { instances: [{ id: 'horizon', name: 'Horizon', version: null, selected: true }],
  installedVersions: ['nightly-test'], preparedVersions: ['nightly-test'], selectedVersion: 'nightly-test' }

test('Horizon downloads its launcher once, listens before starting, and selects the real default instance', async () => {
  const order = [], request = deferred()
  let listener, unlistened = 0
  const c = component('HorizonRustPanel.vue', (command) => {
    order.push(command)
    return command.endsWith('|horizon_install_launcher') ? request.promise : Promise.resolve(horizonStatus)
  }, [], { listen: async (event, callback) => { order.push(event); listener = callback; return () => unlistened++ } })
  const first = c.state.downloadLauncher()
  await Promise.resolve()
  await c.state.downloadLauncher()
  assert.equal(order[0], 'fre-horizon-bootstrap-progress')
  assert.equal(order.filter(x => x.endsWith('|horizon_install_launcher')).length, 1)
  listener({ payload: 55 })
  assert.equal(c.state.downloadProgress.value, 55)
  request.resolve('C:\\managed\\horizon-launcher.exe')
  await first
  assert.equal(c.state.executable.value, 'C:\\managed\\horizon-launcher.exe')
  assert.equal(c.state.launchName.value, 'horizon')
  assert.equal(c.state.canPlay.value, true)
  assert.equal(c.state.pending.value, null)
  assert.equal(unlistened, 1)
})

test('Horizon never offers Play before the selected instance version is prepared', async () => {
  const calls = []
  const c = component('HorizonRustPanel.vue', async command => calls.push(command))
  c.state.executable.value = 'C:\\managed\\horizon-launcher.exe'
  c.state.status.value = { ...horizonStatus, preparedVersions: [] }
  assert.equal(c.state.canPlay.value, false)
  await c.state.playInstance()
  assert.equal(calls.length, 0)
  c.state.status.value = { ...horizonStatus, instances: [{ id: 'horizon', name: 'Horizon', version: 'older' }] }
  assert.equal(c.state.activeVersion.value, 'older')
  assert.equal(c.state.canPlay.value, false)
})

test('Horizon uses a selected pinned version for preparation and reports bootstrap errors without advancing', async () => {
  const calls = []
  const c = component('HorizonRustPanel.vue', async (command, args) => {
    calls.push({ command, args })
    if (command.endsWith('|horizon_install_launcher')) throw new Error('SHA-256 mismatch')
    return command.endsWith('|horizon_status') ? horizonStatus : 'prepared'
  })
  await c.state.downloadLauncher()
  assert.match(c.state.error.value, /SHA-256 mismatch/)
  assert.equal(c.state.status.value, null)
  assert.equal(c.state.executable.value, '')
  c.state.executable.value = 'C:\\managed\\horizon-launcher.exe'
  c.state.status.value = { ...horizonStatus, instances: [{ id: 'horizon', name: 'Horizon', version: 'older' }] }
  await c.state.manageHorizon('prepare')
  assert.equal(calls.find(x => x.command.endsWith('|horizon_prepare')).args.version, 'older')
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
