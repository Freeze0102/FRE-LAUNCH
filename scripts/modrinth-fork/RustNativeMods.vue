<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { Button, defineMessages, useVIntl } from '@modrinth/ui'
import { onMounted, onUnmounted, ref, watch } from 'vue'

type ClientMods = {
  zoom: boolean
  zoomFactor: number
  zoomKey: 'c' | 'v' | 'z'
  fpsCounter: boolean
  coordinates: boolean
}
type ModKey = 'zoom' | 'fpsCounter' | 'coordinates'
const defaults = (): ClientMods => ({ zoom: true, zoomFactor: 4, zoomKey: 'c', fpsCounter: false, coordinates: false })

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
  title: { id: 'fre-rust.mods.title', defaultMessage: 'Rust mods' },
  explanation: {
    id: 'fre-rust.mods.explanation',
    defaultMessage: 'Enable the included client mods below or import your own .hmod content files. Java JAR mods are not compatible.',
  },
  builtinTitle: { id: 'fre-rust.mods.builtin-title', defaultMessage: 'Ready to play' },
  builtinHint: { id: 'fre-rust.mods.builtin-hint', defaultMessage: 'These mods are included in FRE LAUNCH. Close the game before changing them; settings apply to this instance on its next launch.' },
  zoom: { id: 'fre-rust.mods.zoom', defaultMessage: 'Zoom' },
  zoomInfo: { id: 'fre-rust.mods.zoom-info', defaultMessage: 'Hold the zoom key to see distant objects. Release it to return to your normal view.' },
  fps: { id: 'fre-rust.mods.fps', defaultMessage: 'FPS counter' },
  fpsInfo: { id: 'fre-rust.mods.fps-info', defaultMessage: 'Shows frames per second in a small panel while playing.' },
  coords: { id: 'fre-rust.mods.coords', defaultMessage: 'Coordinates' },
  coordsInfo: { id: 'fre-rust.mods.coords-info', defaultMessage: 'Keeps your X, Y and Z coordinates visible without opening the debug screen.' },
  enable: { id: 'fre-rust.mods.enable', defaultMessage: 'Enable' },
  disable: { id: 'fre-rust.mods.disable', defaultMessage: 'Disable' },
  enabled: { id: 'fre-rust.mods.enabled', defaultMessage: 'Enabled' },
  disabled: { id: 'fre-rust.mods.disabled', defaultMessage: 'Disabled' },
  zoomFactor: { id: 'fre-rust.mods.zoom-factor', defaultMessage: 'Magnification' },
  zoomKey: { id: 'fre-rust.mods.zoom-key', defaultMessage: 'Hold key' },
  saved: { id: 'fre-rust.mods.saved', defaultMessage: 'Mod settings saved for this instance.' },
  installed: { id: 'fre-rust.mods.installed', defaultMessage: 'Files in this instance' },
  empty: { id: 'fre-rust.mods.empty', defaultMessage: 'No .hmod content files yet.' },
  importFile: { id: 'fre-rust.mods.import-file', defaultMessage: 'Import .hmod' },
  openFolder: { id: 'fre-rust.mods.open-folder', defaultMessage: 'Open mods folder' },
  remove: { id: 'fre-rust.mods.remove', defaultMessage: 'Remove file' },
  refresh: { id: 'fre-rust.mods.refresh', defaultMessage: 'Refresh' },
})
const installed = ref<ContentMod[]>([])
const settings = ref<ClientMods>(defaults())
const saving = ref(false)
const cards: { key: ModKey; title: typeof m.zoom; description: typeof m.zoomInfo }[] = [
  { key: 'zoom', title: m.zoom, description: m.zoomInfo },
  { key: 'fpsCounter', title: m.fps, description: m.fpsInfo },
  { key: 'coordinates', title: m.coords, description: m.coordsInfo },
]
let loadId = 0
const busy = ref(false)
const error = ref('')
const notice = ref('')

async function load() {
  const request = ++loadId
  const profileId = props.profileId
  busy.value = true
  error.value = ''
  try {
    const [raw, clientMods] = await Promise.all([
      invoke<string>('plugin:fre-rust|rust_mods', { profileId, action: 'list', project: null }),
      invoke<ClientMods>('plugin:fre-rust|rust_client_mods', { profileId, settings: null }),
    ])
    const parsed: unknown = JSON.parse(raw)
    if (!Array.isArray(parsed)) throw new Error('Invalid FRE LAUNCH mod list.')
    if (request !== loadId || profileId !== props.profileId) return
    installed.value = parsed as ContentMod[]
    settings.value = clientMods
  } catch (e) { if (request === loadId) error.value = String(e) }
  finally { if (request === loadId) busy.value = false }
}
async function saveSettings(next: ClientMods) {
  if (busy.value || saving.value) return
  const profileId = props.profileId
  saving.value = true
  error.value = ''
  notice.value = ''
  try {
    const saved = await invoke<ClientMods>('plugin:fre-rust|rust_client_mods', { profileId, settings: next })
    if (profileId !== props.profileId) return
    settings.value = saved
    notice.value = t(m.saved)
  } catch (e) { if (profileId === props.profileId) error.value = String(e) }
  finally { saving.value = false }
}
async function setMod(key: ModKey) {
  await saveSettings({ ...settings.value, [key]: !settings.value[key] })
}
async function setZoomFactor(event: Event) {
  const zoomFactor = Number((event.target as HTMLSelectElement).value)
  if (!Number.isInteger(zoomFactor) || zoomFactor < 2 || zoomFactor > 10) return
  await saveSettings({ ...settings.value, zoomFactor })
}
async function setZoomKey(event: Event) {
  const zoomKey = (event.target as HTMLSelectElement).value
  if (zoomKey !== 'c' && zoomKey !== 'v' && zoomKey !== 'z') return
  await saveSettings({ ...settings.value, zoomKey })
}
async function importFile() {
  if (busy.value || saving.value) return
  const profileId = props.profileId
  const selected = await open({
    directory: false,
    multiple: false,
    filters: [{ name: 'Rust content mod', extensions: ['hmod'] }],
  })
  if (typeof selected !== 'string' || profileId !== props.profileId) return
  busy.value = true
  error.value = ''
  notice.value = ''
  try {
    notice.value = await invoke<string>('plugin:fre-rust|rust_import_mod', {
      profileId, source: selected,
    })
  } catch (e) { error.value = String(e) }
  finally { busy.value = false }
  await load()
}
async function removeMod(filename: string) {
  if (busy.value || saving.value) return
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
onUnmounted(() => { loadId++ })
watch(() => props.profileId, () => {
  installed.value = []
  settings.value = defaults()
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
    <p v-if="error" role="alert" class="m-0 text-sm text-red-400">{{ error }}</p>
    <p v-if="notice" role="status" class="m-0 text-sm text-brand">{{ notice }}</p>
    <div class="flex flex-col gap-2">
      <h4 class="m-0 text-base font-semibold text-contrast">{{ t(m.builtinTitle) }}</h4>
      <p class="m-0 text-sm text-secondary">{{ t(m.builtinHint) }}</p>
    </div>
    <div class="grid grid-cols-1 gap-3 lg:grid-cols-3">
      <article v-for="mod in cards" :key="mod.key" class="flex flex-col gap-3 rounded-xl border border-solid border-surface-4 bg-surface-2 p-4">
        <div class="flex items-center justify-between gap-2">
          <h4 class="m-0 text-base font-semibold text-contrast">{{ t(mod.title) }}</h4>
          <span class="text-xs" :class="settings[mod.key] ? 'text-brand' : 'text-secondary'">
            {{ settings[mod.key] ? t(m.enabled) : t(m.disabled) }}
          </span>
        </div>
        <p class="m-0 flex-1 text-sm text-secondary">{{ t(mod.description) }}</p>
        <div v-if="mod.key === 'zoom'" class="flex flex-wrap gap-3">
          <label class="flex flex-col gap-1 text-xs text-secondary">
            {{ t(m.zoomFactor) }}
            <select :value="settings.zoomFactor" :disabled="busy || saving" class="rounded-lg border border-solid border-surface-4 bg-surface-3 p-2 text-contrast" @change="setZoomFactor">
              <option v-for="factor in [2, 4, 6, 8, 10]" :key="factor" :value="factor">{{ factor }}×</option>
            </select>
          </label>
          <label class="flex flex-col gap-1 text-xs text-secondary">
            {{ t(m.zoomKey) }}
            <select :value="settings.zoomKey" :disabled="busy || saving" class="rounded-lg border border-solid border-surface-4 bg-surface-3 p-2 text-contrast" @change="setZoomKey">
              <option value="c">C</option><option value="v">V</option><option value="z">Z</option>
            </select>
          </label>
        </div>
        <Button :disabled="busy || saving" :type="settings[mod.key] ? 'outlined' : undefined" @click="setMod(mod.key)">
          {{ settings[mod.key] ? t(m.disable) : t(m.enable) }}
        </Button>
      </article>
    </div>
    <h4 class="m-0 mt-3 text-base font-semibold text-contrast">{{ t(m.installed) }}</h4>
    <div class="flex flex-wrap items-center gap-2">
      <Button :disabled="busy || saving" @click="importFile">{{ t(m.importFile) }}</Button>
      <Button type="outlined" :disabled="busy || saving" @click="openFolder">{{ t(m.openFolder) }}</Button>
      <Button type="outlined" :disabled="busy || saving" @click="load">{{ t(m.refresh) }}</Button>
    </div>
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
      <Button type="outlined" :disabled="busy || saving" @click="removeMod(mod.project)">
        {{ t(m.remove) }}
      </Button>
    </div>
  </section>
</template>
