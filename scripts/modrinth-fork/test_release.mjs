import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { createManifest, verifyTauriSignature } from './package-update.mjs'

const fixture = JSON.parse(await readFile(new URL('./fre-update-test.json', import.meta.url), 'utf8'))
const channel = JSON.parse(await readFile(new URL('../../update-channel.json', import.meta.url), 'utf8'))

test('Real Tauri signer fixture verifies with the channel key; corruption cannot be published', () => {
  const data = Buffer.from(fixture.data)
  verifyTauriSignature(data, fixture.signature, channel.publicKey)
  assert.throws(() => verifyTauriSignature(Buffer.from('tampered'), fixture.signature, channel.publicKey))
  const signature = Buffer.from(fixture.signature, 'base64').toString().split('\n')
  signature[2] += ' modified'
  assert.throws(() => verifyTauriSignature(data, Buffer.from(signature.join('\n')).toString('base64'), channel.publicKey))
  const key = Buffer.from(channel.publicKey, 'base64').toString().split('\n')
  const rawKey = Buffer.from(key[1], 'base64')
  rawKey[15] ^= 1
  key[1] = rawKey.toString('base64')
  assert.throws(() => verifyTauriSignature(data, fixture.signature, Buffer.from(key.join('\n')).toString('base64')))
})

test('Static Tauri manifest targets the exact versioned public Windows asset', () => {
  const manifest = createManifest(channel, fixture.signature, 'Release notes', new Date('2026-11-01T12:00:00Z'))
  assert.equal(manifest.version, channel.version)
  assert.equal(manifest.pub_date, '2026-11-01T12:00:00.000Z')
  assert.equal(manifest.platforms['windows-x86_64'].url,
    `https://github.com/${channel.repository}/releases/download/v${channel.version}/FRE-LAUNCH-Windows-x64-Setup.exe`)
  assert.equal(manifest.platforms['windows-x86_64'].signature, fixture.signature)
  for (const version of ['v1.0.0', '1.0.0-local', '../main', '01.0.0']) {
    assert.throws(() => createManifest({ ...channel, version }, fixture.signature, 'Notes'))
  }
})
