// Anonymous real-channel check. Downloads and verifies bytes; never runs an installer.
import { readFile } from 'node:fs/promises'
import { createHash } from 'node:crypto'
import { fileURLToPath } from 'node:url'
import { resolve } from 'node:path'
import { verifyTauriSignature, assetName } from './package-update.mjs'

const root = fileURLToPath(new URL('../../', import.meta.url))
const channel = JSON.parse(await readFile(resolve(root, 'update-channel.json'), 'utf8'))
const current = process.argv[2]
if (!/^\d+\.\d+\.\d+$/.test(current || '')) throw Error('Pass the installed stable version, for example 0.5.8')
const endpoint = `https://github.com/${channel.repository}/releases/latest/download/latest.json`
const response = await fetch(endpoint, { signal: AbortSignal.timeout(30_000) })
if (!response.ok) throw Error(`Anonymous manifest request failed: HTTP ${response.status}`)
const manifestText = await response.text()
if (manifestText.length > 256 * 1024) throw Error('Manifest exceeds the expected size')
const manifest = JSON.parse(manifestText)
if (manifest.version !== channel.version) throw Error('Latest release differs from the prepared source')
const platform = manifest.platforms?.['windows-x86_64']
const expectedUrl = `https://github.com/${channel.repository}/releases/download/v${manifest.version}/${assetName}`
if (!platform || platform.url !== expectedUrl) throw Error('Unexpected Windows updater URL')
const before = current.split('.').map(Number), after = manifest.version.split('.').map(Number)
const differing = after.findIndex((value, i) => value !== before[i])
if (differing < 0 || after[differing] < before[differing]) throw Error('Release is not newer than the installed version')
const installer = await fetch(platform.url, { signal: AbortSignal.timeout(300_000) })
if (!installer.ok) throw Error(`Anonymous installer request failed: HTTP ${installer.status}`)
const maximum = 256 * 1024 * 1024
const declared = installer.headers.get('content-length')
if (declared && (!Number.isSafeInteger(Number(declared)) || Number(declared) < 1 || Number(declared) > maximum)) throw Error('Invalid installer size')
const chunks = []
let bytes = 0
for await (const chunk of installer.body) {
  bytes += chunk.length
  if (bytes > maximum) throw Error('Installer exceeds the updater size limit')
  chunks.push(chunk)
}
const data = Buffer.concat(chunks)
if (data.subarray(0, 2).toString() !== 'MZ' || (declared && bytes !== Number(declared))) throw Error('Truncated or invalid Windows installer')
verifyTauriSignature(data, platform.signature, channel.publicKey)
const sha256 = createHash('sha256').update(data).digest('hex')
const provenance = JSON.parse(await readFile(resolve(root, `release-staging/v${channel.version}/PROVENANCE.json`), 'utf8'))
if (sha256 !== provenance.installer.sha256) throw Error('Published bytes differ from the scanned local artifact')
console.log(JSON.stringify({ currentVersion: current, availableVersion: manifest.version,
  anonymousManifest: true, eligible: true, bytes, sha256, signatureVerified: true,
  matchesScannedArtifact: true, installerExecuted: false }, null, 2))
