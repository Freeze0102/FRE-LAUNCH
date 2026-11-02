<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { computed, inject, onMounted, onUnmounted, ref } from 'vue'

type RustInstance = { id: string; name: string; version: string | null; selected: boolean }
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
async function openFiles(profile: RustInstance) {
  error.value = ''
  try {
    await invoke<void>('plugin:fre-horizon|horizon_open_folder', {
      exePath: executable.value,
      name: profile.id,
      folder: 'root',
    })
  } catch (e) {
    error.value = t(messages.error, { reason: reason(e) })
  }
}
function update() { void refresh() }
onMounted(() => {
  window.addEventListener('fre-horizon-library-refresh', update)
  void refresh()
})
onUnmounted(() => {
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
        @click="openHorizonWizard"
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
    <p v-if="error" role="alert" class="text-sm text-red-400">{{ error }}</p>
  </section>
</template>
