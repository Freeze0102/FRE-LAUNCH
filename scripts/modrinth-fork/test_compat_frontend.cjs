const { test } = require('node:test')
const assert = require('node:assert/strict')
const fs = require('node:fs')
const path = require('node:path')
const { createRequire } = require('node:module')
const source = path.resolve(process.argv[2] || 'FRE-Modrinth-Source')
const workspace = createRequire(path.join(source, 'apps/app-frontend/package.json'))
const ts = workspace('typescript')
const compiler = workspace('@vue/compiler-sfc')
const transpile = text => ts.transpileModule(text, {
  compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS },
}).outputText

function functions(file, names, bindings, prefix = '') {
  const text = fs.readFileSync(path.join(source, file), 'utf8')
  const setup = file.endsWith('.vue') ? compiler.parse(text).descriptor.scriptSetup.content : text
  const ast = ts.createSourceFile(file, setup, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS)
  const declarations = names.map(name => {
    const node = ast.statements.find(s => ts.isFunctionDeclaration(s) && s.name?.text === name)
    assert.ok(node, `${file}: ${name}`)
    return node.getText(ast).replace(/^export /, '')
  }).join('\n')
  return new Function(...Object.keys(bindings), `${transpile(prefix + declarations)}\nreturn {${names}}`)(...Object.values(bindings))
}

function installPrompt(getVersion) {
  const calls = []
  const state = { modpackVersion: { value: null }, requiredContentProject: { value: null }, externalFiles: { value: ['stale.jar'] }, project: { value: null } }
  const bindings = { ...state,
    get_version: getVersion,
    get_project: async id => ({ id }),
    hide: () => calls.push('hide'),
    startInstallingServer: id => calls.push(`start:${id}`),
    stopInstallingServer: id => calls.push(`stop:${id}`),
    installServerProject: async id => calls.push(`install:${id}`),
    onInstallComplete: { value: () => calls.push('complete') },
  }
  return { ...functions('apps/app-frontend/src/components/ui/modal/InstallToPlayModal.vue', ['fetchData', 'handleAccept'], bindings, 'let fetchGeneration = 0;\n'), state, calls }
}

test('Embedded modpack files accept real nullable Modrinth dependency IDs', async () => {
  const p = installPrompt(async () => ({ project_id: 'pack', dependencies: [
    { dependency_type: 'embedded', project_id: null, version_id: null, file_name: 'Manual.jar' },
    { dependency_type: 'embedded', project_id: 'known', version_id: null, file_name: 'Known.jar' },
    { dependency_type: 'embedded', project_id: null, version_id: null, file_name: null },
  ] }))
  await p.fetchData('version')
  assert.deepEqual(p.state.externalFiles.value, ['Manual.jar'])
})

test('An old modpack response cannot replace a newer selection', async () => {
  let resolveOld
  const old = new Promise(resolve => { resolveOld = resolve })
  const p = installPrompt(id => id === 'old' ? old : Promise.resolve({ project_id: 'new', dependencies: [] }))
  const first = p.fetchData('old')
  await p.fetchData('new')
  resolveOld({ project_id: 'old', dependencies: [] })
  await first
  assert.equal(p.state.requiredContentProject.value.id, 'new')
  assert.deepEqual(p.state.externalFiles.value, [])
})

test('A missing server project cannot close the prompt or start an undefined installation', async () => {
  const p = installPrompt(async () => null)
  await p.handleAccept()
  assert.deepEqual(p.calls, [])
  p.state.project.value = { id: 'server' }
  await p.handleAccept()
  assert.deepEqual(p.calls, ['hide', 'start:server', 'install:server', 'complete', 'stop:server'])
})

test('Display names support unnamed synced servers and singleplayer worlds', () => {
  const names = functions('apps/app-frontend/src/helpers/worlds.ts', ['getWorldDisplayName'], { stripMinecraftFormatting: text => text.replace(/§./g, '') })
  assert.equal(names.getWorldDisplayName({ name: '§a', address: 'example.test' }), 'example.test')
  assert.equal(names.getWorldDisplayName({ name: '§aMy world', type: 'singleplayer', path: 'worlds/a' }), 'My world')
  assert.equal(names.getWorldDisplayName({ name: '', type: 'singleplayer', path: 'worlds/b' }), 'worlds/b')
})

test('Notification failures accept native strings, structured errors, and null', () => {
  const exports = {}
  const text = fs.readFileSync(path.join(source, 'packages/ui/src/providers/web-notifications.ts'), 'utf8')
  new Function('require', 'exports', transpile(text))(name => {
    if (name === '@tanstack/vue-query') return { isCancelledError: e => e?.cancelled === true }
    if (name === '.') return { createContext: () => [() => {}, () => {}] }
    throw Error(name)
  }, exports)
  const manager = new (class extends exports.AbstractWebNotificationManager {})()
  const notices = []
  manager.addNotification = notice => notices.push(notice)
  for (const error of ['timeout', { message: 'native failure' }, new Error('network'), null, { cancelled: true }]) manager.handleError(error)
  assert.deepEqual(notices.map(n => n.text), ['timeout', 'native failure', 'network', 'null'])
})

test('Unknown tag values cannot resolve inherited object properties as translations', () => {
  const exports = {}
  const text = fs.readFileSync(path.join(source, 'packages/ui/src/utils/tag-messages.ts'), 'utf8')
  new Function('require', 'exports', transpile(text))(name => {
    if (name === '@modrinth/utils') return { capitalizeString: text => text }
    return { defineMessages: x => x }
  }, exports)
  assert.equal(exports.getTagMessage('fabric').defaultMessage, 'Fabric')
  for (const value of ['__proto__', 'constructor', 'unknown-future-tag']) {
    assert.equal(exports.getTagMessage(value), undefined)
    assert.equal(exports.getCategoryHeaderMessage(value), undefined)
  }
})
