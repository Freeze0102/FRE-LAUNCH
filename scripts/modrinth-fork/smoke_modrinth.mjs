// Optional live, read-only smoke: search -> compatible version -> verified JAR.
// Downloads into memory only. Never imports a mod or starts a launcher/game.
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'

async function get(url) {
  const response = await fetch(url, {
    headers: { 'User-Agent': 'FRE-LAUNCH-local-audit/1.0 (Freeze0102/FRE-LAUNCH)' },
    signal: AbortSignal.timeout(30_000),
  })
  assert.ok(response.ok, `${url}: HTTP ${response.status}`)
  return response
}

const version = '1.21.1'
const search = new URL('https://api.modrinth.com/v3/search')
search.search = new URLSearchParams({ query: 'fabric api', limit: '3', index: 'relevance',
  new_filters: `project_types = "mod" AND game_versions = "${version}" AND categories = "fabric"` })
const results = await (await get(search)).json()
assert.ok(results.hits.length)
const project = results.hits[0]
assert.equal(typeof project.name, 'string')
const versions = new URL(`https://api.modrinth.com/v2/project/${project.project_id}/version`)
versions.search = new URLSearchParams({ game_versions: JSON.stringify([version]), loaders: JSON.stringify(['fabric']) })
const compatible = await (await get(versions)).json()
assert.ok(compatible.length)
const selected = compatible[0]
assert.ok(selected.game_versions.includes(version))
assert.ok(selected.loaders.includes('fabric'))
const file = selected.files.find(f => f.primary) || selected.files[0]
assert.equal(new URL(file.url).hostname, 'cdn.modrinth.com')
assert.match(file.hashes.sha512, /^[a-f0-9]{128}$/)
assert.ok(file.size > 0 && file.size <= 128 * 1024 * 1024)
const bytes = Buffer.from(await (await get(file.url)).arrayBuffer())
assert.equal(bytes.length, file.size)
assert.equal(createHash('sha512').update(bytes).digest('hex'), file.hashes.sha512)
assert.equal(bytes.readUInt32LE(0), 0x04034b50)
console.log(JSON.stringify({ searchApi: 'v3', project: project.slug, version: selected.version_number,
  minecraft: version, loader: 'fabric', jar: file.filename, bytes: bytes.length,
  sha512: 'verified', runtimeInstall: 'not tested' }, null, 2))
