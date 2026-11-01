import { createHash, createPublicKey, verify } from 'node:crypto'
import { readFile, stat, mkdir, writeFile } from 'node:fs/promises'
import { resolve, dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..')
export const assetName = 'FRE-LAUNCH-Windows-x64-Setup.exe'
const maxBytes = 256 * 1024 * 1024

function decode(value) {
  if (!value || !/^[A-Za-z0-9+/]*={0,2}$/.test(value) || value.length % 4 !== 0) throw Error('Invalid base64')
  return Buffer.from(value, 'base64')
}

// Minisign's documented ED format; use Node/OpenSSL for both Ed25519 checks.
export function verifyTauriSignature(data, signature, publicKey) {
  const keyLines = decode(publicKey.trim()).toString('utf8').trim().split(/\r?\n/)
  const lines = decode(signature.trim()).toString('utf8').trim().split(/\r?\n/)
  if (keyLines.length !== 2 || lines.length !== 4 || !lines[2].startsWith('trusted comment: ')) throw Error('Invalid Minisign wrapper')
  const key = decode(keyLines[1]), signed = decode(lines[1]), global = decode(lines[3])
  if (key.length !== 42 || signed.length !== 74 || global.length !== 64 ||
      key.subarray(0, 2).toString() !== 'Ed' || signed.subarray(0, 2).toString() !== 'ED' ||
      !key.subarray(2, 10).equals(signed.subarray(2, 10))) throw Error('Wrong updater key or signature algorithm')
  const publicKeyObject = createPublicKey({ key: {
    kty: 'OKP', crv: 'Ed25519', x: key.subarray(10).toString('base64url'),
  }, format: 'jwk' })
  const detached = signed.subarray(10)
  const message = createHash('blake2b512').update(data).digest()
  const comment = Buffer.from(lines[2].slice('trusted comment: '.length), 'utf8')
  if (!verify(null, message, publicKeyObject, detached) ||
      !verify(null, Buffer.concat([detached, comment]), publicKeyObject, global)) throw Error('Invalid updater signature')
}

export function createManifest(channel, signature, notes, now = new Date()) {
  if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(channel.version) ||
      !/^[\w-]+\/[\w.-]+$/.test(channel.repository)) throw Error('Invalid stable release channel')
  return { version: channel.version, notes, pub_date: now.toISOString(), platforms: {
    'windows-x86_64': { signature: signature.trim(),
      url: `https://github.com/${channel.repository}/releases/download/v${channel.version}/${assetName}` },
  } }
}

export async function packageUpdate(installer, notesPath, destination) {
  const channel = JSON.parse(await readFile(join(repositoryRoot, 'update-channel.json'), 'utf8'))
  const size = (await stat(installer)).size
  if (size === 0 || size > maxBytes) throw Error('Installer must be between 1 byte and 256 MiB')
  const data = await readFile(installer)
  if (data.subarray(0, 2).toString() !== 'MZ') throw Error('Expected a Windows NSIS installer')
  const signature = (await readFile(`${installer}.sig`, 'utf8')).trim()
  verifyTauriSignature(data, signature, channel.publicKey)
  const notes = await readFile(notesPath, 'utf8')
  if (!notes.trim()) throw Error('Release notes are required')
  const manifest = createManifest(channel, signature, notes)
  const output = destination || join(repositoryRoot, 'release-staging', `v${channel.version}`)
  // An existing stage may already be published. Never replace it silently.
  await mkdir(output, { recursive: false })
  await writeFile(join(output, assetName), data, { flag: 'wx' })
  await writeFile(join(output, `${assetName}.sig`), `${signature}\n`, { flag: 'wx' })
  await writeFile(join(output, 'latest.json'), `${JSON.stringify(manifest, null, 2)}\n`, { flag: 'wx' })
  await writeFile(join(output, 'SHA256SUMS.txt'), `${createHash('sha256').update(data).digest('hex')}  ${assetName}\n`, { flag: 'wx' })
  await writeFile(join(output, 'NOTES.md'), notes, { flag: 'wx' })
  return output
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const [installer, notes, output] = process.argv.slice(2)
    if (!installer || !notes) throw Error('Usage: node scripts/modrinth-fork/package-update.mjs INSTALLER.exe NOTES.md [EMPTY_OUTPUT_DIRECTORY]')
    await mkdir(join(repositoryRoot, 'release-staging'), { recursive: true })
    console.log(await packageUpdate(resolve(installer), resolve(notes), output && resolve(output)))
  } catch (error) { console.error(error.message); process.exitCode = 1 }
}
