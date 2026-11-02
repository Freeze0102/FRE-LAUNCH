<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { computed, inject, onMounted, onUnmounted, ref } from 'vue'

type Profile = { id: string; name: string; created: number }
type Runtime = { installed: boolean; prepared: boolean; version: string; reason: string | null }
type RustGameFile = { name: string; bytes: number; isDirectory: boolean }
type FileCategory = 'root' | 'mods' | 'resourcepacks' | 'shaderpacks' | 'saves' | 'logs'
const fileCategories: { id: FileCategory; name: string }[] = [
  { id: 'mods', name: 'Моди' }, { id: 'resourcepacks', name: 'Ресурс-паки' },
  { id: 'shaderpacks', name: 'Шейдери' }, { id: 'saves', name: 'Світи' },
  { id: 'logs', name: 'Журнали' }, { id: 'root', name: 'Усі файли' },
]

const { formatMessage: t } = useVIntl()
const m = defineMessages({
  title: { id: 'fre-rust.library.title', defaultMessage: 'Minecraft on Rust' },
  newProfile: { id: 'fre-rust.library.new', defaultMessage: 'Create Rust profile' },
  empty: { id: 'fre-rust.library.empty', defaultMessage: 'No Rust profiles yet. Create one to see it in your library.' },
  ready: { id: 'fre-rust.library.ready', defaultMessage: 'Ready' },
  install: { id: 'fre-rust.library.install', defaultMessage: 'Game files required' },
  play: { id: 'fre-rust.library.play', defaultMessage: 'Play' },
  playing: { id: 'fre-rust.library.playing', defaultMessage: 'Starting...' },
  manage: { id: 'fre-rust.library.manage', defaultMessage: 'Manage' },
  files: { id: 'fre-rust.library.files', defaultMessage: 'Files' },
  modsInfo: { id: 'fre-rust.library.mods-info', defaultMessage: 'Horizon accepts native Rust mods, not CurseForge Java jars.' },
  error: { id: 'fre-rust.library.error', defaultMessage: 'Rust game: {reason}' },
})

const openRustWizard = inject<(profileId?: string) => void>('showFreRustWizard', () => {})
const profiles = ref<Profile[]>([])
const runtime = ref<Runtime | null>(null)
const loading = ref<string | null>(null)
const error = ref('')
const fileProfile = ref<Profile | null>(null)
const fileCategory = ref<FileCategory>('mods')
const fileEntries = ref<RustGameFile[]>([])
const fileFilter = ref('')
const filesLoading = ref(false)
const filteredFiles = computed(() => fileEntries.value.filter(file =>
  file.name.toLowerCase().includes(fileFilter.value.trim().toLowerCase()),
))
let fileRequest = 0
let request = 0
const playable = computed(() => runtime.value?.installed === true && runtime.value?.prepared === true)
function reason(err: unknown) { return err instanceof Error ? err.message : String(err) }

async function refresh() {
  const n = ++request
  try {
    const [instances, info] = await Promise.all([
      invoke<Profile[]>('plugin:fre-rust|rust_profiles'),
      invoke<Runtime>('plugin:fre-rust|rust_runtime'),
    ])
    if (n !== request) return
    profiles.value = instances
    runtime.value = info
    error.value = ''
  } catch (err) {
    if (n === request) error.value = t(m.error, { reason: reason(err) })
  }
}

async function play(profile: Profile) {
  if (loading.value || !playable.value) return
  loading.value = profile.id
  error.value = ''
  try {
    await invoke<string>('plugin:fre-rust|rust_launch', { profileId: profile.id })
  } catch (err) {
    error.value = t(m.error, { reason: reason(err) })
  } finally {
    loading.value = null
  }
}

async function loadFiles(category: FileCategory = fileCategory.value) {
  const profile = fileProfile.value
  if (!profile) return
  const current = ++fileRequest
  fileCategory.value = category
  filesLoading.value = true
  error.value = ''
  fileEntries.value = []
  try {
    const entries = await invoke<RustGameFile[]>('plugin:fre-rust|rust_list_files', {
      profileId: profile.id,
      folder: category,
    })
    if (current === fileRequest) fileEntries.value = entries
  } catch (err) {
    if (current === fileRequest) error.value = t(m.error, { reason: reason(err) })
  } finally {
    if (current === fileRequest) filesLoading.value = false
  }
}
async function openFiles(profile: Profile) {
  fileProfile.value = profile
  fileFilter.value = ''
  await loadFiles('mods')
}
async function openFolder() {
  const profile = fileProfile.value
  if (!profile) return
  error.value = ''
  try {
    await invoke<void>('plugin:fre-rust|rust_open_folder', {
      profileId: profile.id,
      folder: fileCategory.value,
    })
  } catch (err) {
    error.value = t(m.error, { reason: reason(err) })
  }
}
function closeFiles() {
  fileRequest++
  fileProfile.value = null
  fileEntries.value = []
  filesLoading.value = false
}
function fileSize(bytes: number): string {
  return bytes >= 1024 * 1024 ? (bytes / 1048576).toFixed(1) + ' MB' :
    bytes >= 1024 ? (bytes / 1024).toFixed(1) + ' KB' : bytes + ' B'
}
const onRefresh = () => { void refresh() }
onMounted(() => {
  window.addEventListener('fre-rust-library-refresh', onRefresh)
  void refresh()
})
onUnmounted(() => {
  fileRequest++
  request++
  window.removeEventListener('fre-rust-library-refresh', onRefresh)
})
</script>

<template>
  <section class="flex flex-col gap-3" aria-label="Minecraft on Rust">
    <div class="flex items-center justify-between gap-3">
      <h2 class="m-0 text-2xl font-semibold text-contrast">{{ t(m.title) }}</h2>
      <button type="button" class="cursor-pointer rounded-lg bg-button-bg px-4 py-2 text-sm font-semibold text-contrast" @click="openRustWizard()">{{ t(m.newProfile) }}</button>
    </div>
    <p v-if="!profiles.length && !error" class="m-0 rounded-xl border border-solid border-surface-4 bg-bg-raised p-4 text-sm text-secondary">{{ t(m.empty) }}</p>
    <div v-if="profiles.length" class="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
      <article v-for="profile in profiles" :key="profile.id" class="flex min-h-44 flex-col justify-between gap-3 rounded-xl border border-solid border-surface-4 bg-bg-raised p-4 transition-colors hover:border-surface-5">
        <div class="flex items-center gap-3">
          <div class="flex size-12 items-center justify-center rounded-lg bg-button-bg text-xl font-semibold text-brand">R</div>
          <div class="min-w-0">
            <h3 class="m-0 truncate text-base font-semibold text-contrast">{{ profile.name }}</h3>
            <p class="m-0 mt-1 text-xs text-secondary">Rust · {{ runtime?.version || 'Minecraft' }}</p>
            <p class="m-0 mt-1 text-xs" :class="playable ? 'text-brand' : 'text-secondary'">{{ playable ? t(m.ready) : t(m.install) }}</p>
          </div>
        </div>
        <div class="flex gap-2">
          <button type="button" class="flex-1 cursor-pointer rounded-lg border-0 bg-brand px-4 py-2 text-sm font-semibold text-white disabled:opacity-40" :disabled="!playable || !!loading" @click="play(profile)">
            {{ loading === profile.id ? t(m.playing) : t(m.play) }}
          </button>
          <button type="button" class="cursor-pointer rounded-lg bg-button-bg px-3 py-2 text-sm text-contrast" @click="openFiles(profile)">{{ t(m.files) }}</button>
          <button type="button" class="cursor-pointer rounded-lg bg-button-bg px-3 py-2 text-sm text-contrast" @click="openRustWizard(profile.id)">{{ t(m.manage) }}</button>
        </div>
      </article>
    </div>
    <div v-if="fileProfile" class="rounded-xl border border-solid border-surface-4 bg-bg-raised p-4" aria-label="Файли Rust-збірки">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h3 class="m-0 font-semibold text-contrast">Файли · {{ fileProfile.name }}</h3>
          <p class="m-0 mt-1 text-xs text-secondary">Вміст саме цієї Rust-збірки, окремо від Java</p>
        </div>
        <div class="flex gap-2">
          <button type="button" class="cursor-pointer rounded-lg bg-button-bg px-3 py-2 text-sm text-contrast" :disabled="filesLoading" @click="loadFiles()">Оновити</button>
          <button type="button" class="cursor-pointer rounded-lg bg-button-bg px-3 py-2 text-sm text-contrast" @click="openFolder">Папка Windows</button>
          <button type="button" class="cursor-pointer rounded-lg bg-button-bg px-3 py-2 text-sm text-contrast" @click="closeFiles">Закрити</button>
        </div>
      </div>
      <div class="mt-4 flex flex-wrap gap-2">
        <button v-for="category in fileCategories" :key="category.id" type="button"
          class="cursor-pointer rounded-lg px-3 py-2 text-sm"
          :class="fileCategory === category.id ? 'bg-brand text-white' : 'bg-button-bg text-secondary'"
          :aria-pressed="fileCategory === category.id"
          @click="loadFiles(category.id)">{{ category.name }}</button>
      </div>
      <input v-model="fileFilter" aria-label="Пошук встановлених файлів" placeholder="Пошук файлів у цій збірці..."
        class="mt-4 w-full rounded-lg bg-button-bg p-3 text-contrast" />
      <p v-if="filesLoading" class="mt-3 text-sm text-secondary">Читаємо файли…</p>
      <p v-else-if="!filteredFiles.length" class="mt-3 text-sm text-secondary">Файлів не знайдено.</p>
      <div v-else class="mt-3 max-h-64 overflow-auto rounded-lg border border-solid border-surface-4">
        <div v-for="file in filteredFiles" :key="file.name"
          class="flex items-center justify-between gap-3 border-b border-solid border-surface-4 px-3 py-2 text-sm">
          <span class="min-w-0 truncate text-contrast" :title="file.name">{{ file.isDirectory ? '📁' : '📄' }} {{ file.name }}</span>
          <span class="shrink-0 text-xs text-secondary">{{ file.isDirectory ? 'Папка' : fileSize(file.bytes) }}</span>
        </div>
      </div>
      <p class="mt-3 text-xs text-secondary">Цей перегляд не встановлює Java JAR-моди на Rust. Для встановлення потрібні модифікації, які підтримує сам Rust-клієнт.</p>
    </div>
    <p class="m-0 text-xs text-secondary">{{ t(m.modsInfo) }}</p>
    <p v-if="error" role="alert" class="text-sm text-red-400">{{ error }}</p>
  </section>
</template>
