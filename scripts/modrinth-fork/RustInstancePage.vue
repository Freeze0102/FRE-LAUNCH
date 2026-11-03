<script setup lang="ts">
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import {
  BoxesIcon, ClockIcon, FolderOpenIcon, GlobeIcon, MoreVerticalIcon,
  PlayIcon, SettingsIcon, TerminalSquareIcon, TrashIcon,
} from '@modrinth/assets'
import {
  Avatar, Button, commonMessages, defineMessages, IconButton, NavTabs, NewModal,
  PageHeader, PageHeaderActions, PageHeaderMetadata, PageHeaderMetadataItem, TagIcon,
  TeleportOverflowMenu, useVIntl, injectNotificationManager,
} from '@modrinth/ui'
import { computed, inject, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import RustInstanceFiles from '@/components/ui/RustInstanceFiles.vue'
import ConfirmDeleteInstanceModal from '@/components/ui/modal/ConfirmDeleteInstanceModal.vue'
import type { GameInstance } from '@/helpers/types'
import { useRootBreadcrumb } from '@/shared/breadcrumbs'

type NativeProfile = { id: string; name: string; created: number; iconPath?: string | null }
type Runtime = { installed: boolean; prepared: boolean; version: string; reason: string | null }
type HorizonStatus = {
  instances: { id: string; name: string; version: string | null; selected: boolean }[]
  installedVersions: string[]
  preparedVersions: string[]
  selectedVersion: string | null
}
type Profile = { id: string; name: string; iconPath?: string | null; version: string; ready: boolean }

const route = useRoute()
const router = useRouter()
const { formatMessage: t } = useVIntl()
const { handleError } = injectNotificationManager()
const showHorizonWizard = inject<(id?: string) => void>('showHorizonWizard', () => {})
const m = defineMessages({
  content: { id: 'app.instance.tab.content', defaultMessage: 'Content' },
  files: { id: 'app.instance.tab.files', defaultMessage: 'Files' },
  worlds: { id: 'app.instance.tab.worlds', defaultMessage: 'Worlds' },
  logs: { id: 'app.instance.tab.logs', defaultMessage: 'Logs' },
  play: { id: 'fre-rust.grid.play', defaultMessage: 'Play' },
  setup: { id: 'fre-rust.grid.setup', defaultMessage: 'Set up game' },
  settings: { id: 'fre-rust.grid.settings', defaultMessage: 'Settings' },
  starting: { id: 'fre-rust.grid.starting', defaultMessage: 'Starting…' },
  openFolder: { id: 'fre-rust.grid.open-folder', defaultMessage: 'Open folder' },
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
  delete: { id: 'fre-rust.page.delete', defaultMessage: 'Delete instance' },
  install: { id: 'fre-rust.page.install', defaultMessage: 'Install game resources' },
  installing: { id: 'fre-rust.page.installing', defaultMessage: 'Installing…' },
  installFailed: { id: 'fre-rust.page.install-failed', defaultMessage: 'Automatic preparation needs an installed, legally owned Minecraft Java Edition and an active Microsoft account.' },
  chooseResources: { id: 'fre-rust.page.choose-resources', defaultMessage: 'Choose prepared resources folder' },
  resources: { id: 'fre-rust.page.resources', defaultMessage: 'Game resources' },
  preparing: { id: 'fre-rust.page.preparing', defaultMessage: 'Checking locally available resources…' },
})

const kind = computed<'native' | 'horizon'>(() => route.params.kind === 'horizon' ? 'horizon' : 'native')
const id = computed(() => String(route.params.id ?? ''))
const profile = ref<Profile | null>(null)
const horizonExecutable = ref('')
const runtime = ref<Runtime | null>(null)
const error = ref('')
const loading = ref(false)
const launching = ref(false)
const preparing = ref(false)
const renaming = ref(false)
const deleting = ref(false)
const editedName = ref('')
const setupError = ref('')
const settingsModal = ref<InstanceType<typeof NewModal>>()
const deleteModal = ref<InstanceType<typeof ConfirmDeleteInstanceModal>>()
const displayInstance = computed(() => profile.value ? ({
  id: kind.value + ':' + id.value,
  name: profile.value.name,
  icon_path: profile.value.iconPath ?? null,
  loader: 'rust',
  game_version: profile.value.version,
  modified: 0,
}) as unknown as GameInstance : null)
const logs = ref('')
const basePath = computed(() =>
  '/rust-instance/' + encodeURIComponent(kind.value) + '/' + encodeURIComponent(id.value),
)
const tab = computed(() => {
  const value = route.query.tab
  return value === 'files' || value === 'worlds' || value === 'logs' ? value : 'content'
})
const tabs = computed(() => [
  { label: t(m.content), href: basePath.value, icon: BoxesIcon },
  { label: t(m.files), href: basePath.value + '?tab=files', icon: FolderOpenIcon },
  { label: t(m.worlds), href: basePath.value + '?tab=worlds', icon: GlobeIcon },
  { label: t(m.logs), href: basePath.value + '?tab=logs', icon: TerminalSquareIcon },
])

const iconSrc = computed(() =>
  profile.value?.iconPath ? convertFileSrc(profile.value.iconPath) : undefined,
)
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
    if (kind.value === 'native') {
      const [profiles, status] = await Promise.all([
        invoke<NativeProfile[]>('plugin:fre-rust|rust_profiles'),
        invoke<Runtime>('plugin:fre-rust|rust_runtime'),
      ])
      if (current !== generation) return
      runtime.value = status
      const found = profiles.find(item => item.id === id.value)
      if (!found) throw new Error(t(m.unavailable))
      profile.value = {
        id: found.id, name: found.name, iconPath: found.iconPath,
        version: status.version || '26.3', ready: status.installed && status.prepared,
      }
    } else {
      const exePath = await invoke<string | null>('plugin:fre-horizon|horizon_detect_launcher')
      if (!exePath) throw new Error('Horizon executable not found')
      const status = await invoke<HorizonStatus>('plugin:fre-horizon|horizon_status', { exePath })
      if (current !== generation) return
      const found = status.instances.find(item => item.id === id.value)
      if (!found) throw new Error(t(m.unavailable))
      const version = found.version && found.version !== 'latest'
        ? found.version
        : status.selectedVersion || status.installedVersions[0] || '26.3'
      horizonExecutable.value = exePath
      profile.value = {
        id: found.id, name: found.name, version,
        ready: status.installedVersions.includes(version) && status.preparedVersions.includes(version),
      }
    }
    if (tab.value === 'logs') void loadLogs()
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
async function autoPrepare() {
  if (kind.value !== 'native' || preparing.value) return false
  preparing.value = true
  setupError.value = ''
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
async function play() {
  if (!profile.value || launching.value) return
  // Play remains the only primary action, just like Fabric. If native
  // resources are missing, first attempt an authorized local preparation,
  // and report the REAL blocker if none is available.
  if (!profile.value.ready && kind.value === 'native') {
    if (!await autoPrepare()) {
      settings(false)
      return
    }
  } else if (!profile.value.ready) {
    showHorizonWizard(id.value)
    return
  }
  launching.value = true
  try {
    if (kind.value === 'native') {
      await invoke('plugin:fre-rust|rust_launch', { profileId: id.value })
    } else {
      await invoke('plugin:fre-horizon|horizon_direct_play', {
        exePath: horizonExecutable.value, name: id.value,
      })
    }
  } catch (err) {
    handleError(err instanceof Error ? err : new Error(String(err)))
  } finally {
    launching.value = false
  }
}
async function openFolder(folder = 'root') {
  try {
    if (kind.value === 'native') {
      await invoke('plugin:fre-rust|rust_open_folder', { profileId: id.value, folder })
    } else {
      await invoke('plugin:fre-horizon|horizon_open_folder', {
        exePath: horizonExecutable.value, name: id.value, folder,
      })
    }
  } catch (err) {
    handleError(err instanceof Error ? err : new Error(String(err)))
  }
}
async function loadLogs() {
  logs.value = ''
  try {
    logs.value = kind.value === 'native'
      ? await invoke<string>('plugin:fre-rust|rust_logs', { profileId: id.value })
      : await invoke<string>('plugin:fre-horizon|horizon_logs', { exePath: horizonExecutable.value })
  } catch {
    logs.value = ''
  }
}
watch(() => [route.params.kind, route.params.id], () => { void refresh() }, { immediate: true })
watch(tab, next => { if (next === 'logs') void loadLogs() })
const onRefresh = () => { void refresh() }
onMounted(() => {
  window.addEventListener('fre-rust-library-refresh', onRefresh)
  window.addEventListener('fre-horizon-library-refresh', onRefresh)
})
onUnmounted(() => {
  generation++
  window.removeEventListener('fre-rust-library-refresh', onRefresh)
  window.removeEventListener('fre-horizon-library-refresh', onRefresh)
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
    <NewModal ref="settingsModal" :header="t(m.settings)" max-width="600px">
      <div class="flex flex-col gap-5">
        <div v-if="kind === 'native'" class="flex flex-col gap-2">
          <label class="text-sm font-semibold text-contrast" for="fre-rust-name">{{ t(m.rename) }}</label>
          <div class="flex flex-wrap gap-2">
            <input id="fre-rust-name" v-model="editedName" class="min-w-0 flex-1 rounded-xl border border-solid border-surface-4 bg-surface-3 px-4 py-3 text-contrast" maxlength="64" />
            <Button :disabled="renaming || !editedName.trim() || editedName.trim() === profile.name" @click="renameProfile">{{ t(m.saveName) }}</Button>
          </div>
        </div>
        <div class="flex flex-col gap-2">
          <h3 class="m-0 text-base font-semibold">{{ t(m.resources) }}</h3>
          <p v-if="!profile.ready" class="m-0 text-sm text-secondary">{{ statusText }}</p>
          <div class="flex flex-wrap gap-2">
            <Button v-if="kind === 'native' && !profile.ready" :disabled="preparing" @click="autoPrepare">
              {{ preparing ? t(m.installing) : t(m.install) }}
            </Button>
            <Button v-if="kind === 'native' && !profile.ready" type="outlined" :disabled="preparing" @click="chooseResources">
              {{ t(m.chooseResources) }}
            </Button>
            <Button type="outlined" @click="openFolder()">{{ t(m.openFolder) }}</Button>
          </div>
          <p v-if="setupError" role="alert" class="m-0 text-sm text-red-400">{{ setupError }}</p>
        </div>
        <Button v-if="kind === 'native'" type="colored" color="red" class="self-start" @click="deleteModal?.show()">
          <TrashIcon />{{ t(m.delete) }}
        </Button>
      </div>
    </NewModal>
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
            <PageHeaderMetadataItem :icon="ClockIcon">
              {{ t(m.neverPlayed) }}
            </PageHeaderMetadataItem>
          </PageHeaderMetadata>
        </template>
        <template #actions>
          <PageHeaderActions>
            <Button
              type="colored"
              color="brand"
              size="xl"
              native-type="button"
              :disabled="launching || preparing"
              @click="play"
            >
              <PlayIcon />
              {{ preparing ? t(m.installing) : launching ? t(m.starting) : t(m.play) }}
            </Button>
            <IconButton
              :label="t(m.settings)"
              v-tooltip="t(m.settings)"
              size="xl"
              native-type="button"
              @click="settings"
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
      <NavTabs :links="tabs" />
      <p v-if="setupError" role="alert" class="mt-4 text-sm text-red-400">{{ setupError }}</p>
      <p v-if="!profile.ready" class="mt-4 rounded-xl border border-solid border-surface-4 bg-surface-2 p-4 text-sm text-secondary">
        {{ statusText }}
      </p>
    </div>

    <div class="min-h-0 flex-1 overflow-y-auto p-6 pt-4">
      <div v-if="tab === 'content'" class="flex flex-col gap-3">
        <p class="m-0 text-sm text-secondary">{{ t(m.contentNote) }}</p>
        <RustInstanceFiles
          :key="kind + id + ':mods'"
          :kind="kind"
          :profile-id="id"
          :horizon-executable="horizonExecutable"
          initial-folder="mods"
        />
      </div>
      <RustInstanceFiles
        v-else-if="tab === 'files'"
        :key="kind + id + ':files'"
        :kind="kind"
        :profile-id="id"
        :horizon-executable="horizonExecutable"
        initial-folder="root"
      />
      <RustInstanceFiles
        v-else-if="tab === 'worlds'"
        :key="kind + id + ':worlds'"
        :kind="kind"
        :profile-id="id"
        :horizon-executable="horizonExecutable"
        initial-folder="saves"
      />
      <div v-else-if="tab === 'logs'" class="flex flex-col gap-4">
        <div class="flex justify-end">
          <Button @click="loadLogs">{{ t(m.retry) }}</Button>
        </div>
        <pre v-if="logs" class="m-0 max-h-full overflow-auto rounded-xl border border-solid border-surface-4 bg-surface-2 p-4 font-mono text-sm text-contrast">{{ logs }}</pre>
        <p v-else class="py-12 text-center text-secondary">{{ t(m.noLogs) }}</p>
      </div>
    </div>
  </div>
  <div v-else-if="loading" class="p-6 text-secondary">{{ t(commonMessages.loadingLabel) }}</div>
  <div v-else class="flex flex-col items-start gap-4 p-6">
    <p role="alert" class="text-red-400">{{ error || t(m.unavailable) }}</p>
    <Button @click="router.push('/')">Home</Button>
  </div>
</template>
