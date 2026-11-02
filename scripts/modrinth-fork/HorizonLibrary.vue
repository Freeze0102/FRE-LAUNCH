<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { computed, inject, onMounted, onUnmounted, ref } from 'vue'

type RustInstance = { id: string; name: string; version: string | null; selected: boolean }
type HorizonGameFile = { name: string; bytes: number; isDirectory: boolean }
type FileCategory = 'mods' | 'resourcepacks' | 'shaderpacks' | 'saves' | 'logs' | 'root'
const fileCategories: { id: FileCategory; label: string }[] = [
  { id: 'mods', label: 'Моди' }, { id: 'resourcepacks', label: 'Ресурс-паки' },
  { id: 'shaderpacks', label: 'Шейдери' }, { id: 'saves', label: 'Світи' },
  { id: 'logs', label: 'Журнали' }, { id: 'root', label: 'Усі файли' },
]
type RustStatus = {
  instances: RustInstance[]
  installedVersions: string[]
  preparedVersions: string[]
  selectedVersion: string | null
}
const { formatMessage: t } = useVIntl()
const messages = defineMessages({
  title: { id: 'fre-launch.rust.library.title', defaultMessage: 'Horizon — existing profiles' },
  subtitle: { id: 'fre-launch.rust.library.subtitle', defaultMessage: 'Horizon profiles are separate game installations with their own worlds, options and game files.' },
  version: { id: 'fre-launch.rust.library.version', defaultMessage: 'Version {version}' },
  play: { id: 'fre-launch.rust.library.play', defaultMessage: 'Play' },
  playing: { id: 'fre-launch.rust.library.playing', defaultMessage: 'Starting...' },
  manage: { id: 'fre-launch.rust.library.manage', defaultMessage: 'Manage' },
  files: { id: 'fre-launch.rust.library.files', defaultMessage: 'Files' },
  add: { id: 'fre-launch.rust.library.add', defaultMessage: 'New Rust profile' },
  needsPrepare: { id: 'fre-launch.rust.library.needs-prepare', defaultMessage: 'Installation or resources required' },
  empty: { id: 'fre-launch.rust.library.empty', defaultMessage: 'No Horizon profiles found. Open the manager to add one.' },
  ready: { id: 'fre-launch.rust.library.ready', defaultMessage: 'Ready to play' },
  error: { id: 'fre-launch.rust.library.error', defaultMessage: 'Horizon: {reason}' },
})
const openHorizonWizard = inject<(instanceId?: string) => void>('showHorizonWizard', () => {})
const status = ref<RustStatus | null>(null)
const executable = ref('')
const error = ref('')
const launching = ref<string | null>(null)
const fileProfile = ref<RustInstance | null>(null)
const fileCategory = ref<FileCategory>('mods')
const fileEntries = ref<HorizonGameFile[]>([])
const fileFilter = ref('')
const filesLoading = ref(false)
let fileSequence = 0
const filteredFiles = computed(() => fileEntries.value.filter(entry =>
  entry.name.toLowerCase().includes(fileFilter.value.trim().toLowerCase()),
))
let generation = 0
const profiles = computed(() => status.value?.instances.filter(i =>
  i.id === 'horizon' || /^[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}$/.test(i.id),
) ?? [])
function activeVersion(profile: RustInstance): string | null {
  return profile.version && profile.version !== 'latest'
    ? profile.version
    : status.value?.selectedVersion || status.value?.installedVersions[0] || null
}
function playable(profile: RustInstance): boolean {
  const version = activeVersion(profile)
  return !!version
    && !!status.value?.installedVersions.includes(version)
    && !!status.value?.preparedVersions.includes(version)
}
function reason(value: unknown): string {
  return value instanceof Error ? value.message : String(value)
}
async function refresh() {
  const sequence = ++generation
  try {
    const detected = await invoke<string | null>('plugin:fre-horizon|horizon_detect_launcher')
    let saved = ''
    try { saved = localStorage.getItem('fre-launch-horizon-executable') || '' } catch { /* optional */ }
    const path = detected || saved
    if (!path) {
      if (sequence === generation) { status.value = null; executable.value = ''; error.value = '' }
      return
    }
    const next = await invoke<RustStatus>('plugin:fre-horizon|horizon_status', { exePath: path })
    if (sequence !== generation) return
    executable.value = path
    status.value = next
    error.value = ''
  } catch (e) {
    if (sequence === generation) { error.value = reason(e); status.value = null }
  }
}
async function play(profile: RustInstance) {
  if (launching.value || !playable(profile)) return
  launching.value = profile.id
  error.value = ''
  try {
    await invoke<string>('plugin:fre-horizon|horizon_direct_play', {
      exePath: executable.value,
      name: profile.id,
    })
  } catch (e) {
    error.value = t(messages.error, { reason: reason(e) })
  } finally { launching.value = null }
}
async function refreshFiles(category: FileCategory = fileCategory.value) {
  const profile = fileProfile.value
  if (!profile) return
  const current = ++fileSequence
  fileCategory.value = category
  filesLoading.value = true
  fileEntries.value = []
  error.value = ''
  try {
    const files = await invoke<HorizonGameFile[]>('plugin:fre-horizon|horizon_list_files', {
      exePath: executable.value,
      name: profile.id,
      folder: category,
    })
    if (current === fileSequence) fileEntries.value = files
  } catch (e) {
    if (current === fileSequence) error.value = t(messages.error, { reason: reason(e) })
  } finally {
    if (current === fileSequence) filesLoading.value = false
  }
}
async function openFiles(profile: RustInstance) {
  fileProfile.value = profile
  fileFilter.value = ''
  await refreshFiles('mods')
}
async function openExplorer() {
  const profile = fileProfile.value
  if (!profile) return
  error.value = ''
  try {
    await invoke<void>('plugin:fre-horizon|horizon_open_folder', {
      exePath: executable.value,
      name: profile.id,
      folder: fileCategory.value,
    })
  } catch (e) {
    error.value = t(messages.error, { reason: reason(e) })
  }
}
function closeFiles() {
  fileSequence++
  fileProfile.value = null
  fileEntries.value = []
  filesLoading.value = false
}
function formatSize(bytes: number) {
  return bytes >= 1048576 ? (bytes / 1048576).toFixed(1) + ' MB' :
    bytes >= 1024 ? (bytes / 1024).toFixed(1) + ' KB' : bytes + ' B'
}
function update() { void refresh() }
onMounted(() => {
  window.addEventListener('fre-horizon-library-refresh', update)
  void refresh()
})
onUnmounted(() => {
  fileSequence++
  generation++
  window.removeEventListener('fre-horizon-library-refresh', update)
})
</script>

<template>
  <section v-if="profiles.length || executable || error" class="flex flex-col gap-3">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div>
        <h2 class="m-0 text-2xl font-semibold text-contrast">{{ t(messages.title) }}</h2>
        <p class="mb-0 mt-1 text-sm text-secondary">{{ t(messages.subtitle) }}</p>
      </div>
      <button
        type="button"
        class="cursor-pointer rounded-lg border border-solid border-surface-4 bg-button-bg px-4 py-2 text-sm font-semibold text-contrast"
        @click="openHorizonWizard()"
      >{{ t(messages.add) }}</button>
    </div>
    <p v-if="!profiles.length && !error" class="m-0 rounded-xl border border-solid border-surface-4 bg-bg-raised p-4 text-sm text-secondary">{{ t(messages.empty) }}</p>
    <div v-if="profiles.length" class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
      <article
        v-for="profile in profiles"
        :key="profile.id"
        class="flex min-h-44 flex-col justify-between gap-3 rounded-xl border border-solid border-surface-4 bg-bg-raised p-4 transition-colors hover:border-surface-5"
      >
        <div class="flex gap-3">
          <div class="flex size-12 shrink-0 items-center justify-center rounded-lg bg-button-bg text-2xl font-bold text-brand">R</div>
          <div class="min-w-0">
            <h3 class="m-0 truncate text-base font-semibold text-contrast">{{ profile.name }}</h3>
            <p class="m-0 mt-1 text-xs text-secondary">Horizon · Rust · {{ activeVersion(profile) || '—' }}</p>
            <p class="m-0 mt-2 text-xs" :class="playable(profile) ? 'text-brand' : 'text-secondary'">
              {{ playable(profile) ? t(messages.ready) : t(messages.needsPrepare) }}
            </p>
          </div>
        </div>
        <div class="flex gap-2">
          <button
            type="button"
            class="flex-1 cursor-pointer rounded-lg border-0 bg-brand px-3 py-2 text-sm font-semibold text-white disabled:cursor-not-allowed disabled:opacity-40"
            :disabled="!playable(profile) || !!launching"
            @click="play(profile)"
          >{{ launching === profile.id ? t(messages.playing) : t(messages.play) }}</button>
          <button type="button" class="cursor-pointer rounded-lg border border-solid border-surface-5 bg-button-bg px-3 py-2 text-sm text-contrast" @click="openFiles(profile)">{{ t(messages.files) }}</button>
          <button type="button" class="cursor-pointer rounded-lg border border-solid border-surface-5 bg-button-bg px-3 py-2 text-sm text-contrast" @click="openHorizonWizard(profile.id)">
            {{ t(messages.manage) }}
          </button>
        </div>
      </article>
    </div>
    <div v-if="fileProfile" class="rounded-xl border border-solid border-surface-4 bg-bg-raised p-4" aria-label="Файли Horizon-збірки">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h3 class="m-0 font-semibold text-contrast">Файли · {{ fileProfile.name }}</h3>
          <p class="m-0 mt-1 text-xs text-secondary">Окремі файли Horizon, без Java-модів</p>
        </div>
        <div class="flex gap-2">
          <button type="button" class="cursor-pointer rounded-lg bg-button-bg px-3 py-2 text-sm text-contrast" :disabled="filesLoading" @click="refreshFiles()">Оновити</button>
          <button type="button" class="cursor-pointer rounded-lg bg-button-bg px-3 py-2 text-sm text-contrast" @click="openExplorer">Папка Windows</button>
          <button type="button" class="cursor-pointer rounded-lg bg-button-bg px-3 py-2 text-sm text-contrast" @click="closeFiles">Закрити</button>
        </div>
      </div>
      <div class="mt-4 flex flex-wrap gap-2">
        <button v-for="category in fileCategories" :key="category.id" type="button"
          class="cursor-pointer rounded-lg px-3 py-2 text-sm"
          :class="fileCategory === category.id ? 'bg-brand text-white' : 'bg-button-bg text-secondary'"
          :aria-pressed="fileCategory === category.id"
          @click="refreshFiles(category.id)">{{ category.label }}</button>
      </div>
      <input v-model="fileFilter" aria-label="Пошук файлів Horizon" placeholder="Пошук встановлених файлів..."
        class="mt-4 w-full rounded-lg bg-button-bg p-3 text-contrast" />
      <p v-if="filesLoading" class="mt-3 text-sm text-secondary">Читаємо файли…</p>
      <p v-else-if="!filteredFiles.length" class="mt-3 text-sm text-secondary">Файлів не знайдено.</p>
      <div v-else class="mt-3 max-h-64 overflow-auto rounded-lg border border-solid border-surface-4">
        <div v-for="file in filteredFiles" :key="file.name"
          class="flex items-center justify-between gap-3 border-b border-solid border-surface-4 px-3 py-2 text-sm">
          <span class="min-w-0 truncate text-contrast" :title="file.name">{{ file.isDirectory ? '📁' : '📄' }} {{ file.name }}</span>
          <span class="shrink-0 text-xs text-secondary">{{ file.isDirectory ? 'Папка' : formatSize(file.bytes) }}</span>
        </div>
      </div>
      <p class="mt-3 text-xs text-secondary">Java JAR-моди несумісні з нативним Horizon. Встановлюй лише формати, які підтримує цей клієнт.</p>
    </div>
    <p v-if="error" role="alert" class="text-sm text-red-400">{{ error }}</p>
  </section>
</template>
