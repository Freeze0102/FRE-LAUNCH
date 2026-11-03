<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { Button, defineMessages, useVIntl } from '@modrinth/ui'
import { computed, onMounted, ref, watch } from 'vue'

type ModSearch = {
  id: string
  slug: string
  title: string
  summary?: string
  author?: string
  support?: string
  reason?: string
}
type InstalledMod = {
  project: string
  slug: string
  title: string
  number: string
  file: string
  source: string
}
const props = defineProps<{ profileId: string }>()
const { formatMessage: t } = useVIntl()
const m = defineMessages({
  title: { id: 'fre-rust.mods.title', defaultMessage: 'Horizon native mods' },
  explanation: { id: 'fre-rust.mods.explanation', defaultMessage: 'Search the official Horizon registry. Files are verified and installed into this Rust instance only. Fabric and Forge JARs are not native Rust mods.' },
  search: { id: 'fre-rust.mods.search', defaultMessage: 'Search' },
  placeholder: { id: 'fre-rust.mods.placeholder', defaultMessage: 'Find a Horizon mod…' },
  installed: { id: 'fre-rust.mods.installed', defaultMessage: 'Installed' },
  empty: { id: 'fre-rust.mods.empty', defaultMessage: 'No mods installed from the Horizon registry yet.' },
  install: { id: 'fre-rust.mods.install', defaultMessage: 'Install' },
  remove: { id: 'fre-rust.mods.remove', defaultMessage: 'Remove' },
  refresh: { id: 'fre-rust.mods.refresh', defaultMessage: 'Refresh' },
  none: { id: 'fre-rust.mods.none', defaultMessage: 'No matching native mods found.' },
})
const query = ref('')
const found = ref<ModSearch[]>([])
const installed = ref<InstalledMod[]>([])
const searched = ref(false)
const busy = ref(false)
const error = ref('')
const notice = ref('')
const installedIds = computed(() => new Set(installed.value.map(mod => mod.project)))
async function call(action: string, project = '') {
  return invoke<string>('plugin:fre-rust|rust_mods', {
    profileId: props.profileId, action, project,
  })
}
function parseList<T>(raw: string): T[] {
  const decoded: unknown = JSON.parse(raw)
  if (!Array.isArray(decoded)) throw new Error('Horizon returned an unexpected mod list.')
  return decoded as T[]
}
async function load() {
  if (busy.value) return
  busy.value = true
  error.value = ''
  try {
    installed.value = parseList<InstalledMod>(await call('list'))
  } catch (e) { error.value = String(e) }
  finally { busy.value = false }
}
async function search() {
  if (busy.value || !query.value.trim()) return
  busy.value = true
  error.value = ''
  notice.value = ''
  try {
    found.value = parseList<ModSearch>(await call('search', query.value.trim()))
    searched.value = true
  } catch (e) { error.value = String(e) }
  finally { busy.value = false }
}
async function change(action: 'install' | 'remove', project: string) {
  if (busy.value) return
  busy.value = true
  error.value = ''
  notice.value = ''
  try {
    notice.value = await call(action, project)
    installed.value = parseList<InstalledMod>(await call('list'))
  } catch (e) { error.value = String(e) }
  finally { busy.value = false }
}
onMounted(() => { void load() })
watch(() => props.profileId, () => {
  found.value = []
  installed.value = []
  searched.value = false
  void load()
})
</script>

<template>
  <section class="flex flex-col gap-4">
    <div class="flex flex-col gap-1">
      <h3 class="m-0 text-lg font-semibold text-contrast">{{ t(m.title) }}</h3>
      <p class="m-0 text-sm text-secondary">{{ t(m.explanation) }}</p>
    </div>
    <form class="flex flex-wrap gap-2" @submit.prevent="search">
      <input
        v-model="query"
        class="min-w-44 flex-1 rounded-lg border border-solid border-surface-4 bg-surface-2 px-3 py-2 text-contrast"
        :placeholder="t(m.placeholder)"
        :aria-label="t(m.placeholder)"
        maxlength="100"
        :disabled="busy"
      />
      <Button :disabled="busy || !query.trim()" @click="search">{{ t(m.search) }}</Button>
    </form>
    <p v-if="error" role="alert" class="m-0 text-sm text-red-400">{{ error }}</p>
    <p v-if="notice" role="status" class="m-0 text-sm text-brand">{{ notice }}</p>
    <div v-if="searched" class="flex flex-col gap-2">
      <p v-if="!found.length" class="m-0 text-secondary">{{ t(m.none) }}</p>
      <div
        v-for="mod in found"
        :key="mod.id"
        class="flex items-start justify-between gap-3 rounded-xl border border-solid border-surface-4 bg-surface-2 p-3"
      >
        <div class="min-w-0">
          <p class="m-0 font-semibold text-contrast">{{ mod.title }}</p>
          <p v-if="mod.summary" class="m-0 mt-1 text-sm text-secondary">{{ mod.summary }}</p>
          <p v-if="mod.support" class="m-0 mt-1 text-xs text-secondary">
            {{ mod.support }} <span v-if="mod.reason">— {{ mod.reason }}</span>
          </p>
        </div>
        <Button :disabled="busy || installedIds.has(mod.id)" @click="change('install', mod.id)">
          {{ installedIds.has(mod.id) ? t(m.installed) : t(m.install) }}
        </Button>
      </div>
    </div>
    <div class="flex items-center justify-between">
      <h3 class="m-0 text-lg font-semibold text-contrast">{{ t(m.installed) }}</h3>
      <Button type="outlined" :disabled="busy" @click="load">{{ t(m.refresh) }}</Button>
    </div>
    <p v-if="!installed.length" class="m-0 text-sm text-secondary">{{ t(m.empty) }}</p>
    <div
      v-for="mod in installed"
      :key="mod.project"
      class="flex items-center justify-between gap-3 rounded-xl border border-solid border-surface-4 p-3"
    >
      <div class="min-w-0">
        <p class="m-0 font-semibold text-contrast">{{ mod.title }}</p>
        <p class="m-0 text-xs text-secondary">{{ mod.number }} · {{ mod.file }}</p>
      </div>
      <Button type="outlined" :disabled="busy" @click="change('remove', mod.project)">
        {{ t(m.remove) }}
      </Button>
    </div>
  </section>
</template>
