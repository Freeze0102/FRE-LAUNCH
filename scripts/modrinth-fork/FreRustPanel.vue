<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { computed, onMounted, ref } from 'vue'

type Profile = { id: string; name: string; created: number }
type Runtime = { installed: boolean; prepared: boolean; version: string; reason: string | null }

const { formatMessage: t } = useVIntl()
const m = defineMessages({
  title: { id: 'fre-rust.setup.title', defaultMessage: 'Minecraft on Rust — FRE LAUNCH' },
  intro: { id: 'fre-rust.setup.intro', defaultMessage: 'Your own Rust Minecraft profiles, installed and launched directly by FRE LAUNCH. No second launcher.' },
  name: { id: 'fre-rust.setup.name', defaultMessage: 'Profile name' },
  create: { id: 'fre-rust.setup.create', defaultMessage: 'Create Rust profile' },
  creating: { id: 'fre-rust.setup.creating', defaultMessage: 'Creating...' },
  play: { id: 'fre-rust.setup.play', defaultMessage: 'Play' },
  playing: { id: 'fre-rust.setup.playing', defaultMessage: 'Starting...' },
  close: { id: 'fre-rust.setup.close', defaultMessage: 'Close' },
  installed: { id: 'fre-rust.setup.installed', defaultMessage: 'Native Rust game bundled' },
  missing: { id: 'fre-rust.setup.missing', defaultMessage: 'Native Rust game is not installed in this FRE LAUNCH build.' },
  ready: { id: 'fre-rust.setup.ready', defaultMessage: 'Minecraft resources prepared' },
  notReady: { id: 'fre-rust.setup.not-ready', defaultMessage: 'Minecraft resources not prepared' },
  details: { id: 'fre-rust.setup.details', defaultMessage: 'The Rust engine is part of FRE LAUNCH. Import the prepared game resources from your own Minecraft Java Edition installation; private resources are never included in public updates.' },
  source: { id: 'fre-rust.setup.source', defaultMessage: 'Prepared game resources folder (optional; leave blank for automatic detection)' },
  browse: { id: 'fre-rust.setup.browse', defaultMessage: 'Choose folder' },
  import: { id: 'fre-rust.setup.import', defaultMessage: 'Import Minecraft resources' },
  importing: { id: 'fre-rust.setup.importing', defaultMessage: 'Importing game resources...' },
  importHelp: { id: 'fre-rust.setup.import-help', defaultMessage: 'Import a prepared resource folder containing horizon-resources.json. Your original files and worlds are not moved or deleted.' },
  none: { id: 'fre-rust.setup.none', defaultMessage: 'No Rust profiles yet.' },
  logs: { id: 'fre-rust.setup.logs', defaultMessage: 'Show game log' },
  files: { id: 'fre-rust.setup.files', defaultMessage: 'Files' },
  worlds: { id: 'fre-rust.setup.worlds', defaultMessage: 'Worlds' },
  mods: { id: 'fre-rust.setup.mods', defaultMessage: 'Native mods' },
  resourcepacks: { id: 'fre-rust.setup.resourcepacks', defaultMessage: 'Resource packs' },
  shaderpacks: { id: 'fre-rust.setup.shaderpacks', defaultMessage: 'Shader packs' },
  error: { id: 'fre-rust.setup.error', defaultMessage: 'Error: {reason}' },
})
const visible = ref(false)
const profiles = ref<Profile[]>([])
const runtime = ref<Runtime | null>(null)
const name = ref('')
const source = ref('')
const selectedId = ref('')
const error = ref('')
const notice = ref('')
const logs = ref('')
const pending = ref<'load' | 'create' | 'play' | 'logs' | 'import' | null>(null)
const playable = computed(() => runtime.value?.installed === true && runtime.value?.prepared === true)
const gameFolders = computed(() => [
  { key: 'root' as const, label: t(m.files) },
  { key: 'saves' as const, label: t(m.worlds) },
  { key: 'mods' as const, label: t(m.mods) },
  { key: 'resourcepacks' as const, label: t(m.resourcepacks) },
  { key: 'shaderpacks' as const, label: t(m.shaderpacks) },
])

function reason(e: unknown): string {
  return e instanceof Error ? e.message : String(e)
}

async function refresh() {
  const [all, info] = await Promise.all([
    invoke<Profile[]>('plugin:fre-rust|rust_profiles'),
    invoke<Runtime>('plugin:fre-rust|rust_runtime'),
  ])
  profiles.value = all
  runtime.value = info
  if (!all.some(p => p.id === selectedId.value)) {
    selectedId.value = all[0]?.id ?? ''
  }
  window.dispatchEvent(new Event('fre-rust-library-refresh'))
}

async function show(profileId?: string) {
  visible.value = true
  if (pending.value) return
  pending.value = 'load'
  error.value = ''
  try {
    await refresh()
    if (profileId && profiles.value.some(p => p.id === profileId)) selectedId.value = profileId
  }
  catch (e) { error.value = t(m.error, { reason: reason(e) }) }
  finally { pending.value = null }
}

function hide() {
  if (pending.value) return
  visible.value = false
}

async function create() {
  if (pending.value || !name.value.trim()) return
  pending.value = 'create'
  error.value = ''
  notice.value = ''
  try {
    const profile = await invoke<Profile>('plugin:fre-rust|rust_create_profile', {
      name: name.value,
    })
    await refresh()
    selectedId.value = profile.id
    name.value = ''
    notice.value = profile.name
  } catch (e) { error.value = t(m.error, { reason: reason(e) }) }
  finally { pending.value = null }
}

async function chooseSourceFolder() {
  if (pending.value) return
  error.value = ''
  try {
    const selected = await open({ multiple: false, directory: true })
    if (typeof selected === 'string') source.value = selected
  } catch (e) {
    error.value = t(m.error, { reason: reason(e) })
  }
}

async function importResources() {
  if (pending.value || !runtime.value?.installed || runtime.value.prepared) return
  pending.value = 'import'
  error.value = ''
  notice.value = ''
  try {
    const result = await invoke<string>('plugin:fre-rust|rust_import_resources', {
      source: source.value.trim(),
    })
    notice.value = result
    await refresh()
  } catch (e) { error.value = t(m.error, { reason: reason(e) }) }
  finally { pending.value = null }
}

async function play() {
  if (!selectedId.value || !playable.value || pending.value) return
  pending.value = 'play'
  error.value = ''
  try {
    notice.value = await invoke<string>('plugin:fre-rust|rust_launch', { profileId: selectedId.value })
  } catch (e) { error.value = t(m.error, { reason: reason(e) }) }
  finally { pending.value = null }
}

async function openProfileFiles(folder: 'root' | 'saves' | 'mods' | 'resourcepacks' | 'shaderpacks') {
  if (!selectedId.value || pending.value) return
  error.value = ''
  try {
    await invoke<void>('plugin:fre-rust|rust_open_folder', { profileId: selectedId.value, folder })
  } catch (e) { error.value = t(m.error, { reason: reason(e) }) }
}

async function showLogs() {
  if (!selectedId.value || pending.value) return
  pending.value = 'logs'
  error.value = ''
  try {
    logs.value = await invoke<string>('plugin:fre-rust|rust_logs', { profileId: selectedId.value })
  } catch (e) { error.value = t(m.error, { reason: reason(e) }) }
  finally { pending.value = null }
}
onMounted(() => { void refresh().catch(() => {}) })
defineExpose({ show, hide })
</script>

<template>
  <div v-if="visible" class="fixed inset-0 z-[500] flex items-center justify-center bg-black/80 p-4 backdrop-blur-sm" role="dialog" aria-modal="true" :aria-label="t(m.title)">
    <section class="max-h-[90vh] w-full max-w-2xl overflow-y-auto rounded-2xl border border-solid border-surface-4 bg-bg-raised p-6">
      <div class="flex items-start justify-between gap-4">
        <div>
          <h2 class="m-0 text-2xl font-bold text-contrast">{{ t(m.title) }}</h2>
          <p class="mt-2 text-sm text-secondary">{{ t(m.intro) }}</p>
        </div>
        <button type="button" class="rounded-lg bg-button-bg px-3 py-2 text-contrast" :disabled="!!pending" @click="hide">{{ t(m.close) }}</button>
      </div>

      <div class="mt-4 grid gap-3 md:grid-cols-2">
        <div class="rounded-xl border border-solid border-surface-5 p-3">
          <p class="m-0 text-sm font-semibold" :class="runtime?.installed ? 'text-brand' : 'text-secondary'">
            {{ runtime?.installed ? t(m.installed) : t(m.missing) }}
          </p>
        </div>
        <div class="rounded-xl border border-solid border-surface-5 p-3">
          <p class="m-0 text-sm font-semibold" :class="runtime?.prepared ? 'text-brand' : 'text-secondary'">
            {{ runtime?.prepared ? t(m.ready) : t(m.notReady) }}
          </p>
        </div>
      </div>
      <p v-if="runtime?.reason" class="mt-3 text-sm text-secondary">{{ runtime.reason }}</p>
      <p class="mt-2 text-xs text-secondary">{{ t(m.details) }}</p>
      <div v-if="runtime?.installed && !runtime.prepared" class="mt-4 rounded-xl border border-solid border-surface-5 p-4">
        <p class="m-0 text-xs text-secondary">{{ t(m.importHelp) }}</p>
        <form class="mt-3 flex flex-wrap gap-2" @submit.prevent="importResources">
          <input v-model="source" class="min-w-48 flex-1 rounded-lg bg-button-bg p-3 text-contrast" :aria-label="t(m.source)" :placeholder="t(m.source)" :disabled="!!pending" />
          <button type="button" class="cursor-pointer rounded-lg border border-solid border-surface-4 bg-button-bg px-4 py-2 font-semibold text-contrast disabled:opacity-40" :disabled="!!pending" @click="chooseSourceFolder">
            {{ t(m.browse) }}
          </button>
          <button type="submit" class="rounded-lg border-0 bg-brand px-4 py-2 font-semibold text-white disabled:opacity-40" :disabled="!!pending">
            {{ pending === 'import' ? t(m.importing) : t(m.import) }}
          </button>
        </form>
      </div>

      <form class="mt-5 flex flex-wrap gap-2" @submit.prevent="create">
        <input v-model="name" class="min-w-48 flex-1 rounded-lg bg-button-bg p-3 text-contrast" maxlength="64" :placeholder="t(m.name)" :disabled="!!pending" />
        <button type="submit" class="rounded-lg border-0 bg-brand px-4 py-2 font-semibold text-white disabled:opacity-40" :disabled="!!pending || !name.trim()">
          {{ pending === 'create' ? t(m.creating) : t(m.create) }}
        </button>
      </form>
      <div class="mt-5 rounded-xl border border-solid border-surface-5 p-4">
        <p v-if="!profiles.length" class="m-0 text-secondary">{{ t(m.none) }}</p>
        <template v-else>
          <select v-model="selectedId" class="w-full rounded-lg bg-button-bg p-3 text-contrast" :disabled="!!pending">
            <option v-for="profile in profiles" :key="profile.id" :value="profile.id">{{ profile.name }}</option>
          </select>
          <div class="mt-3 flex flex-wrap gap-2">
            <button type="button" class="rounded-lg border-0 bg-brand px-5 py-2 font-semibold text-white disabled:opacity-40" :disabled="!!pending || !playable" @click="play">
              {{ pending === 'play' ? t(m.playing) : t(m.play) }}
            </button>
            <button type="button" class="rounded-lg bg-button-bg px-4 py-2 text-contrast disabled:opacity-40" :disabled="!!pending" @click="showLogs">{{ t(m.logs) }}</button>
          </div>
          <div class="mt-3 flex flex-wrap gap-2">
            <button v-for="entry in gameFolders" :key="entry.key" type="button"
              class="cursor-pointer rounded-lg border border-solid border-surface-4 bg-button-bg px-3 py-2 text-xs text-contrast disabled:opacity-40"
              :disabled="!!pending" @click="openProfileFiles(entry.key)">{{ entry.label }}</button>
          </div>
        </template>
      </div>
      <p v-if="error" role="alert" class="mt-4 text-sm text-red-400">{{ error }}</p>
      <p v-if="notice" role="status" class="mt-4 text-sm text-brand">{{ notice }}</p>
      <pre v-if="logs" class="mt-4 max-h-48 overflow-auto whitespace-pre-wrap rounded-lg bg-button-bg p-3 text-xs text-secondary">{{ logs }}</pre>
    </section>
  </div>
</template>
