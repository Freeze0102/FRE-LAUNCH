const { test } = require('node:test')
const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const { createRequire } = require('node:module')
const workspace = createRequire(path.resolve(process.argv[2] || 'FRE-Modrinth-Source', 'apps/app-frontend/package.json'))
const ts = workspace('typescript')
const code = ts.transpileModule(fs.readFileSync(path.join(__dirname, 'fre-app-updater.ts'), 'utf8'), {
  compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS },
}).outputText
const exportsObject = {}
new Function('exports', code)(exportsObject)
const { createAppUpdater } = exportsObject
const deferred = () => {
  let resolve, reject
  const promise = new Promise((a, b) => { resolve = a; reject = b })
  return { promise, resolve, reject }
}
function fixture(overrides = {}) {
  const calls = []
  const state = Object.fromEntries(Object.entries({ availableUpdate: null, finishedDownloading: false,
    downloading: false, restarting: false, updatesEnabled: false, metered: true, updateSize: null,
    progress: 0 }).map(([key, value]) => [key, { value }]))
  const update = { rid: 7, version: '0.5.9' }
  const options = { state,
    enabled: async () => true,
    check: async () => update,
    metered: async () => false,
    size: async () => 100,
    subscribe: async () => { calls.push('listen'); return () => calls.push('unlisten') },
    download: async () => { calls.push('download') },
    release: async (rid) => { calls.push(`release:${rid}`) },
    ready: () => calls.push('ready'),
    available: () => calls.push('available'),
    restart: async (value) => { calls.push(`restart:${value}`) },
    close: async () => { calls.push('close') },
    error: () => calls.push('error'), warn: () => calls.push('warn'), ...overrides }
  const updater = createAppUpdater(options)
  return { state, calls, updater, update, options }
}

test('Startup subscribes before downloading, stages once, and waits for the restart click', async () => {
  const f = fixture()
  try {
    await f.updater.start()
    await f.updater.start()
    await f.updater.check()
    await f.updater.download()
    assert.deepEqual(f.calls, ['listen', 'download', 'ready', 'unlisten'])
    assert.equal(f.state.finishedDownloading.value, true)
    assert.equal(f.state.progress.value, 1)
    await f.updater.install()
    await f.updater.install()
    assert.deepEqual(f.calls.slice(-2), ['restart:true', 'close'])
  } finally { f.updater.stop() }
})

test('Signature/download failure leaves a visible retry instead of a ready update', async () => {
  let attempts = 0
  const f = fixture({ download: async () => { if (++attempts === 1) throw Error('Invalid signature') } })
  try {
    await f.updater.start()
    assert.equal(f.state.finishedDownloading.value, false)
    assert.equal(f.state.downloading.value, false)
    assert.equal(f.state.progress.value, 0)
    await f.updater.install()
    assert.equal(f.calls.includes('close'), false)
    assert.equal(f.calls.includes('restart:true'), false)
    await f.updater.download()
    assert.equal(f.state.finishedDownloading.value, true)
    assert.equal(f.calls.filter(c => c === 'ready').length, 1)
  } finally { f.updater.stop() }
})

test('Offline startup continues polling and recovers without reopening the app', async () => {
  let checks = 0
  const f = fixture({ check: async () => { if (++checks === 1) throw Error('offline'); return f.update }, intervalMs: 5 })
  try {
    await f.updater.start()
    assert.deepEqual(f.calls, ['warn'])
    await new Promise(resolve => setTimeout(resolve, 50))
    assert.equal(f.state.finishedDownloading.value, true)
  } finally { f.updater.stop() }
})

test('Metered network requires one manual download, not repeated popups', async () => {
  const f = fixture({ metered: async () => true })
  try {
    await f.updater.start()
    await f.updater.check()
    assert.deepEqual(f.calls, ['available'])
    await f.updater.download()
    assert.equal(f.state.finishedDownloading.value, true)
  } finally { f.updater.stop() }
})

test('Failed window close rolls back the restart flag and allows another click', async () => {
  let closes = 0
  const f = fixture({ close: async () => { if (++closes === 1) throw Error('cannot save window state') } })
  try {
    await f.updater.start()
    await f.updater.install()
    assert.equal(f.state.restarting.value, false)
    assert.ok(f.calls.includes('restart:false'))
    await f.updater.install()
    assert.equal(closes, 2)
  } finally { f.updater.stop() }
})

test('Duplicate requests and stopping during a check cannot download or show stale UI', async () => {
  const pending = deferred()
  let checks = 0
  const f = fixture({ check: () => { checks++; return pending.promise } })
  const first = f.updater.check()
  await f.updater.check()
  assert.equal(checks, 1)
  f.updater.stop()
  pending.resolve(f.update)
  await first
  assert.deepEqual(f.calls, ['release:7'])
})

test('An active check cannot swap the installer underneath a manual download', async () => {
  const pending = deferred()
  const f = fixture({ check: () => pending.promise, metered: async () => true })
  f.state.availableUpdate.value = { rid: 1, version: '0.5.8' }
  try {
    const checking = f.updater.check()
    await f.updater.download()
    assert.equal(f.calls.includes('download'), false)
    pending.resolve(f.update)
    await checking
    await f.updater.download()
    assert.deepEqual(f.calls, ['release:1', 'available', 'listen', 'download', 'ready', 'unlisten'])
  } finally { f.updater.stop() }
})

test('Development/disabled builds never check, download, or restart', async () => {
  const f = fixture({ enabled: async () => false, check: async () => { throw Error('must not check') } })
  try {
    await f.updater.start()
    assert.deepEqual(f.calls, [])
    assert.equal(f.state.updatesEnabled.value, false)
  } finally { f.updater.stop() }
})

test('A slow size HEAD request never blocks the automatic download', async () => {
  const pending = deferred()
  const f = fixture({ size: () => pending.promise })
  try {
    await f.updater.start()
    assert.equal(f.state.finishedDownloading.value, true)
    assert.equal(f.state.updateSize.value, null)
    f.updater.stop()
    pending.resolve(999)
    await Promise.resolve()
    assert.equal(f.state.updateSize.value, null)
  } finally { f.updater.stop() }
})
