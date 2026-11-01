// Explicit local release workflow. No Actions, signing-key upload, asset replacement or token logging.
import { execFileSync } from 'node:child_process'
import { readFile } from 'node:fs/promises'
import { createHash } from 'node:crypto'
import { fileURLToPath } from 'node:url'
import { resolve } from 'node:path'
import { assetName, verifyTauriSignature } from './package-update.mjs'

const root = fileURLToPath(new URL('../../', import.meta.url))
const channel = JSON.parse(await readFile(resolve(root, 'update-channel.json'), 'utf8'))
if (channel.repository !== 'Freeze0102/FRE-LAUNCH' || !/^0\.\d+\.\d+$/.test(channel.version)) throw Error('Unexpected release channel')
const stage = resolve(root, `release-staging/v${channel.version}`)
const names = [assetName, `${assetName}.sig`, 'latest.json', 'SHA256SUMS.txt', 'PROVENANCE.json']
const files = new Map(await Promise.all(names.map(async name => [name, await readFile(resolve(stage, name))])))
const notes = await readFile(resolve(stage, 'NOTES.md'), 'utf8')
const manifest = JSON.parse(files.get('latest.json'))
const provenance = JSON.parse(files.get('PROVENANCE.json'))
const sha = bytes => createHash('sha256').update(bytes).digest('hex')
const installer = files.get(assetName)
const signature = files.get(`${assetName}.sig`).toString().trim()
verifyTauriSignature(installer, signature, channel.publicKey)
if (manifest.version !== channel.version || manifest.platforms?.['windows-x86_64']?.signature !== signature ||
    manifest.platforms['windows-x86_64'].url !== `https://github.com/${channel.repository}/releases/download/v${channel.version}/${assetName}` ||
    provenance.installer.sha256 !== sha(installer) || !provenance.installer.signatureVerified ||
    provenance.defender?.active !== true || provenance.defender.threatsFound !== 0 ||
    provenance.defender.launcherExitCode !== 0 || provenance.defender.installerExitCode !== 0 ||
    files.get('SHA256SUMS.txt').toString().trim() !== `${sha(installer)}  ${assetName}` ||
    !/^[0-9a-f]{40}$/.test(provenance.sourceCommit) || !notes.includes(provenance.sourceCommit)) {
  throw Error('Release validation failed; nothing was uploaded')
}
execFileSync('git', ['merge-base', '--is-ancestor', provenance.sourceCommit, 'HEAD'], { cwd: root, stdio: 'pipe' })
if (process.argv.includes('--dry-run')) {
  console.log(JSON.stringify({ version: channel.version, source: provenance.sourceCommit,
    assets: names.map(name => ({ name, bytes: files.get(name).length, sha256: sha(files.get(name)) })), uploaded: false }, null, 2))
  process.exit(0)
}
let raw
try {
  raw = execFileSync('git', ['credential', 'fill'], { cwd: root,
    input: 'protocol=https\nhost=github.com\npath=Freeze0102/FRE-LAUNCH.git\n\n', encoding: 'utf8',
    env: { ...process.env, GIT_TERMINAL_PROMPT: '0', GCM_INTERACTIVE: 'never' }, timeout: 20_000,
    stdio: ['pipe', 'pipe', 'pipe'] })
} catch { throw Error('Existing GitHub authorization is unavailable') }
const token = raw.split(/\r?\n/).find(line => line.startsWith('password='))?.slice(9)
if (!token) throw Error('Existing GitHub authorization is unavailable')
const headers = { Authorization: `Bearer ${token}`, Accept: 'application/vnd.github+json', 'User-Agent': 'FRE-LAUNCH-local-release' }
const api = `https://api.github.com/repos/${channel.repository}`
async function request(url, method = 'GET', body) {
  const response = await fetch(url, { method, headers: { ...headers, ...(body ? { 'Content-Type': 'application/json' } : {}) },
    body: body && JSON.stringify(body), signal: AbortSignal.timeout(30_000), redirect: 'error' })
  if (response.status === 404 && method === 'GET') return null
  if (!response.ok) throw Error(`GitHub ${method} failed: HTTP ${response.status}`)
  return response.json()
}
// The source must already be accessible in this repository, before creating its tag.
if (!(await request(`${api}/git/commits/${provenance.sourceCommit}`))) throw Error('Push the exact reviewed source before releasing')
const tag = `v${channel.version}`
const ref = await request(`${api}/git/ref/tags/${tag}`)
if (ref && (ref.object?.type !== 'commit' || ref.object.sha !== provenance.sourceCommit)) {
  throw Error('Existing tag does not directly identify the reviewed source; nothing was changed')
}
const releases = await request(`${api}/releases?per_page=100`)
const matching = releases.filter(release => release.tag_name === tag)
if (matching.length > 1) throw Error('Ambiguous existing release tag; nothing was changed')
let release = matching[0]
if (release && (!release.draft || release.target_commitish !== provenance.sourceCommit)) {
  throw Error('Existing release is published or targets another source; it was not changed')
}
if (release && release.body !== notes) throw Error('Existing draft notes differ from the reviewed local notes')
if (!release) release = await request(`${api}/releases`, 'POST', {
  tag_name: tag, target_commitish: provenance.sourceCommit,
  name: `FRE LAUNCH ${channel.version} — Horizon setup and signed updates`,
  body: notes, draft: true, prerelease: false,
})
for (const name of names) {
  const bytes = files.get(name)
  const previous = release.assets.find(asset => asset.name === name)
  if (previous) {
    if (previous.digest !== `sha256:${sha(bytes)}`) throw Error(`Draft asset ${name} differs; nothing was replaced`)
    continue
  }
  const url = `https://uploads.github.com/repos/${channel.repository}/releases/${release.id}/assets?name=${encodeURIComponent(name)}`
  const response = await fetch(url, { method: 'POST', headers: { ...headers, 'Content-Type': 'application/octet-stream' },
    body: bytes, signal: AbortSignal.timeout(300_000), redirect: 'error' })
  if (!response.ok) throw Error(`Uploading ${name} failed: HTTP ${response.status}; release remains a draft`)
  const asset = await response.json()
  if (asset.digest !== `sha256:${sha(bytes)}` || asset.size !== bytes.length) throw Error('Uploaded bytes differ; release remains a draft')
}
release = await request(`${api}/releases/${release.id}`)
if (names.some(name => release.assets.find(asset => asset.name === name)?.digest !== `sha256:${sha(files.get(name))}`)) throw Error('Draft verification failed')
if (process.argv.includes('--publish')) {
  release = await request(`${api}/releases/${release.id}`, 'PATCH', { draft: false, make_latest: 'true' })
}
console.log(JSON.stringify({ id: release.id, tag: release.tag_name, draft: release.draft,
  url: release.html_url, assets: release.assets.map(asset => ({ name: asset.name, bytes: asset.size, digest: asset.digest })) }, null, 2))
