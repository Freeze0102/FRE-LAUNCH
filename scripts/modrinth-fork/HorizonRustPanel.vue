<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { openUrl } from '@tauri-apps/plugin-opener'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { computed, ref } from 'vue'

const { formatMessage: t } = useVIntl()
const messages = defineMessages({
  title: { id: "fre-launch.horizon.setup.title", defaultMessage: "Horizon — Minecraft in Rust" },
  intro: { id: "fre-launch.horizon.setup.intro", defaultMessage: "Create your own Rust profile. FRE LAUNCH installs the game and starts horizon-client.exe directly." },
  close: { id: "fre-launch.horizon.setup.close", defaultMessage: "Close" },
  launcherStep: { id: "fre-launch.horizon.setup.launcherStep", defaultMessage: "Rust game setup" },
  launcherHelp: { id: "fre-launch.horizon.setup.launcherHelp", defaultMessage: "FRE LAUNCH securely downloads the official setup components. No second launcher window is required to play." },
  done: { id: "fre-launch.horizon.setup.done", defaultMessage: "Ready" },
  download: { id: "fre-launch.horizon.setup.download", defaultMessage: "Install Rust game components" },
  downloading: { id: "fre-launch.horizon.setup.downloading", defaultMessage: "Downloading… {percent}%" },
  chooseExe: { id: "fre-launch.horizon.setup.chooseExe", defaultMessage: "I already have horizon-launcher.exe" },
  launcherReady: { id: "fre-launch.horizon.setup.launcherReady", defaultMessage: "Horizon launcher is ready. Next, install the game in step 2." },
  wrongExe: { id: "fre-launch.horizon.setup.wrongExe", defaultMessage: "Choose horizon-launcher.exe from the official Horizon archive." },
  gameStep: { id: "fre-launch.horizon.setup.gameStep", defaultMessage: "Install the game" },
  gameHelp: { id: "fre-launch.horizon.setup.gameHelp", defaultMessage: "Downloads the current recommended Horizon build. It is stored separately from your Java instances." },
  installGame: { id: "fre-launch.horizon.setup.installGame", defaultMessage: "Install Horizon" },
  updateGame: { id: "fre-launch.horizon.setup.updateGame", defaultMessage: "Check and install the latest build" },
  installing: { id: "fre-launch.horizon.setup.installing", defaultMessage: "Installing the game…" },
  gameReady: { id: "fre-launch.horizon.setup.gameReady", defaultMessage: "Horizon is installed. Now prepare the resources in step 3." },
  assetsStep: { id: "fre-launch.horizon.setup.assetsStep", defaultMessage: "Prepare Minecraft resources" },
  assetsHelp: { id: "fre-launch.horizon.setup.assetsHelp", defaultMessage: "You need Minecraft Java 26.3, started at least once. Press Prepare. If Horizon cannot find the files, select that 26.3 instance's folder." },
  prepare: { id: "fre-launch.horizon.setup.prepare", defaultMessage: "Prepare resources" },
  preparing: { id: "fre-launch.horizon.setup.preparing", defaultMessage: "Preparing resources…" },
  prepareDone: { id: "fre-launch.horizon.setup.prepareDone", defaultMessage: "Preparation completed. The resource status has been checked again." },
  minecraftFolder: { id: "fre-launch.horizon.setup.minecraftFolder", defaultMessage: "Choose the 26.3 instance folder" },
  autoFolder: { id: "fre-launch.horizon.setup.autoFolder", defaultMessage: "Find automatically" },
  folderAuto: { id: "fre-launch.horizon.setup.folderAuto", defaultMessage: "Horizon will first try to find your Minecraft folder automatically." },
  instance: { id: "fre-launch.horizon.setup.instance", defaultMessage: "Instance to play" },
  play: { id: "fre-launch.horizon.setup.play", defaultMessage: "Play Horizon" },
  playing: { id: "fre-launch.horizon.setup.playing", defaultMessage: "Starting Rust Minecraft…" },
  prepareFirst: { id: "fre-launch.horizon.setup.prepareFirst", defaultMessage: "First install Horizon and prepare Minecraft resources in steps 2–3." },
  accountHelp: { id: "fre-launch.horizon.setup.accountHelp", defaultMessage: "For account verification, some Horizon versions still require the original sign-in helper once. Normal Play starts the Rust client directly." },
  openLauncher: { id: "fre-launch.horizon.setup.openLauncher", defaultMessage: "Open Horizon / sign in" },
  launchRequested: { id: "fre-launch.horizon.setup.launchRequested", defaultMessage: "FRE LAUNCH started the native Rust Minecraft client. If it closes, check the logs." },
  wait: { id: "fre-launch.horizon.setup.wait", defaultMessage: "Horizon is working. Installation and preparation can take a few minutes." },
  advanced: { id: "fre-launch.horizon.setup.advanced", defaultMessage: "Other instances, versions and diagnostics" },
  defaultInstance: { id: "fre-launch.horizon.setup.defaultInstance", defaultMessage: "A Horizon instance already exists for your first launch. Create another only when you want separate worlds and settings." },
  name: { id: "fre-launch.horizon.setup.name", defaultMessage: "New instance name" },
  placeholder: { id: "fre-launch.horizon.setup.placeholder", defaultMessage: "My Rust Minecraft" },
  create: { id: "fre-launch.horizon.setup.create", defaultMessage: "Create a separate instance" },
  created: { id: "fre-launch.horizon.setup.created", defaultMessage: "Your Rust profile is now in the FRE LAUNCH library." },
  releaseTag: { id: "fre-launch.horizon.setup.releaseTag", defaultMessage: "Exact version tag or latest" },
  installVersion: { id: "fre-launch.horizon.setup.installVersion", defaultMessage: "Install this tag" },
  useVersion: { id: "fre-launch.horizon.setup.useVersion", defaultMessage: "Use for the selected instance" },
  versionChanged: { id: "fre-launch.horizon.setup.versionChanged", defaultMessage: "The instance version was changed." },
  showVersions: { id: "fre-launch.horizon.setup.showVersions", defaultMessage: "Show installed versions" },
  selectInstance: { id: "fre-launch.horizon.setup.selectInstance", defaultMessage: "Select a Horizon instance from the list." },
  refresh: { id: "fre-launch.horizon.setup.refresh", defaultMessage: "Refresh status and instances" },
  logs: { id: "fre-launch.horizon.setup.logs", defaultMessage: "Show logs" },
  release: { id: "fre-launch.horizon.setup.release", defaultMessage: "Official guide / manual download" },
  logPrivacy: { id: "fre-launch.horizon.setup.logPrivacy", defaultMessage: "Logs are read locally; nothing is uploaded." },
  notConfigured: { id: "fre-launch.horizon.setup.notConfigured", defaultMessage: "Download Horizon in step 1 or select an existing EXE first." },
  error: { id: "fre-launch.horizon.setup.error", defaultMessage: "Horizon error: {reason}" },
  about: { id: "fre-launch.horizon.setup.about", defaultMessage: "Horizon is an independent Rust game. FRE LAUNCH installs and starts the native client directly; its official helper is kept for verified updates, resources and optional account refresh." },
})

interface HorizonStatus {
  installedVersions: string[]
  preparedVersions: string[]
  selectedVersion: string | null
  instances: { id: string; name: string; version: string | null; selected: boolean }[]
}
type Action = 'bootstrap' | 'status' | 'create' | 'play' | 'logs' | 'versions' | 'install-version' | 'set-version' | 'prepare' | 'open'
const visible = ref(false)
const executable = ref('')
const minecraftPath = ref('')
const instanceName = ref('')
const launchName = ref('horizon')
const releaseTag = ref('latest')
const status = ref<HorizonStatus | null>(null)
const pending = ref<Action | null>(null)
const feedback = ref('')
const error = ref('')
const output = ref('')
const logs = ref('')
const downloadProgress = ref(0)
const activeInstance = computed(() => status.value?.instances.find(i => i.id === launchName.value))
const pinnedVersion = computed(() => {
  const version = activeInstance.value?.version
  return version && version !== 'latest' ? version : null
})
const activeVersion = computed(() => {
  return pinnedVersion.value || status.value?.selectedVersion || status.value?.installedVersions[0] || null
})
const gameInstalled = computed(() => !!activeVersion.value && !!status.value?.installedVersions.includes(activeVersion.value))
const prepared = computed(() => !!activeVersion.value && !!status.value?.preparedVersions.includes(activeVersion.value))
const canPlay = computed(() => !!activeInstance.value && prepared.value && !pending.value)

try {
  executable.value = localStorage.getItem('fre-launch-horizon-executable') || ''
  launchName.value = localStorage.getItem('fre-launch-horizon-instance') || 'horizon'
} catch { /* The picker still works without browser storage. */ }

function failure(err: unknown): string {
  if (err instanceof Error) return err.message
  if (err && typeof err === 'object' && 'message' in err) return String(err.message)
  return String(err)
}
function saveExecutable(path: string) {
  executable.value = path
  status.value = null
  try { localStorage.setItem('fre-launch-horizon-executable', path) } catch { /* optional */ }
}
async function refreshStatus() {
  const path = executable.value
  const next = await invoke<HorizonStatus>('plugin:fre-horizon|horizon_status', { exePath: path })
  if (path !== executable.value) return
  status.value = next
  window.dispatchEvent(new Event('fre-horizon-library-refresh'))
  if (!next.instances.some(i => i.id === launchName.value)) {
    launchName.value = next.instances.find(i => i.selected)?.id || next.instances[0]?.id || ''
  }
}
async function show() {
  visible.value = true
  if (pending.value) return
  error.value = ''
  feedback.value = ''
  pending.value = 'status'
  try {
    if (!executable.value) {
      const detected = await invoke<string | null>('plugin:fre-horizon|horizon_detect_launcher')
      if (detected) saveExecutable(detected)
    }
    if (executable.value) await refreshStatus()
  } catch (err) { error.value = t(messages.error, { reason: failure(err) }) }
  finally { pending.value = null }
}
function hide() { if (!pending.value) visible.value = false }

async function downloadLauncher() {
  if (pending.value) return
  pending.value = 'bootstrap'
  error.value = ''
  feedback.value = ''
  downloadProgress.value = 0
  let unlisten: (() => void) | undefined
  try {
    unlisten = await listen<number>('fre-horizon-bootstrap-progress', event => {
      if (typeof event.payload === 'number') downloadProgress.value = Math.min(100, Math.max(0, event.payload))
    })
    const path = await invoke<string>('plugin:fre-horizon|horizon_install_launcher')
    saveExecutable(path)
    await refreshStatus()
    feedback.value = t(messages.launcherReady)
  } catch (err) { error.value = t(messages.error, { reason: failure(err) }) }
  finally { unlisten?.(); pending.value = null }
}
async function selectExe() {
  if (pending.value) return
  try {
    const picked = await open({ multiple: false, directory: false, filters: [{ name: 'Horizon', extensions: ['exe'] }] })
    if (typeof picked !== 'string') return
    if (!/[\\/]horizon-launcher\.exe$/i.test(picked)) { error.value = t(messages.wrongExe); return }
    pending.value = 'status'
    saveExecutable(picked)
    error.value = ''
    await refreshStatus()
  } catch (err) { error.value = t(messages.error, { reason: failure(err) }) }
  finally { pending.value = null }
}
async function selectMinecraftFolder() {
  if (pending.value) return
  try {
    const picked = await open({ multiple: false, directory: true })
    if (typeof picked === 'string') minecraftPath.value = picked
  } catch (err) { error.value = t(messages.error, { reason: failure(err) }) }
}
async function refreshInstances() {
  if (pending.value || !executable.value) return
  pending.value = 'status'
  error.value = ''
  try { await refreshStatus() }
  catch (err) { error.value = t(messages.error, { reason: failure(err) }) }
  finally { pending.value = null }
}
async function createInstance() {
  if (pending.value) return
  if (!executable.value) { error.value = t(messages.notConfigured); return }
  const name = instanceName.value.trim()
  if (!name) return
  pending.value = 'create'
  error.value = ''
  try {
    output.value = await invoke<string>('plugin:fre-horizon|horizon_create', { exePath: executable.value, name })
    instanceName.value = ''
    await refreshStatus()
    launchName.value = status.value?.instances.find(i => i.name === name)?.id || launchName.value
    feedback.value = t(messages.created)
  } catch (err) { error.value = t(messages.error, { reason: failure(err) }) }
  finally { pending.value = null }
}
async function playInstance() {
  if (pending.value) return
  if (!executable.value) { error.value = t(messages.notConfigured); return }
  if (!launchName.value.trim()) return
  if (!prepared.value) { error.value = t(messages.prepareFirst); return }
  pending.value = 'play'
  error.value = ''
  feedback.value = ''
  try {
    output.value = await invoke<string>('plugin:fre-horizon|horizon_direct_play', { exePath: executable.value, name: launchName.value })
    try { localStorage.setItem('fre-launch-horizon-instance', launchName.value) } catch { /* optional */ }
    feedback.value = t(messages.launchRequested)
  } catch (err) { error.value = t(messages.error, { reason: failure(err) }) }
  finally { pending.value = null }
}
async function showLogs() {
  if (pending.value || !executable.value) return
  pending.value = 'logs'
  error.value = ''
  try { logs.value = await invoke<string>('plugin:fre-horizon|horizon_logs', { exePath: executable.value }) }
  catch (err) { error.value = t(messages.error, { reason: failure(err) }) }
  finally { pending.value = null }
}
async function openLauncher() {
  if (pending.value || !executable.value) return
  pending.value = 'open'
  error.value = ''
  try { await invoke('plugin:fre-horizon|horizon_open_launcher', { exePath: executable.value }) }
  catch (err) { error.value = t(messages.error, { reason: failure(err) }) }
  finally { pending.value = null }
}
async function manageHorizon(action: 'versions' | 'install-version' | 'set-version' | 'prepare', custom = false) {
  if (pending.value) return
  if (!executable.value) { error.value = t(messages.notConfigured); return }
  pending.value = action
  error.value = ''
  feedback.value = ''
  try {
    const exePath = executable.value
    if (action === 'versions') {
      output.value = await invoke<string>('plugin:fre-horizon|horizon_versions', { exePath, installedOnly: true })
    } else if (action === 'install-version') {
      output.value = await invoke<string>('plugin:fre-horizon|horizon_install_version', { exePath, version: custom ? releaseTag.value : pinnedVersion.value || 'latest' })
      feedback.value = t(messages.gameReady)
    } else if (action === 'set-version') {
      if (!launchName.value) throw new Error(t(messages.selectInstance))
      output.value = await invoke<string>('plugin:fre-horizon|horizon_set_version', { exePath, name: launchName.value, version: releaseTag.value })
      feedback.value = t(messages.versionChanged)
    } else {
      output.value = await invoke<string>('plugin:fre-horizon|horizon_prepare', {
        exePath, minecraftPath: minecraftPath.value || null,
        version: custom ? releaseTag.value : activeVersion.value || null,
      })
      feedback.value = t(messages.prepareDone)
    }
    await refreshStatus()
  } catch (err) { error.value = t(messages.error, { reason: failure(err) }) }
  finally { pending.value = null }
}
defineExpose({ show, hide })
</script>

<template>
  <div v-if="visible" class="fixed inset-0 z-[500] flex items-center justify-center bg-black/80 p-4 backdrop-blur-sm" role="dialog" aria-modal="true" :aria-label="t(messages.title)">
    <div class="max-h-[90vh] w-full max-w-2xl overflow-y-auto rounded-2xl border border-solid border-surface-4 bg-bg-raised p-6 shadow-2xl">
      <div class="flex items-start justify-between gap-4">
        <div>
          <h2 class="m-0 text-2xl font-bold text-contrast">{{ t(messages.title) }}</h2>
          <p class="mb-0 mt-2 text-sm text-secondary">{{ t(messages.intro) }}</p>
        </div>
        <button type="button" class="shrink-0 cursor-pointer rounded-lg border border-solid border-surface-5 bg-button-bg px-3 py-2 text-sm text-contrast disabled:opacity-40" :disabled="!!pending" @click="hide">{{ t(messages.close) }}</button>
      </div>
      <ol class="m-0 mt-5 list-none space-y-3 p-0">
        <li class="rounded-xl border border-solid border-surface-4 p-4">
          <div class="flex items-center justify-between gap-3">
            <h3 class="m-0 text-base font-semibold text-contrast">1. {{ t(messages.launcherStep) }}</h3>
            <span v-if="status" class="text-xs font-semibold text-brand">{{ t(messages.done) }}</span>
          </div>
          <p class="mb-3 mt-2 text-sm text-secondary">{{ t(messages.launcherHelp) }}</p>
          <button v-if="!status" type="button" class="cursor-pointer rounded-lg border-0 bg-brand px-4 py-2 font-semibold text-white disabled:opacity-40" :disabled="!!pending" @click="downloadLauncher">{{ pending === 'bootstrap' ? t(messages.downloading, { percent: Math.round(downloadProgress) }) : t(messages.download) }}</button>
          <progress v-if="pending === 'bootstrap'" class="mt-3 block w-full accent-brand" max="100" :value="downloadProgress" :aria-label="t(messages.download)" />
          <p v-if="status" class="mb-0 mt-1 break-all text-xs text-secondary">{{ executable }}</p>
          <button type="button" class="ml-0 mt-3 block cursor-pointer border-0 bg-transparent p-0 text-sm text-brand hover:underline disabled:opacity-40" :disabled="!!pending" @click="selectExe">{{ t(messages.chooseExe) }}</button>
        </li>
        <li class="rounded-xl border border-solid border-surface-4 p-4" :class="{ 'opacity-60': !status }">
          <div class="flex items-center justify-between gap-3">
            <h3 class="m-0 text-base font-semibold text-contrast">2. {{ t(messages.gameStep) }}</h3>
            <span v-if="gameInstalled" class="text-xs font-semibold text-brand">{{ t(messages.done) }}</span>
          </div>
          <p class="mb-3 mt-2 text-sm text-secondary">{{ t(messages.gameHelp) }}</p>
          <p v-if="activeVersion" class="my-2 break-all text-xs text-secondary">{{ activeVersion }}</p>
          <button type="button" class="cursor-pointer rounded-lg border-0 bg-brand px-4 py-2 text-sm font-semibold text-white disabled:opacity-40" :disabled="!!pending || !status" @click="manageHorizon('install-version')">{{ pending === 'install-version' ? t(messages.installing) : gameInstalled && !pinnedVersion ? t(messages.updateGame) : t(messages.installGame) }}</button>
        </li>
        <li class="rounded-xl border border-solid border-surface-4 p-4" :class="{ 'opacity-60': !gameInstalled }">
          <div class="flex items-center justify-between gap-3">
            <h3 class="m-0 text-base font-semibold text-contrast">3. {{ t(messages.assetsStep) }}</h3>
            <span v-if="prepared" class="text-xs font-semibold text-brand">{{ t(messages.done) }}</span>
          </div>
          <p class="mb-3 mt-2 text-sm text-secondary">{{ t(messages.assetsHelp) }}</p>
          <div class="flex flex-wrap items-center gap-3">
            <button type="button" class="cursor-pointer rounded-lg border-0 bg-brand px-4 py-2 text-sm font-semibold text-white disabled:opacity-40" :disabled="!!pending || !gameInstalled" @click="manageHorizon('prepare')">{{ pending === 'prepare' ? t(messages.preparing) : t(messages.prepare) }}</button>
            <button type="button" class="cursor-pointer border-0 bg-transparent p-0 text-sm text-brand hover:underline disabled:opacity-40" :disabled="!!pending" @click="selectMinecraftFolder">{{ t(messages.minecraftFolder) }}</button>
            <button v-if="minecraftPath" type="button" class="cursor-pointer border-0 bg-transparent p-0 text-xs text-secondary hover:underline" :disabled="!!pending" @click="minecraftPath = ''">{{ t(messages.autoFolder) }}</button>
          </div>
          <p class="mb-0 mt-3 break-all text-xs text-secondary">{{ minecraftPath || t(messages.folderAuto) }}</p>
        </li>
      </ol>
      <div class="mt-5 rounded-xl bg-button-bg p-4">
        <label for="fre-horizon-instance" class="block text-sm font-semibold text-contrast">{{ t(messages.instance) }}</label>
        <select id="fre-horizon-instance" v-model="launchName" class="my-3 w-full rounded-lg bg-bg-raised p-3 text-contrast" :disabled="!!pending || !status?.instances.length">
          <option v-for="instance in status?.instances || []" :key="instance.id" :value="instance.id">{{ instance.name }}</option>
          <option v-if="!status?.instances.length" value="horizon">Horizon</option>
        </select>
        <button type="button" class="w-full cursor-pointer rounded-lg border-0 bg-brand px-4 py-3 font-semibold text-white disabled:opacity-40" :disabled="!canPlay" @click="playInstance">{{ pending === 'play' ? t(messages.playing) : t(messages.play) }}</button>
      </div>
      <p v-if="pending && pending !== 'bootstrap'" role="status" class="mt-4 text-sm text-secondary">{{ t(messages.wait) }}</p>
      <p v-if="error" role="alert" class="mt-4 break-words rounded-lg border border-solid border-red-500/50 p-3 text-sm text-red-400">{{ error }}</p>
      <p v-if="feedback" role="status" class="mt-4 rounded-lg border border-solid border-surface-5 p-3 text-sm text-brand">{{ feedback }}</p>
      <div class="mt-5 rounded-xl border border-solid border-surface-4 p-4">
        <h3 class="m-0 text-base font-semibold text-contrast">{{ t(messages.name) }}</h3>
        <div class="mt-3 flex flex-wrap gap-2">
          <input v-model="instanceName" class="min-w-48 flex-1 rounded-lg bg-button-bg p-3 text-contrast" maxlength="64" :placeholder="t(messages.placeholder)" :disabled="!!pending" />
          <button type="button" class="cursor-pointer rounded-lg border-0 bg-brand px-4 py-2 font-semibold text-white disabled:opacity-40" :disabled="!!pending || !status || !instanceName.trim()" @click="createInstance">{{ t(messages.create) }}</button>
        </div>
        <p class="mb-0 mt-2 text-xs text-secondary">{{ t(messages.defaultInstance) }}</p>
      </div>
      <details class="mt-5 rounded-xl border border-solid border-surface-4 p-4">
        <summary class="cursor-pointer text-sm font-semibold text-contrast">{{ t(messages.advanced) }}</summary>
        <div class="mt-4 space-y-3">
          <label for="fre-horizon-version" class="block text-sm text-secondary">{{ t(messages.releaseTag) }}</label>
          <input id="fre-horizon-version" v-model="releaseTag" class="w-full rounded-lg bg-button-bg p-3 text-contrast" maxlength="80" :disabled="!!pending" />
          <div class="flex flex-wrap gap-2">
            <button type="button" class="cursor-pointer rounded-lg border border-solid border-surface-5 bg-button-bg px-3 py-2 text-sm text-contrast disabled:opacity-40" :disabled="!!pending || !status || !releaseTag.trim()" @click="manageHorizon('install-version', true)">{{ t(messages.installVersion) }}</button>
            <button type="button" class="cursor-pointer rounded-lg border border-solid border-surface-5 bg-button-bg px-3 py-2 text-sm text-contrast disabled:opacity-40" :disabled="!!pending || !activeInstance || !releaseTag.trim()" @click="manageHorizon('set-version', true)">{{ t(messages.useVersion) }}</button>
            <button type="button" class="cursor-pointer rounded-lg border border-solid border-surface-5 bg-button-bg px-3 py-2 text-sm text-contrast disabled:opacity-40" :disabled="!!pending || !status" @click="manageHorizon('versions')">{{ t(messages.showVersions) }}</button>
          </div>
          <p class="mt-3 text-xs text-secondary">{{ t(messages.accountHelp) }}</p>
          <button type="button" class="cursor-pointer border-0 bg-transparent p-0 text-sm text-brand hover:underline disabled:opacity-40" :disabled="!!pending || !executable" @click="openLauncher">{{ t(messages.openLauncher) }}</button>
          <div class="flex flex-wrap gap-3">
            <button type="button" class="cursor-pointer border-0 bg-transparent p-0 text-sm text-brand hover:underline disabled:opacity-40" :disabled="!!pending || !executable" @click="refreshInstances">{{ t(messages.refresh) }}</button>
            <button type="button" class="cursor-pointer border-0 bg-transparent p-0 text-sm text-brand hover:underline disabled:opacity-40" :disabled="!!pending || !executable" @click="showLogs">{{ t(messages.logs) }}</button>
            <button type="button" class="cursor-pointer border-0 bg-transparent p-0 text-sm text-brand hover:underline" @click="openUrl('https://playhorizon.dev/docs/install')">{{ t(messages.release) }}</button>
          </div>
          <pre v-if="output" class="max-h-48 overflow-auto whitespace-pre-wrap rounded-lg bg-button-bg p-3 text-xs text-secondary">{{ output }}</pre>
          <p v-if="logs" class="text-xs text-secondary">{{ t(messages.logPrivacy) }}</p>
          <pre v-if="logs" class="max-h-52 overflow-auto whitespace-pre-wrap rounded-lg bg-button-bg p-3 text-xs text-secondary">{{ logs }}</pre>
        </div>
      </details>
      <p class="mb-0 mt-4 text-xs text-secondary">{{ t(messages.about) }}</p>
    </div>
  </div>
</template>
