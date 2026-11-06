<script setup lang="ts">
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import {
  BoxesIcon, ChevronRightIcon, ClockIcon, FolderOpenIcon, GlobeIcon, MoreVerticalIcon,
  PlayIcon, SettingsIcon, StopCircleIcon, TerminalSquareIcon, TrashIcon,
} from '@modrinth/assets'
import {
  Avatar, Button, commonMessages, defineMessages, IconButton, NavTabs, TabbedModal,
  useRelativeTime, PageHeader, PageHeaderActions, PageHeaderMetadata, PageHeaderMetadataItem, TagIcon,
  TeleportOverflowMenu, useVIntl, injectNotificationManager,
} from '@modrinth/ui'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import RustInstanceFiles from '@/components/ui/RustInstanceFiles.vue'
import RustNativeMods from '@/components/ui/RustNativeMods.vue'
import RustWorlds from '@/components/ui/RustWorlds.vue'
import RustLogs from '@/components/ui/RustLogs.vue'
import ConfirmDeleteInstanceModal from '@/components/ui/modal/ConfirmDeleteInstanceModal.vue'
import type { GameInstance } from '@/helpers/types'
import { useRootBreadcrumb } from '@/shared/breadcrumbs'

type NativeProfile = { playtime?: number; lastPlayed?: number | null; id: string; name: string; created: number; modified?: number; iconPath?: string | null }
type Runtime = { installed: boolean; prepared: boolean; version: string; reason: string | null }
type Profile = { playtime?: number; lastPlayed?: number | null; id: string; name: string; modified?: number; iconPath?: string | null; version: string; ready: boolean }

const route = useRoute()
const router = useRouter()
const { formatMessage: t } = useVIntl()
const relativeTime = useRelativeTime()
const { handleError } = injectNotificationManager()
const m = defineMessages({
  content: { id: 'app.instance.tab.content', defaultMessage: 'Content' },
  files: { id: 'app.instance.tab.files', defaultMessage: 'Files' },
  worlds: { id: 'app.instance.tab.worlds', defaultMessage: 'Worlds' },
  logs: { id: 'app.instance.tab.logs', defaultMessage: 'Logs' },
  play: { id: 'fre-rust.grid.play', defaultMessage: 'Play' },
  stop: { id: 'fre-rust.page.stop', defaultMessage: 'Stop' },
  stopping: { id: 'fre-rust.page.stopping', defaultMessage: 'Stopping…' },
  setup: { id: 'fre-rust.grid.setup', defaultMessage: 'Set up game' },
  settings: { id: 'fre-rust.grid.settings', defaultMessage: 'Settings' },
  starting: { id: 'fre-rust.grid.starting', defaultMessage: 'Starting…' },
  openFolder: { id: 'fre-rust.grid.open-folder', defaultMessage: 'Open folder' },
  buildProgress: { id: 'fre-rust.catalog.build-progress', defaultMessage: 'Building your modded game:' },
  compiler: { id: 'fre-rust.catalog.compiler', defaultMessage: 'Preparing the private Rust compiler…' },
  dependencies: { id: 'fre-rust.catalog.dependencies', defaultMessage: 'Downloading pinned build dependencies…' },
  applying: { id: 'fre-rust.catalog.applying', defaultMessage: 'Combining the selected mods…' },
  compiling: { id: 'fre-rust.catalog.compiling', defaultMessage: 'Compiling the game. The first build can take a while…' },
  scanning: { id: 'fre-rust.catalog.scanning', defaultMessage: 'Checking the compiled game with Defender…' },
  playtime: { id: 'fre-rust.page.playtime', defaultMessage: '{minutes} min played' },
  lastPlayed: { id: 'fre-rust.page.last-played', defaultMessage: 'Last played: {time}' },
  neverPlayed: { id: 'instance.playtime.never-played', defaultMessage: 'Never played' },
  unavailable: { id: 'fre-rust.page.unavailable', defaultMessage: 'The Rust profile was not found.' },
  contentNote: {
    id: 'fre-rust.page.content-note',
    defaultMessage: 'Native Rust mods are managed in the game files. Java Fabric/Forge mods are not compatible.',
  },
  noLogs: { id: 'fre-rust.page.no-logs', defaultMessage: 'No local game logs yet.' },
  retry: { id: 'fre-rust.page.retry', defaultMessage: 'Refresh' },
  rename: { id: 'fre-rust.page.rename', defaultMessage: 'Name' },
  saveName: { id: 'fre-rust.page.save-name', defaultMessage: 'Save name' },
  changeIcon: { id: 'fre-rust.page.change-icon', defaultMessage: 'Choose icon' },
  resetIcon: { id: 'fre-rust.page.reset-icon', defaultMessage: 'Restore default icon' },
  repair: { id: 'fre-rust.page.repair', defaultMessage: 'Check and repair game resources' },
  delete: { id: 'fre-rust.page.delete', defaultMessage: 'Delete instance' },
  install: { id: 'fre-rust.page.install', defaultMessage: 'Install game resources' },
  installing: { id: 'fre-rust.page.installing', defaultMessage: 'Preparing Minecraft…' },
  installFailed: { id: 'fre-rust.page.install-failed', defaultMessage: 'Automatic preparation requires a Microsoft account that owns Minecraft Java Edition in FRE LAUNCH. The native resource preparer may require Java 25.' },
  chooseResources: { id: 'fre-rust.page.choose-resources', defaultMessage: 'Choose prepared resources folder' },
  firstRunHint: { id: 'fre-rust.page.first-run-hint', defaultMessage: 'FRE LAUNCH prepares Minecraft 26.3 using your own Microsoft Minecraft account, verified Mojang downloads and its bundled native game. No separate launcher is required.' },
  prepHelper: { id: 'fre-rust.page.prep-helper', defaultMessage: 'Checking native FRE LAUNCH setup…' },
  prepVersion: { id: 'fre-rust.page.prep-version', defaultMessage: 'Checking Rust game version…' },
  prepLocal: { id: 'fre-rust.page.prep-local', defaultMessage: 'Checking local Minecraft resources…' },
  prepDownload: { id: 'fre-rust.page.prep-download', defaultMessage: 'Downloading missing verified Minecraft files from Mojang…' },
  prepJava: { id: 'fre-rust.page.prep-java', defaultMessage: 'Checking Java 25 and installing it privately in FRE LAUNCH if needed…' },
  prepPrepare: { id: 'fre-rust.page.prep-prepare', defaultMessage: 'Building verified native Minecraft resources…' },
  prepCopy: { id: 'fre-rust.page.prep-copy', defaultMessage: 'Saving game resources in the FRE LAUNCH cache…' },
  prepReady: { id: 'fre-rust.page.prep-ready', defaultMessage: 'Minecraft resources ready.' },
  resources: { id: 'fre-rust.page.resources', defaultMessage: 'Game resources' },
  preparing: { id: 'fre-rust.page.preparing', defaultMessage: 'Checking locally available resources…' },
  generalTab: { id: 'fre-rust.page.tab.general', defaultMessage: 'General' },
  installationTab: { id: 'fre-rust.page.tab.installation', defaultMessage: 'Installation' },
  instanceSection: { id: 'fre-rust.page.section.instance', defaultMessage: 'Instance' },
  gameSection: { id: 'fre-rust.page.section.game', defaultMessage: 'Game' },
  deleteNotice: { id: 'fre-rust.page.delete-notice', defaultMessage: 'Permanently deletes this Rust instance and its worlds. Shared game resources are kept.' },
  notDownloaded: { id: 'fre-rust.page.not-downloaded', defaultMessage: 'If the resource cache is missing, FRE LAUNCH downloads verified Minecraft assets from Mojang after verifying account ownership.' },
})

const kind = computed<'native' | 'legacy'>(() => route.params.kind === 'legacy' ? 'legacy' : 'native')
const id = computed(() => String(route.params.id ?? ''))
const profile = ref<Profile | null>(null)
const runtime = ref<Runtime | null>(null)
const error = ref('')
const loading = ref(false)
const launching = ref(false)
const stopping = ref(false)
const running = ref(false)
const preparing = ref(false)
const renaming = ref(false)
const savingIcon = ref(false)
const deleting = ref(false)
const editedName = ref('')
const setupError = ref('')
const prepareStage = ref('')
const buildStage = ref('')
const settingsModal = ref<InstanceType<typeof TabbedModal>>()
const deleteModal = ref<InstanceType<typeof ConfirmDeleteInstanceModal>>()
const displayInstance = computed(() => profile.value ? ({
  id: kind.value + ':' + id.value,
  name: profile.value.name,
  icon_path: profile.value.iconPath ?? null,
  loader: 'rust',
  game_version: profile.value.version,
  modified: profile.value.modified ?? 0,
}) as unknown as GameInstance : null)
const basePath = computed(() =>
  '/rust-instance/' + encodeURIComponent(kind.value) + '/' + encodeURIComponent(id.value),
)
const tab = computed(() => {
  const value = route.query.tab
  return value === 'files' || value === 'worlds' || value === 'logs' ? value : 'content'
})
const tabs = computed(() => [
  { label: t(m.content), href: 'content', icon: BoxesIcon, subpages: [''] },
  { label: t(m.files), href: 'files', icon: FolderOpenIcon },
  { label: t(m.worlds), href: 'worlds', icon: GlobeIcon },
  { label: t(m.logs), href: 'logs', icon: TerminalSquareIcon },
])
const settingsTabs = [
  { name: m.generalTab, category: m.instanceSection, icon: SettingsIcon },
  { name: m.installationTab, category: m.gameSection, icon: FolderOpenIcon },
]

const iconSrc = computed(() =>
  profile.value?.iconPath ? convertFileSrc(profile.value.iconPath) + '?v=' + (profile.value.modified ?? 0) : undefined,
)
const prepareStageText = computed(() => {
  const labels: Record<string, keyof typeof m> = {
    helper: 'prepHelper', version: 'prepVersion', local: 'prepLocal',
    download: 'prepDownload', java: 'prepJava', prepare: 'prepPrepare', copy: 'prepCopy', ready: 'prepReady',
  }
  return prepareStage.value && labels[prepareStage.value]
    ? t(m[labels[prepareStage.value]])
    : ''
})
const statusText = computed(() =>
  kind.value === 'native'
    ? runtime.value?.reason || t(m.setup)
    : t(m.setup),
)
const options = computed(() => [
  { id: 'open-folder', label: t(m.openFolder), icon: FolderOpenIcon, action: () => { void openFolder() } },
  ...(kind.value === 'native'
    ? [{ id: 'delete', label: t(m.delete), icon: TrashIcon, action: () => deleteModal.value?.show() }]
    : []),
])
let generation = 0

async function refresh() {
  const current = ++generation
  loading.value = true
  error.value = ''
  try {
    if (kind.value !== 'native') throw new Error(t(m.unavailable))
    const [profiles, status, active] = await Promise.all([
      invoke<NativeProfile[]>('plugin:fre-rust|rust_profiles'),
      invoke<Runtime>('plugin:fre-rust|rust_runtime'),
      invoke<string[]>('plugin:fre-rust|rust_running').catch((): string[] => []),
    ])
    if (current !== generation) return
    runtime.value = status
    running.value = active.includes(id.value)
    const found = profiles.find(item => item.id === id.value)
    if (!found) throw new Error(t(m.unavailable))
    profile.value = {
      id: found.id, name: found.name, iconPath: found.iconPath,
      modified: found.modified, playtime: found.playtime, lastPlayed: found.lastPlayed,
      version: status.version || '26.3', ready: status.installed && status.prepared,
    }
  } catch (err) {
    if (current !== generation) return
    profile.value = null
    error.value = err instanceof Error ? err.message : String(err)
  } finally {
    if (current === generation) loading.value = false
  }
}
function settings(clearError = true) {
  editedName.value = profile.value?.name ?? ''
  if (clearError === true) setupError.value = ''
  settingsModal.value?.show()
}
async function renameProfile() {
  if (kind.value !== 'native' || renaming.value || !profile.value) return
  renaming.value = true
  try {
    await invoke('plugin:fre-rust|rust_rename_profile', {
      profileId: id.value, name: editedName.value.trim(),
    })
    await refresh()
    window.dispatchEvent(new Event('fre-rust-library-refresh'))
    settingsModal.value?.hide()
  } catch (err) {
    handleError(err instanceof Error ? err : new Error(String(err)))
  } finally { renaming.value = false }
}
async function deleteProfile() {
  if (kind.value !== 'native' || deleting.value) return
  deleting.value = true
  try {
    await invoke('plugin:fre-rust|rust_delete_profile', { profileId: id.value })
    window.dispatchEvent(new Event('fre-rust-library-refresh'))
    settingsModal.value?.hide()
    await router.push('/')
  } catch (err) {
    handleError(err instanceof Error ? err : new Error(String(err)))
  } finally { deleting.value = false }
}
async function changeIcon(reset = false) {
  if (savingIcon.value || !profile.value) return
  let source: string | null = null
  if (!reset) {
    const { open } = await import('@tauri-apps/plugin-dialog')
    const selected = await open({ multiple: false, filters: [{ name: 'PNG / JPEG / WebP', extensions: ['png', 'jpg', 'jpeg', 'webp'] }] })
    if (typeof selected !== 'string') return
    source = selected
  }
  savingIcon.value = true
  try {
    await invoke('plugin:fre-rust|rust_set_profile_icon', { profileId: id.value, source })
    await refresh()
    window.dispatchEvent(new Event('fre-rust-library-refresh'))
  } catch (err) {
    handleError(err instanceof Error ? err : new Error(String(err)))
  } finally { savingIcon.value = false }
}
async function autoPrepare() {
  if (kind.value !== 'native' || preparing.value) return false
  preparing.value = true
  setupError.value = ''
  prepareStage.value = ''
  try {
    await invoke<string>('plugin:fre-rust|rust_import_resources', { source: '' })
    await refresh()
    window.dispatchEvent(new Event('fre-rust-library-refresh'))
    return profile.value?.ready === true
  } catch (err) {
    setupError.value = err instanceof Error ? err.message : String(err)
    return false
  } finally { preparing.value = false }
}
async function chooseResources() {
  if (kind.value !== 'native' || preparing.value) return
  const { open } = await import('@tauri-apps/plugin-dialog')
  const selected = await open({ directory: true, multiple: false })
  if (typeof selected !== 'string') return
  preparing.value = true
  setupError.value = ''
  try {
    await invoke('plugin:fre-rust|rust_import_resources', { source: selected })
    await refresh()
    window.dispatchEvent(new Event('fre-rust-library-refresh'))
  } catch (err) {
    setupError.value = err instanceof Error ? err.message : String(err)
  } finally { preparing.value = false }
}
async function play(destination?: { world?: string; server?: string }) {
  if (!profile.value || launching.value || stopping.value || running.value) return
  // Play remains the only primary action, just like Fabric. If native
  // resources are missing, first attempt an authorized local preparation,
  // and report the REAL blocker if none is available.
  if (!profile.value.ready && kind.value === 'native') {
    if (!await autoPrepare()) {
      settings(false)
      return
    }
  }
  launching.value = true
  buildStage.value = ''
  try {
    await invoke('plugin:fre-rust|rust_launch', { profileId: id.value, world: destination?.world ?? null, server: destination?.server ?? null })
    await refreshRunning()
  } catch (err) {
    handleError(err instanceof Error ? err : new Error(String(err)))
  } finally {
    launching.value = false
  }
}
async function refreshRunning() {
  try {
    const active = await invoke<string[]>('plugin:fre-rust|rust_running')
    running.value = active.includes(id.value)
    const profiles = await invoke<NativeProfile[]>('plugin:fre-rust|rust_profiles')
    const current = profiles.find(item => item.id === id.value)
    if (profile.value && current) { profile.value.playtime = current.playtime; profile.value.lastPlayed = current.lastPlayed }
  } catch { /* The profile's other tabs remain usable. */ }
}
async function stop() {
  if (!running.value || stopping.value || launching.value) return
  stopping.value = true
  try {
    await invoke('plugin:fre-rust|rust_stop', { profileId: id.value })
    await refreshRunning()
  } catch (err) {
    handleError(err instanceof Error ? err : new Error(String(err)))
  } finally {
    stopping.value = false
  }
}
async function openFolder(folder = 'root') {
  try {
    await invoke('plugin:fre-rust|rust_open_folder', { profileId: id.value, folder })
  } catch (err) {
    handleError(err instanceof Error ? err : new Error(String(err)))
  }
}
watch(() => [route.params.kind, route.params.id], () => { void refresh() }, { immediate: true })
const onRefresh = () => { void refresh() }
let unlistenPrepareStage: (() => void) | null = null
let unlistenProcess: (() => void) | null = null
let unlistenBuild: (() => void) | null = null
let stageListenerDisposed = false
onMounted(() => {
  stageListenerDisposed = false
  void listen<{ profileId: string; stage: string }>('fre-rust-mod-build', event => {
    if (event.payload.profileId === id.value) buildStage.value = event.payload.stage
  }).then(unlisten => { if (stageListenerDisposed) unlisten(); else unlistenBuild = unlisten }).catch(() => {})
  void listen<string>('fre-rust-prepare-stage', event => {
    if (kind.value === 'native' && preparing.value) prepareStage.value = event.payload
  }).then(unlisten => {
    if (stageListenerDisposed) unlisten()
    else unlistenPrepareStage = unlisten
  }).catch(() => { /* Stage indicators are optional, not proof of success. */ })
  void listen('fre-rust-process-changed', () => { void refresh() })
    .then(unlisten => {
      if (stageListenerDisposed) unlisten()
      else unlistenProcess = unlisten
    }).catch(() => {})
  window.addEventListener('fre-rust-library-refresh', onRefresh)
  window.addEventListener('focus', refreshRunning)
})
onUnmounted(() => {
  generation++
  stageListenerDisposed = true
  unlistenPrepareStage?.()
  unlistenPrepareStage = null
  unlistenBuild?.()
  unlistenProcess?.()
  unlistenProcess = null
  window.removeEventListener('fre-rust-library-refresh', onRefresh)
  window.removeEventListener('focus', refreshRunning)
})
useRootBreadcrumb({
  slot: 'instance',
  id: () => 'rust-instance:' + kind.value + ':' + id.value,
  label: () => profile.value?.name ?? 'Rust',
  visual: () => ({ type: 'image', src: iconSrc.value, alt: profile.value?.name ?? 'Rust', tintBy: id.value }),
  to: () => basePath.value,
})
</script>

<template>
  <div v-if="profile" class="flex h-full min-h-0 flex-col">
    <ConfirmDeleteInstanceModal
      v-if="kind === 'native' && displayInstance"
      ref="deleteModal"
      :instances="[displayInstance]"
      @delete="deleteProfile"
    />
    <TabbedModal
      ref="settingsModal"
      :tabs="settingsTabs"
      width="928px"
      max-width="928px"
    >
      <template #title>
        <span class="flex items-center gap-2 text-lg font-semibold text-primary">
          <Avatar :src="iconSrc" size="24px" :tint-by="id" pad-transparent-corners />
          {{ profile.name }}
          <ChevronRightIcon />
          <span class="font-extrabold text-contrast">{{ t(m.settings) }}</span>
        </span>
      </template>
      <template #content="{ index }">
        <div v-if="index === 0" class="flex flex-col gap-6">
          <div v-if="kind === 'native'" class="flex flex-col gap-2">
            <label class="text-lg font-semibold text-contrast" for="fre-rust-name">{{ t(m.rename) }}</label>
            <div class="flex flex-wrap gap-2">
              <input
                id="fre-rust-name"
                v-model="editedName"
                class="min-w-0 flex-1 rounded-xl border border-solid border-surface-4 bg-surface-3 px-4 py-3 text-contrast"
                maxlength="64"
              />
              <Button :disabled="renaming || !editedName.trim() || editedName.trim() === profile.name" @click="renameProfile">
                {{ t(m.saveName) }}
              </Button>
            </div>
          </div>
          <div class="flex flex-wrap items-center gap-2">
            <Avatar :src="iconSrc" size="64px" :tint-by="id" pad-transparent-corners />
            <Button :disabled="savingIcon" @click="changeIcon()">{{ t(m.changeIcon) }}</Button>
            <Button type="outlined" :disabled="savingIcon" @click="changeIcon(true)">{{ t(m.resetIcon) }}</Button>
          </div>
          <div>
            <h3 class="m-0 text-lg font-semibold text-contrast">{{ t(m.openFolder) }}</h3>
            <Button type="outlined" class="mt-2" @click="openFolder()"><FolderOpenIcon />{{ t(m.openFolder) }}</Button>
          </div>
          <div v-if="kind === 'native'" class="flex flex-col items-start gap-2">
            <h3 class="m-0 text-lg font-semibold text-contrast">{{ t(m.delete) }}</h3>
            <Button type="colored" color="red" @click="deleteModal?.show()">
              <TrashIcon />{{ t(m.delete) }}
            </Button>
            <p class="m-0 text-sm text-secondary">{{ t(m.deleteNotice) }}</p>
          </div>
        </div>
        <div v-else class="flex flex-col gap-4">
          <h3 class="m-0 text-lg font-semibold text-contrast">{{ t(m.resources) }}</h3>
          <p class="m-0 text-sm text-secondary">{{ profile.ready ? t(m.play) : statusText }}</p>
          <div class="flex flex-wrap gap-2">
            <Button v-if="kind === 'native'" :disabled="preparing" @click="autoPrepare">
              {{ preparing ? t(m.installing) : profile.ready ? t(m.repair) : t(m.install) }}
            </Button>

            <Button v-if="kind === 'native' && !profile.ready" type="outlined" :disabled="preparing" @click="chooseResources">
              {{ t(m.chooseResources) }}
            </Button>
            <Button type="outlined" @click="openFolder()"><FolderOpenIcon />{{ t(m.openFolder) }}</Button>
          </div>
          <p v-if="preparing && prepareStageText" role="status" class="m-0 text-sm text-brand">{{ prepareStageText }}</p>
          <p v-if="kind === 'native' && !profile.ready" class="m-0 text-sm text-secondary">{{ t(m.firstRunHint) }}</p>
          <p v-if="!profile.ready" class="m-0 text-sm text-secondary">{{ t(m.notDownloaded) }}</p>
          <p v-if="launching && buildStage" role="status" class="mt-4 rounded-xl bg-surface-3 p-4 text-brand">{{ t(m.buildProgress) }} {{ t(m[buildStage as keyof typeof m] ?? m.compiling) }}</p>
      <p v-if="setupError" role="alert" class="m-0 text-sm text-red-400">{{ setupError }}</p>
        </div>
      </template>
    </TabbedModal>
    <div class="shrink-0 p-6 pr-2 pb-4">
      <PageHeader :title="profile.name">
        <template #leading>
          <Avatar
            :src="iconSrc"
            :alt="profile.name"
            :tint-by="kind + ':' + profile.id"
            size="64px"
            pad-transparent-corners
          />
        </template>
        <template #metadata>
          <PageHeaderMetadata>
            <PageHeaderMetadataItem :icon="TagIcon">
              Rust {{ profile.version }}
            </PageHeaderMetadataItem>
            <PageHeaderMetadataItem v-if="profile.playtime" :icon="ClockIcon">{{ t(m.playtime, { minutes: Math.floor(profile.playtime / 60) }) }}</PageHeaderMetadataItem>
            <PageHeaderMetadataItem :icon="ClockIcon">
              {{ profile.lastPlayed ? t(m.lastPlayed, { time: relativeTime(new Date(profile.lastPlayed)) }) : t(m.neverPlayed) }}
            </PageHeaderMetadataItem>
          </PageHeaderMetadata>
        </template>
        <template #actions>
          <PageHeaderActions>
            <Button
              type="colored"
              :color="running ? 'red' : 'brand'"
              size="xl"
              native-type="button"
              :disabled="launching || stopping || preparing"
              @click="running ? stop() : play(undefined)"
            >
              <StopCircleIcon v-if="running" />
              <PlayIcon v-else />
              {{ preparing ? t(m.installing) : stopping ? t(m.stopping) : launching ? t(m.starting) : running ? t(m.stop) : t(m.play) }}
            </Button>
            <IconButton
              :label="t(m.settings)"
              v-tooltip="t(m.settings)"
              size="xl"
              native-type="button"
              @click="settings()"
            >
              <SettingsIcon />
            </IconButton>
            <TeleportOverflowMenu
              type="quiet"
              size="xl"
              :label="t(m.openFolder)"
              :tooltip="t(m.openFolder)"
              :options="options"
            >
              <MoreVerticalIcon />
            </TeleportOverflowMenu>
          </PageHeaderActions>
        </template>
      </PageHeader>
    </div>

    <div class="shrink-0 px-6">
      <NavTabs :links="tabs" mode="local" :active-index="['content', 'files', 'worlds', 'logs'].indexOf(tab)" @tab-click="index => router.push({ path: route.path, query: { tab: ['content', 'files', 'worlds', 'logs'][index] } })" />
      <p v-if="launching && buildStage" role="status" class="mt-4 rounded-xl bg-surface-3 p-4 text-brand">{{ t(m.buildProgress) }} {{ t(m[buildStage as keyof typeof m] ?? m.compiling) }}</p>
      <p v-if="setupError" role="alert" class="mt-4 text-sm text-red-400">{{ setupError }}</p>
      <p v-if="!profile.ready" class="mt-4 rounded-xl border border-solid border-surface-4 bg-surface-2 p-4 text-sm text-secondary">
        {{ statusText }}
      </p>
    </div>

    <Transition name="fre-rust-tab" mode="out-in">
    <div :key="tab" class="min-h-0 flex-1 overflow-y-auto p-6 pt-4">
      <div v-if="tab === 'content'" class="flex flex-col gap-3">
        <RustNativeMods v-if="kind === 'native'" :profile-id="id" :running="running || launching" :catalog-open="route.query.view === 'catalog'" @browse="router.push({ path: route.path, query: { tab: 'content', view: 'catalog' } })" @back="router.push({ path: route.path, query: { tab: 'content' } })" />

      </div>
      <RustInstanceFiles
        v-else-if="tab === 'files'"
        :key="kind + id + ':files'"
        :profile-id="id"
        initial-folder="root"
      />
      <RustWorlds v-else-if="tab === 'worlds'" :profile-id="id" :running="running" :starting="launching" @play="play" @stop="stop" />
      <RustLogs v-else-if="tab === 'logs'" :profile-id="id" :running="running" class="h-full min-h-[28rem]" />
    </div>
    </Transition>
  </div>
  <div v-else-if="loading" class="p-6 text-secondary">{{ t(commonMessages.loadingLabel) }}</div>
  <div v-else class="flex flex-col items-start gap-4 p-6">
    <p role="alert" class="text-red-400">{{ error || t(m.unavailable) }}</p>
    <Button @click="router.push('/')">Home</Button>
  </div>
</template>


<style scoped>
.fre-rust-tab-enter-active,
.fre-rust-tab-leave-active {
  transition: opacity 180ms ease, transform 180ms ease;
}
.fre-rust-tab-enter-from {
  opacity: 0;
  transform: translateY(8px);
}
.fre-rust-tab-leave-to {
  opacity: 0;
  transform: translateY(-6px);
}
@media (prefers-reduced-motion: reduce) {
  .fre-rust-tab-enter-active,
  .fre-rust-tab-leave-active {
    transition: none;
  }
}
</style>
