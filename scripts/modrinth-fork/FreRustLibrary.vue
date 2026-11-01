<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { computed, inject, onMounted, onUnmounted, ref } from 'vue'

type Profile = { id: string; name: string; created: number }
type Runtime = { installed: boolean; prepared: boolean; version: string; reason: string | null }

const { formatMessage: t } = useVIntl()
const m = defineMessages({
  title: { id: 'fre-rust.library.title', defaultMessage: 'Minecraft on Rust' },
  newProfile: { id: 'fre-rust.library.new', defaultMessage: 'Create Rust profile' },
  ready: { id: 'fre-rust.library.ready', defaultMessage: 'Ready' },
  install: { id: 'fre-rust.library.install', defaultMessage: 'Game files required' },
  play: { id: 'fre-rust.library.play', defaultMessage: 'Play' },
  playing: { id: 'fre-rust.library.playing', defaultMessage: 'Starting...' },
  manage: { id: 'fre-rust.library.manage', defaultMessage: 'Manage' },
  error: { id: 'fre-rust.library.error', defaultMessage: 'Rust game: {reason}' },
})

const openRustWizard = inject<() => void>('showFreRustWizard', () => {})
const profiles = ref<Profile[]>([])
const runtime = ref<Runtime | null>(null)
const loading = ref<string | null>(null)
const error = ref('')
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

const onRefresh = () => { void refresh() }
onMounted(() => {
  window.addEventListener('fre-rust-library-refresh', onRefresh)
  void refresh()
})
onUnmounted(() => {
  request++
  window.removeEventListener('fre-rust-library-refresh', onRefresh)
})
</script>

<template>
  <section v-if="profiles.length > 0" class="flex flex-col gap-3">
    <div class="flex items-center justify-between gap-3">
      <h2 class="m-0 text-2xl font-semibold text-contrast">{{ t(m.title) }}</h2>
      <button type="button" class="cursor-pointer rounded-lg bg-button-bg px-4 py-2 text-sm font-semibold text-contrast" @click="openRustWizard">{{ t(m.newProfile) }}</button>
    </div>
    <div class="grid grid-cols-1 gap-3 md:grid-cols-2 xl:grid-cols-3">
      <article v-for="profile in profiles" :key="profile.id" class="flex min-h-36 flex-col justify-between gap-3 rounded-xl border border-solid border-surface-4 bg-bg-raised p-4">
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
          <button type="button" class="cursor-pointer rounded-lg bg-button-bg px-4 py-2 text-sm text-contrast" @click="openRustWizard">{{ t(m.manage) }}</button>
        </div>
      </article>
    </div>
    <p v-if="error" role="alert" class="text-sm text-red-400">{{ error }}</p>
  </section>
</template>
