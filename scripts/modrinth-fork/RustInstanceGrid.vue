<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { computed, inject, onMounted, onUnmounted, ref, watch } from 'vue'

type NativeProfile = { id: string; name: string; created: number }
type Runtime = { installed: boolean; prepared: boolean; version: string; reason: string | null }
type HorizonProfile = { id: string; name: string; version: string | null; selected: boolean }
type HorizonStatus = {
  instances: HorizonProfile[]
  installedVersions: string[]
  preparedVersions: string[]
  selectedVersion: string | null
}
type Card = {
  key: string
  id: string
  name: string
  kind: 'native' | 'horizon'
  version: string
  ready: boolean
}
type FileCategory = 'mods' | 'resourcepacks' | 'shaderpacks' | 'saves' | 'logs' | 'root'
type GameFile = { name: string; bytes: number; isDirectory: boolean }

const props = defineProps<{
  startIndex: number
  columnCount: number
  cardWidth: number
  cardHeight: number
  gap: number
  compact: boolean
  search: string
}>()
const emit = defineEmits<{ (event: 'count-change', count: number): void }>()
const { formatMessage: t } = useVIntl()
const m = defineMessages({
  open: { id: 'fre-rust.grid.open', defaultMessage: 'Open {name}' },
  start: { id: 'fre-rust.grid.start', defaultMessage: 'Play {name}' },
  close: { id: 'fre-rust.grid.close', defaultMessage: 'Close' },
  starting: { id: 'fre-rust.grid.starting', defaultMessage: 'Starting…' },
  play: { id: 'fre-rust.grid.play', defaultMessage: 'Play' },
  setup: { id: 'fre-rust.grid.setup', defaultMessage: 'Set up game' },
  settings: { id: 'fre-rust.grid.settings', defaultMessage: 'Settings' },
  openFolder: { id: 'fre-rust.grid.open-folder', defaultMessage: 'Open folder' },
  files: { id: 'fre-rust.grid.files', defaultMessage: 'Instance files' },
  searchFiles: { id: 'fre-rust.grid.search-files', defaultMessage: 'Search files…' },
  refresh: { id: 'fre-rust.grid.refresh', defaultMessage: 'Refresh' },
  reading: { id: 'fre-rust.grid.reading', defaultMessage: 'Reading files…' },
  noFiles: { id: 'fre-rust.grid.no-files', defaultMessage: 'No files in this folder.' },
  folder: { id: 'fre-rust.grid.folder', defaultMessage: 'Folder' },
  nativeRequired: { id: 'fre-rust.grid.native-required', defaultMessage: 'Import your own Minecraft resources in Rust settings before playing.' },
  gameRequired: { id: 'fre-rust.grid.game-required', defaultMessage: 'Your own installed Minecraft resources are required.' },
  horizonRequired: { id: 'fre-rust.grid.horizon-required', defaultMessage: 'Prepare this Horizon version in settings before playing.' },
  modsWarning: { id: 'fre-rust.grid.mods-warning', defaultMessage: 'Java JAR mods are not supported by the native Rust client. Each profile keeps its own worlds and files.' },
  mods: { id: 'fre-rust.grid.mods', defaultMessage: 'Mods' },
  resourcepacks: { id: 'fre-rust.grid.resourcepacks', defaultMessage: 'Resource packs' },
  shaderpacks: { id: 'fre-rust.grid.shaderpacks', defaultMessage: 'Shaders' },
  saves: { id: 'fre-rust.grid.saves', defaultMessage: 'Worlds' },
  logs: { id: 'fre-rust.grid.logs', defaultMessage: 'Logs' },
  root: { id: 'fre-rust.grid.root', defaultMessage: 'All files' },
})
const showRustWizard = inject<(id?: string) => void>('showFreRustWizard', () => {})
const showHorizonWizard = inject<(id?: string) => void>('showHorizonWizard', () => {})

const nativeProfiles = ref<NativeProfile[]>([])
const runtime = ref<Runtime | null>(null)
const horizonStatus = ref<HorizonStatus | null>(null)
const horizonExecutable = ref('')
const error = ref('')
const busy = ref<string | null>(null)
const selected = ref<Card | null>(null)
const category = ref<FileCategory>('mods')
const files = ref<GameFile[]>([])
const fileFilter = ref('')
const filesLoading = ref(false)
const fileCategories: FileCategory[] = ['mods', 'resourcepacks', 'shaderpacks', 'saves', 'logs', 'root']
let requestId = 0
let fileRequestId = 0
const errorText = (value: unknown) => value instanceof Error ? value.message : String(value)
const cards = computed<Card[]>(() => {
  const native = nativeProfiles.value.map(profile => ({
    key: 'rust:' + profile.id,
    id: profile.id,
    name: profile.name,
    kind: 'native' as const,
    version: runtime.value?.version || 'Rust',
    ready: runtime.value?.installed === true && runtime.value?.prepared === true,
  }))
  const legacy = (horizonStatus.value?.instances || [])
    .filter(profile => profile.id === 'horizon' || /^[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}$/.test(profile.id))
    .map(profile => {
      const version = profile.version && profile.version !== 'latest'
        ? profile.version
        : horizonStatus.value?.selectedVersion || horizonStatus.value?.installedVersions[0] || ''
      return {
        key: 'horizon:' + profile.id,
        id: profile.id,
        name: profile.name,
        kind: 'horizon' as const,
        version: version || 'Rust',
        ready: !!version
          && horizonStatus.value!.installedVersions.includes(version)
          && horizonStatus.value!.preparedVersions.includes(version),
      }
    })
  const term = props.search.trim().toLocaleLowerCase()
  return [...native, ...legacy].filter(card =>
    !term || card.name.toLocaleLowerCase().includes(term) ||
    'rust'.includes(term) || 'раст'.includes(term),
  )
})
const filteredFiles = computed(() => files.value.filter(file =>
  file.name.toLocaleLowerCase().includes(fileFilter.value.trim().toLocaleLowerCase()),
))
watch(() => cards.value.length, length => emit('count-change', length), { immediate: true })

async function refresh() {
  const request = ++requestId
  // Each backend is independent: a missing Horizon executable must never
  // conceal bundled native Rust profiles or emit a global red library error.
  try {
    const [profiles, info] = await Promise.all([
      invoke<NativeProfile[]>('plugin:fre-rust|rust_profiles'),
      invoke<Runtime>('plugin:fre-rust|rust_runtime'),
    ])
    if (request !== requestId) return
    nativeProfiles.value = profiles
    runtime.value = info
  } catch (err) {
    if (request === requestId) error.value = 'Rust: ' + errorText(err)
  }
  try {
    const path = await invoke<string | null>('plugin:fre-horizon|horizon_detect_launcher')
    if (request !== requestId) return
    if (!path) {
      horizonExecutable.value = ''
      horizonStatus.value = null
      return
    }
    const status = await invoke<HorizonStatus>('plugin:fre-horizon|horizon_status', { exePath: path })
    if (request !== requestId) return
    horizonExecutable.value = path
    horizonStatus.value = status
  } catch {
    // An obsolete Horizon path must not show a broken phantom section.
    if (request === requestId) {
      horizonExecutable.value = ''
      horizonStatus.value = null
    }
  }
}

function cardStyle(index: number) {
  const position = props.startIndex + index
  const columns = Math.max(1, props.columnCount)
  return {
    position: 'absolute' as const,
    width: props.cardWidth + 'px',
    left: (position % columns) * (props.cardWidth + props.gap) + 'px',
    top: Math.floor(position / columns) * (props.cardHeight + props.gap) + 'px',
    height: props.cardHeight + 'px',
  }
}
function openCard(card: Card) {
  selected.value = card
  error.value = ''
  fileFilter.value = ''
  void loadFiles('mods')
}
function closeCard() {
  fileRequestId++
  selected.value = null
  files.value = []
  filesLoading.value = false
  error.value = ''
}
function manage(card: Card) {
  if (card.kind === 'native') showRustWizard(card.id)
  else showHorizonWizard(card.id)
}
async function play(card: Card) {
  if (busy.value) return
  if (!card.ready) {
    selected.value = card
    error.value = card.kind === 'native'
      ? runtime.value?.reason || t(m.nativeRequired)
      : t(m.horizonRequired)
    return
  }
  busy.value = card.key
  error.value = ''
  try {
    if (card.kind === 'native') {
      await invoke<string>('plugin:fre-rust|rust_launch', { profileId: card.id })
    } else {
      await invoke<string>('plugin:fre-horizon|horizon_direct_play', {
        exePath: horizonExecutable.value,
        name: card.id,
      })
    }
  } catch (err) {
    selected.value = card
    error.value = errorText(err)
  } finally {
    busy.value = null
  }
}
async function loadFiles(folder: FileCategory = category.value) {
  const card = selected.value
  if (!card) return
  const current = ++fileRequestId
  category.value = folder
  filesLoading.value = true
  files.value = []
  try {
    const entries = card.kind === 'native'
      ? await invoke<GameFile[]>('plugin:fre-rust|rust_list_files', { profileId: card.id, folder })
      : await invoke<GameFile[]>('plugin:fre-horizon|horizon_list_files', {
        exePath: horizonExecutable.value, name: card.id, folder,
      })
    if (current === fileRequestId) files.value = entries
  } catch (err) {
    if (current === fileRequestId) error.value = errorText(err)
  } finally {
    if (current === fileRequestId) filesLoading.value = false
  }
}
async function openExplorer() {
  const card = selected.value
  if (!card) return
  try {
    if (card.kind === 'native') {
      await invoke<void>('plugin:fre-rust|rust_open_folder', { profileId: card.id, folder: category.value })
    } else {
      await invoke<void>('plugin:fre-horizon|horizon_open_folder', {
        exePath: horizonExecutable.value, name: card.id, folder: category.value,
      })
    }
  } catch (err) {
    error.value = errorText(err)
  }
}
function readableSize(bytes: number) {
  return bytes >= 1048576 ? (bytes / 1048576).toFixed(1) + ' MB' :
    bytes >= 1024 ? (bytes / 1024).toFixed(1) + ' KB' : bytes + ' B'
}
const onRefresh = () => { void refresh() }
onMounted(() => {
  window.addEventListener('fre-rust-library-refresh', onRefresh)
  window.addEventListener('fre-horizon-library-refresh', onRefresh)
  void refresh()
})
onUnmounted(() => {
  requestId++
  fileRequestId++
  window.removeEventListener('fre-rust-library-refresh', onRefresh)
  window.removeEventListener('fre-horizon-library-refresh', onRefresh)
  emit('count-change', 0)
})
</script>

<template>
  <template v-for="(card, index) in cards" :key="card.key">
    <div class="min-w-0 w-full" :style="cardStyle(index)">
      <article
        data-library-instance-card
        :data-instance-id="card.key"
        role="button"
        tabindex="0"
        :aria-label="t(m.open, { name: card.name })"
        class="group/card relative flex h-full min-w-0 cursor-pointer flex-col rounded-[22px] border border-solid border-surface-4 bg-bg-raised p-3 text-contrast transition-[border-color,filter,transform] duration-150 hover:border-surface-5 hover:brightness-110 focus-visible:outline-2 focus-visible:outline-brand"
        :class="props.compact ? 'flex-row items-center gap-3' : ''"
        @click.stop="openCard(card)"
        @keydown.enter.prevent="openCard(card)"
        @keydown.space.prevent="openCard(card)"
      >
        <div
          class="relative flex shrink-0 items-center justify-center overflow-hidden rounded-2xl bg-surface-3 text-contrast"
          :class="props.compact ? 'size-11' : 'aspect-square w-full'"
        >
          <span class="select-none font-bold tracking-tight text-secondary" :class="props.compact ? 'text-xl' : 'text-6xl'">R</span>
          <button
            type="button"
            :aria-label="t(m.start, { name: card.name })"
            class="absolute inset-0 flex cursor-pointer items-center justify-center border-0 bg-black/40 text-white opacity-0 transition-opacity group-hover/card:opacity-100 focus-visible:opacity-100"
            @click.stop="play(card)"
          >
            <span class="flex size-11 items-center justify-center rounded-xl bg-brand text-xl">▶</span>
          </button>
        </div>
        <div class="mt-2 min-w-0 flex-1" :class="props.compact ? '!mt-0' : ''">
          <h3 class="m-0 truncate text-sm font-semibold" :title="card.name">{{ card.name }}</h3>
          <p class="m-0 mt-1 truncate text-xs text-secondary">Rust {{ card.version }}</p>
        </div>
      </article>
    </div>
  </template>

  <Teleport to="body">
    <div v-if="selected" class="fixed inset-0 z-[9998] flex items-center justify-center bg-black/65 p-4" @click.self="closeCard">
      <section
        role="dialog"
        aria-modal="true"
        :aria-label="'Rust · ' + selected.name"
        class="flex max-h-[90vh] w-full max-w-2xl flex-col overflow-hidden rounded-2xl border border-solid border-surface-4 bg-bg-raised text-contrast shadow-2xl"
        @keydown.esc.stop="closeCard"
      >
        <div class="flex items-center justify-between gap-4 border-b border-solid border-surface-4 p-5">
          <div class="min-w-0">
            <h2 class="m-0 truncate text-xl font-bold">{{ selected.name }}</h2>
            <p class="m-0 mt-1 text-sm text-secondary">Minecraft · Rust {{ selected.version }}</p>
          </div>
          <button type="button" class="rounded-lg bg-button-bg px-3 py-2" :aria-label="t(m.close)" @click="closeCard">✕</button>
        </div>
        <div class="overflow-y-auto p-5">
          <div class="flex flex-wrap gap-2">
            <button type="button" class="rounded-lg border-0 bg-brand px-5 py-2 font-semibold text-white disabled:opacity-50"
              :disabled="!!busy" @click="play(selected)">
              {{ busy === selected.key ? t(m.starting) : selected.ready ? '▶ ' + t(m.play) : t(m.setup) }}
            </button>
            <button type="button" class="rounded-lg bg-button-bg px-4 py-2 text-sm font-semibold" @click="manage(selected)">
              {{ t(m.settings) }}
            </button>
            <button type="button" class="rounded-lg bg-button-bg px-4 py-2 text-sm font-semibold" @click="openExplorer">
              {{ t(m.openFolder) }}
            </button>
          </div>
          <p v-if="!selected.ready" class="mt-3 text-sm text-secondary">
            {{ selected.kind === 'native'
              ? runtime?.reason || t(m.gameRequired)
              : t(m.horizonRequired) }}
          </p>
          <p v-if="error" role="alert" class="mt-3 text-sm text-red-400">{{ error }}</p>
          <div class="mt-5 border-t border-solid border-surface-4 pt-4">
            <h3 class="m-0 mb-3 text-base font-semibold">{{ t(m.files) }}</h3>
            <div class="flex flex-wrap gap-2">
              <button v-for="item in fileCategories" :key="item" type="button"
                class="cursor-pointer rounded-lg px-3 py-2 text-sm"
                :class="category === item ? 'bg-brand text-white' : 'bg-button-bg text-secondary'"
                :aria-pressed="category === item"
                @click="loadFiles(item)">{{ t(m[item]) }}</button>
            </div>
            <div class="mt-3 flex gap-2">
              <input v-model="fileFilter" class="min-w-0 flex-1 rounded-lg bg-button-bg p-3 text-sm text-contrast"
                :aria-label="t(m.searchFiles)" :placeholder="t(m.searchFiles)" />
              <button type="button" class="rounded-lg bg-button-bg px-3 text-sm" @click="loadFiles()">{{ t(m.refresh) }}</button>
            </div>
            <p v-if="filesLoading" class="mt-3 text-sm text-secondary">{{ t(m.reading) }}</p>
            <p v-else-if="!filteredFiles.length" class="mt-3 text-sm text-secondary">{{ t(m.noFiles) }}</p>
            <div v-else class="mt-3 max-h-64 overflow-y-auto rounded-lg border border-solid border-surface-4">
              <div v-for="file in filteredFiles" :key="file.name"
                class="flex items-center justify-between gap-3 border-b border-solid border-surface-4 px-3 py-2 text-sm">
                <span class="min-w-0 truncate" :title="file.name">{{ file.isDirectory ? '📁' : '📄' }} {{ file.name }}</span>
                <span class="shrink-0 text-xs text-secondary">{{ file.isDirectory ? t(m.folder) : readableSize(file.bytes) }}</span>
              </div>
            </div>
          </div>
          <p class="mt-3 text-xs text-secondary">{{ t(m.modsWarning) }}</p>
        </div>
      </section>
    </div>
  </Teleport>
</template>
