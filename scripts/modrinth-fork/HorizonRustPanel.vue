<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { openUrl } from '@tauri-apps/plugin-opener'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { ref } from 'vue'

const { formatMessage: t } = useVIntl()
const messages = defineMessages({
  title: { id: 'fre-launch.horizon.title', defaultMessage: 'Horizon (Rust)' },
  subtitle: { id: 'fre-launch.horizon.subtitle', defaultMessage: 'A separate Rust-based Minecraft client. Horizon versions, mods, worlds and updates are managed independently of Java instances.' },
  requirement: { id: 'fre-launch.horizon.requirement', defaultMessage: 'Requires the official Horizon Windows launcher and Minecraft: Java Edition 26.3 installed and opened once. Microsoft sign-in may not work in Horizon yet; offline single player is available. Java Fabric/Forge mods are not supported.' },
  launcherLabel: { id: 'fre-launch.horizon.launcher-label', defaultMessage: 'Horizon executable' },
  chooseExe: { id: 'fre-launch.horizon.choose-exe', defaultMessage: 'Choose horizon-launcher.exe' },
  release: { id: 'fre-launch.horizon.release', defaultMessage: 'Official download and installation instructions' },
  name: { id: 'fre-launch.horizon.name', defaultMessage: 'New Horizon instance name' },
  namePlaceholder: { id: 'fre-launch.horizon.placeholder', defaultMessage: 'My Rust Minecraft' },
  create: { id: 'fre-launch.horizon.create', defaultMessage: 'Create Horizon instance' },
  creating: { id: 'fre-launch.horizon.creating', defaultMessage: 'Creating...' },
  launchName: { id: 'fre-launch.horizon.launch-name', defaultMessage: 'Instance to launch (name)' },
  play: { id: 'fre-launch.horizon.play', defaultMessage: 'Play Horizon' },
  playing: { id: 'fre-launch.horizon.playing', defaultMessage: 'Launching...' },
  list: { id: 'fre-launch.horizon.list', defaultMessage: 'Show Horizon instances' },
  logs: { id: 'fre-launch.horizon.logs', defaultMessage: 'Show Horizon logs' },
  logPrivacy: { id: 'fre-launch.horizon.log-privacy', defaultMessage: 'Logs are read locally only; nothing is uploaded.' },
  versions: { id: 'fre-launch.horizon.versions', defaultMessage: 'Horizon releases' },
  showVersions: { id: 'fre-launch.horizon.show-versions', defaultMessage: 'Show installed versions' },
  releaseTag: { id: 'fre-launch.horizon.release-tag', defaultMessage: 'Horizon version or release tag' },
  installVersion: { id: 'fre-launch.horizon.install-version', defaultMessage: 'Install this Horizon version' },
  useVersion: { id: 'fre-launch.horizon.use-version', defaultMessage: 'Use version for selected instance' },
  prepare: { id: 'fre-launch.horizon.prepare', defaultMessage: 'Prepare Minecraft 26.3 assets' },
  firstRun: { id: 'fre-launch.horizon.first-run', defaultMessage: 'First install a Horizon version, then prepare game assets from a Minecraft 26.3 installation before Play.' },
  closing: { id: 'fre-launch.horizon.close', defaultMessage: 'Close' },
  notConfigured: { id: 'fre-launch.horizon.not-configured', defaultMessage: 'Choose the Horizon launcher executable first.' },
  error: { id: 'fre-launch.horizon.error', defaultMessage: 'Horizon error: {reason}' },
  about: { id: 'fre-launch.horizon.about', defaultMessage: 'Horizon is maintained by its own developers. FRE LAUNCH starts its official CLI; it does not change or repair Horizon game code.' },
  separate: { id: 'fre-launch.horizon.separate', defaultMessage: 'Existing Minecraft Java instances, mods and worlds remain untouched.' },
})

const visible = ref(false)
const executable = ref('')
const instanceName = ref('')
const launchName = ref('')
const listing = ref('')
const logs = ref('')
const versions = ref('')
const releaseTag = ref('latest')
const feedback = ref('')
const error = ref('')
const pending = ref<'create' | 'play' | 'list' | 'logs' | 'versions' | 'install-version' | 'set-version' | 'prepare' | null>(null)

try {
  const saved = localStorage.getItem('fre-launch-horizon-executable')
  if (saved) executable.value = saved
} catch {
  // The file picker remains available even if browser storage is disabled.
}

function show() {
  visible.value = true
  feedback.value = ''
  error.value = ''
}

function hide() {
  if (pending.value) return
  visible.value = false
}

async function selectExe() {
  try {
    const picked = await open({
      multiple: false,
      directory: false,
      filters: [{ name: 'Horizon Windows launcher', extensions: ['exe'] }],
    })
    if (typeof picked !== 'string') return
    if (!/[\\/]horizon-launcher\.exe$/i.test(picked)) {
      error.value = 'Select horizon-launcher.exe from the official Horizon Windows archive.'
      return
    }
    executable.value = picked
    try { localStorage.setItem('fre-launch-horizon-executable', picked) } catch { /* optional */ }
    error.value = ''
  } catch (err) {
    error.value = String(err)
  }
}

function failure(err: unknown): string {
  return err instanceof Error ? err.message : String(err)
}

async function createInstance() {
  if (!executable.value) { error.value = t(messages.notConfigured); return }
  if (!instanceName.value.trim()) return
  pending.value = 'create'
  feedback.value = ''
  error.value = ''
  try {
    feedback.value = await invoke<string>('plugin:fre-horizon|horizon_create', {
      exePath: executable.value, name: instanceName.value,
    })
    launchName.value = instanceName.value.trim()
    await refreshInstances()
  } catch (err) {
    error.value = t(messages.error, { reason: failure(err) })
  } finally {
    pending.value = null
  }
}

async function playInstance() {
  if (!executable.value) { error.value = t(messages.notConfigured); return }
  if (!launchName.value.trim()) return
  pending.value = 'play'
  feedback.value = ''
  error.value = ''
  try {
    feedback.value = await invoke<string>('plugin:fre-horizon|horizon_play', {
      exePath: executable.value, name: launchName.value,
    })
  } catch (err) {
    error.value = t(messages.error, { reason: failure(err) })
  } finally {
    pending.value = null
  }
}

async function refreshInstances() {
  if (!executable.value) { error.value = t(messages.notConfigured); return }
  const originalPending = pending.value
  if (!originalPending) pending.value = 'list'
  try {
    listing.value = await invoke<string>('plugin:fre-horizon|horizon_instances', {
      exePath: executable.value,
    })
  } catch (err) {
    error.value = t(messages.error, { reason: failure(err) })
  } finally {
    if (!originalPending) pending.value = null
  }
}
async function showLogs() {
  if (!executable.value) { error.value = t(messages.notConfigured); return }
  pending.value = 'logs'
  error.value = ''
  try {
    logs.value = await invoke<string>('plugin:fre-horizon|horizon_logs', {
      exePath: executable.value,
    })
  } catch (err) {
    error.value = t(messages.error, { reason: failure(err) })
  } finally {
    pending.value = null
  }
}
async function manageHorizon(action: 'versions' | 'install-version' | 'set-version' | 'prepare') {
  if (!executable.value) { error.value = t(messages.notConfigured); return }
  if (pending.value) return
  pending.value = action
  error.value = ''
  feedback.value = ''
  try {
    if (action === 'versions') {
      versions.value = await invoke<string>('plugin:fre-horizon|horizon_versions', {
        exePath: executable.value,
        installedOnly: true,
      })
    } else if (action === 'install-version') {
      feedback.value = await invoke<string>('plugin:fre-horizon|horizon_install_version', {
        exePath: executable.value,
        version: releaseTag.value,
      })
      versions.value = await invoke<string>('plugin:fre-horizon|horizon_versions', {
        exePath: executable.value,
        installedOnly: true,
      })
    } else if (action === 'set-version') {
      if (!launchName.value.trim()) throw new Error('Select a Horizon instance first.')
      feedback.value = await invoke<string>('plugin:fre-horizon|horizon_set_version', {
        exePath: executable.value,
        name: launchName.value,
        version: releaseTag.value,
      })
    } else {
      feedback.value = await invoke<string>('plugin:fre-horizon|horizon_prepare', {
        exePath: executable.value,
      })
    }
  } catch (err) {
    error.value = t(messages.error, { reason: failure(err) })
  } finally {
    pending.value = null
  }
}
defineExpose({ show, hide })
</script>

<template>
  <div
    v-if="visible"
    class="fixed inset-0 z-[500] flex items-center justify-center bg-black/80 p-4 backdrop-blur-sm"
    role="dialog"
    aria-modal="true"
    :aria-label="t(messages.title)"
  >
    <div class="max-h-[90vh] w-full max-w-2xl overflow-y-auto rounded-2xl border border-solid border-surface-4 bg-bg-raised p-6 shadow-2xl">
      <div class="flex items-center justify-between gap-3">
        <div>
          <h2 class="m-0 text-2xl font-bold text-contrast">{{ t(messages.title) }}</h2>
          <p class="mt-2 mb-0 text-sm text-secondary">{{ t(messages.subtitle) }}</p>
        </div>
        <button type="button" class="cursor-pointer rounded-lg border border-solid border-surface-5 bg-button-bg p-2 text-contrast" :disabled="!!pending" @click="hide">
          {{ t(messages.closing) }}
        </button>
      </div>

      <p class="mt-4 rounded-lg border border-solid border-surface-5 p-3 text-sm text-secondary">{{ t(messages.requirement) }}</p>
      <p class="mt-2 text-sm text-secondary">{{ t(messages.separate) }}</p>

      <div class="mt-4 space-y-2">
        <label class="block text-sm font-semibold text-contrast">{{ t(messages.launcherLabel) }}</label>
        <div class="flex flex-wrap gap-2">
          <input :value="executable" readonly class="min-w-0 flex-1 rounded-lg bg-button-bg px-3 text-sm text-secondary" />
          <button type="button" class="cursor-pointer rounded-lg border border-solid border-surface-5 bg-button-bg px-3 py-2 text-sm text-contrast" :disabled="!!pending" @click="selectExe">
            {{ t(messages.chooseExe) }}
          </button>
        </div>
        <button type="button" class="cursor-pointer border-0 bg-transparent p-0 text-left text-sm text-brand hover:underline" @click="openUrl('https://playhorizon.dev/docs/install')">
          {{ t(messages.release) }}
        </button>
      </div>

      <div class="mt-5 rounded-xl border border-solid border-surface-4 p-4">
        <h3 class="m-0 text-base font-semibold text-contrast">{{ t(messages.versions) }}</h3>
        <p class="my-2 text-xs text-secondary">{{ t(messages.firstRun) }}</p>
        <label class="mt-3 block text-sm text-secondary">{{ t(messages.releaseTag) }}</label>
        <input v-model="releaseTag" class="mt-1 w-full rounded-lg bg-button-bg p-3 text-contrast" maxlength="80" />
        <div class="mt-3 flex flex-wrap gap-2">
          <button type="button" class="cursor-pointer rounded-lg border border-solid border-surface-5 bg-button-bg px-3 py-2 text-sm text-contrast disabled:opacity-40" :disabled="!!pending" @click="manageHorizon('versions')">
            {{ t(messages.showVersions) }}
          </button>
          <button type="button" class="cursor-pointer rounded-lg border-0 bg-brand px-3 py-2 text-sm font-semibold text-white disabled:opacity-40" :disabled="!!pending || !releaseTag.trim()" @click="manageHorizon('install-version')">
            {{ t(messages.installVersion) }}
          </button>
          <button type="button" class="cursor-pointer rounded-lg border border-solid border-surface-5 bg-button-bg px-3 py-2 text-sm text-contrast disabled:opacity-40" :disabled="!!pending" @click="manageHorizon('prepare')">
            {{ t(messages.prepare) }}
          </button>
        </div>
        <pre v-if="versions" class="mt-3 max-h-40 overflow-auto whitespace-pre-wrap rounded-lg bg-button-bg p-3 text-xs text-secondary">{{ versions }}</pre>
      </div>

      <div class="mt-5 flex flex-col gap-2">
        <label class="text-sm font-semibold text-contrast">{{ t(messages.name) }}</label>
        <input v-model="instanceName" class="w-full rounded-lg bg-button-bg p-3 text-contrast" maxlength="64" :placeholder="t(messages.namePlaceholder)" />
        <button type="button" class="cursor-pointer rounded-lg border-0 bg-brand px-4 py-2 font-semibold text-white disabled:opacity-40" :disabled="!!pending || !instanceName.trim()" @click="createInstance">
          {{ pending === 'create' ? t(messages.creating) : t(messages.create) }}
        </button>
      </div>
      <div class="mt-5 flex flex-col gap-2">
        <label class="text-sm font-semibold text-contrast">{{ t(messages.launchName) }}</label>
        <input v-model="launchName" class="w-full rounded-lg bg-button-bg p-3 text-contrast" maxlength="64" :placeholder="t(messages.namePlaceholder)" />
        <div class="flex flex-wrap gap-2">
          <button type="button" class="cursor-pointer rounded-lg border border-solid border-surface-5 bg-button-bg px-4 py-2 text-contrast disabled:opacity-40" :disabled="!!pending || !launchName.trim() || !releaseTag.trim()" @click="manageHorizon('set-version')">
            {{ t(messages.useVersion) }}
          </button>
          <button type="button" class="cursor-pointer rounded-lg border-0 bg-brand px-4 py-2 font-semibold text-white disabled:opacity-40" :disabled="!!pending || !launchName.trim()" @click="playInstance">
            {{ pending === 'play' ? t(messages.playing) : t(messages.play) }}
          </button>
          <button type="button" class="cursor-pointer rounded-lg border border-solid border-surface-5 bg-button-bg px-4 py-2 text-contrast disabled:opacity-40" :disabled="!!pending" @click="refreshInstances">
            {{ t(messages.list) }}
          </button>
          <button type="button" class="cursor-pointer rounded-lg border border-solid border-surface-5 bg-button-bg px-4 py-2 text-contrast disabled:opacity-40" :disabled="!!pending" @click="showLogs">
            {{ t(messages.logs) }}
          </button>
        </div>
      </div>

      <p v-if="error" role="alert" class="mt-4 rounded-lg border border-solid border-red-500/50 p-3 text-sm text-red-400">{{ error }}</p>
      <p v-if="feedback" role="status" class="mt-4 rounded-lg border border-solid border-surface-5 p-3 text-sm text-brand">{{ feedback }}</p>
      <pre v-if="listing" class="mt-4 max-h-48 overflow-auto whitespace-pre-wrap rounded-lg bg-button-bg p-3 text-xs text-secondary">{{ listing }}</pre>
      <p v-if="logs" class="mt-3 text-xs text-secondary">{{ t(messages.logPrivacy) }}</p>
      <pre v-if="logs" class="mt-2 max-h-52 overflow-auto whitespace-pre-wrap rounded-lg bg-button-bg p-3 text-xs text-secondary">{{ logs }}</pre>
      <p class="mt-4 text-xs text-secondary">{{ t(messages.about) }}</p>
    </div>
  </div>
</template>
