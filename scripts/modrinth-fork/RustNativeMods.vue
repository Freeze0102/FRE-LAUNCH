<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { Button, defineMessages, useVIntl } from '@modrinth/ui'
import { onMounted, ref, watch } from 'vue'

type ContentMod = {
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
  title: { id: 'fre-rust.mods.title', defaultMessage: 'Rust content mods' },
  explanation: {
    id: 'fre-rust.mods.explanation',
    defaultMessage: 'Import .hmod content files directly into this FRE LAUNCH instance; enable them in the game’s Mods settings. Fabric/Forge JARs are not compatible. Native .hnmod source mods require a separate trusted build.',
  },
  installed: { id: 'fre-rust.mods.installed', defaultMessage: 'Files in this instance' },
  empty: { id: 'fre-rust.mods.empty', defaultMessage: 'No .hmod content files yet.' },
  importFile: { id: 'fre-rust.mods.import-file', defaultMessage: 'Import .hmod' },
  openFolder: { id: 'fre-rust.mods.open-folder', defaultMessage: 'Open mods folder' },
  remove: { id: 'fre-rust.mods.remove', defaultMessage: 'Remove file' },
  refresh: { id: 'fre-rust.mods.refresh', defaultMessage: 'Refresh' },
})
const installed = ref<ContentMod[]>([])
const busy = ref(false)
const error = ref('')
const notice = ref('')

async function load() {
  if (busy.value) return
  busy.value = true
  error.value = ''
  try {
    const raw = await invoke<string>('plugin:fre-rust|rust_mods', {
      profileId: props.profileId, action: 'list', project: null,
    })
    const parsed: unknown = JSON.parse(raw)
    if (!Array.isArray(parsed)) throw new Error('Invalid FRE LAUNCH mod list.')
    installed.value = parsed as ContentMod[]
  } catch (e) { error.value = String(e) }
  finally { busy.value = false }
}
async function importFile() {
  if (busy.value) return
  const selected = await open({
    directory: false,
    multiple: false,
    filters: [{ name: 'Rust content mod', extensions: ['hmod'] }],
  })
  if (typeof selected !== 'string') return
  busy.value = true
  error.value = ''
  notice.value = ''
  try {
    notice.value = await invoke<string>('plugin:fre-rust|rust_import_mod', {
      profileId: props.profileId, source: selected,
    })
  } catch (e) { error.value = String(e) }
  finally { busy.value = false }
  await load()
}
async function removeMod(filename: string) {
  if (busy.value) return
  busy.value = true
  error.value = ''
  notice.value = ''
  try {
    notice.value = await invoke<string>('plugin:fre-rust|rust_mods', {
      profileId: props.profileId, action: 'remove', project: filename,
    })
  } catch (e) { error.value = String(e) }
  finally { busy.value = false }
  await load()
}
async function openFolder() {
  try {
    await invoke<void>('plugin:fre-rust|rust_open_folder', {
      profileId: props.profileId, folder: 'mods',
    })
  } catch (e) { error.value = String(e) }
}
onMounted(() => { void load() })
watch(() => props.profileId, () => {
  installed.value = []
  notice.value = ''
  void load()
})
</script>

<template>
  <section class="flex flex-col gap-4">
    <div class="flex flex-col gap-1">
      <h3 class="m-0 text-lg font-semibold text-contrast">{{ t(m.title) }}</h3>
      <p class="m-0 text-sm text-secondary">{{ t(m.explanation) }}</p>
    </div>
    <div class="flex flex-wrap items-center gap-2">
      <Button :disabled="busy" @click="importFile">{{ t(m.importFile) }}</Button>
      <Button type="outlined" :disabled="busy" @click="openFolder">{{ t(m.openFolder) }}</Button>
      <Button type="outlined" :disabled="busy" @click="load">{{ t(m.refresh) }}</Button>
    </div>
    <p v-if="error" role="alert" class="m-0 text-sm text-red-400">{{ error }}</p>
    <p v-if="notice" role="status" class="m-0 text-sm text-brand">{{ notice }}</p>
    <p v-if="!installed.length" class="m-0 text-sm text-secondary">{{ t(m.empty) }}</p>
    <div
      v-for="mod in installed"
      :key="mod.project"
      class="flex items-center justify-between gap-3 rounded-xl border border-solid border-surface-4 p-3"
    >
      <div class="min-w-0">
        <p class="m-0 font-semibold text-contrast">{{ mod.title }}</p>
        <p class="m-0 text-xs text-secondary">{{ mod.file }}</p>
      </div>
      <Button type="outlined" :disabled="busy" @click="removeMod(mod.project)">
        {{ t(m.remove) }}
      </Button>
    </div>
  </section>
</template>
