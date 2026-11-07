<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { FolderOpenIcon, MoreVerticalIcon, PlusIcon, RefreshCwIcon, SearchIcon, SettingsIcon, TrashIcon, XIcon } from '@modrinth/assets'
import { Button, Combobox, ContentCardTable, defineMessages, Input, NewModal, TeleportOverflowMenu, useVIntl } from '@modrinth/ui'
import type { ContentCardTableItem, ContentCardTableSortColumn, ContentCardTableSortDirection } from '@modrinth/ui'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import RustModCatalog from '@/components/ui/RustModCatalog.vue'

type ClientMods = { zoom: boolean; zoomFactor: number; zoomKey: 'c' | 'v' | 'z'; fpsCounter: boolean; coordinates: boolean }
type ModKey = 'zoom' | 'fpsCounter' | 'coordinates'
type ContentMod = { project: string; slug: string; title: string; number: string; file: string; source: string; enabled?: boolean; authors?: string[]; icon_url?: string }
const defaults = (): ClientMods => ({ zoom: true, zoomFactor: 4, zoomKey: 'c', fpsCounter: false, coordinates: false })
const props = defineProps<{ profileId: string; catalogOpen?: boolean; running?: boolean }>()
const emit = defineEmits<{ browse: []; back: [] }>()
const { formatMessage: t } = useVIntl()
const m = defineMessages({
  title: { id: 'fre-rust.mods.title', defaultMessage: 'Mods' },
  add: { id: 'fre-rust.mods.add', defaultMessage: 'Add mods' },
  search: { id: 'fre-rust.mods.search', defaultMessage: 'Search installed mods…' },
  all: { id: 'fre-rust.mods.all', defaultMessage: 'All mods' },
  builtinTitle: { id: 'fre-rust.mods.builtin-title', defaultMessage: 'Included in FRE LAUNCH' },
  installed: { id: 'fre-rust.mods.installed', defaultMessage: 'Installed files' },
  empty: { id: 'fre-rust.mods.empty', defaultMessage: 'No mods match your search.' },
  zoom: { id: 'fre-rust.mods.zoom', defaultMessage: 'Zoom' },
  fps: { id: 'fre-rust.mods.fps', defaultMessage: 'FPS counter' },
  coords: { id: 'fre-rust.mods.coords', defaultMessage: 'Coordinates' },
  enable: { id: 'fre-rust.mods.enable', defaultMessage: 'Enable' },
  disable: { id: 'fre-rust.mods.disable', defaultMessage: 'Disable' },
  zoomFactor: { id: 'fre-rust.mods.zoom-factor', defaultMessage: 'Magnification' },
  zoomKey: { id: 'fre-rust.mods.zoom-key', defaultMessage: 'Hold key' },
  saved: { id: 'fre-rust.mods.saved', defaultMessage: 'Mod settings saved for this instance.' },
  importFile: { id: 'fre-rust.mods.import-file', defaultMessage: 'Add from file' },
  openFolder: { id: 'fre-rust.mods.open-folder', defaultMessage: 'Open mods folder' },
  remove: { id: 'fre-rust.mods.remove', defaultMessage: 'Delete' },
  refresh: { id: 'fre-rust.mods.refresh', defaultMessage: 'Refresh' },
  selected: { id: 'fre-rust.mods.selected', defaultMessage: '{count} selected' },
  settings: { id: 'fre-rust.mods.settings', defaultMessage: 'Mod settings' },
  confirmRemove: { id: 'fre-rust.mods.confirm-remove', defaultMessage: 'Delete selected mods?' },
  removeHint: { id: 'fre-rust.mods.remove-hint', defaultMessage: 'These mod files will be removed from this instance.' },
  cancel: { id: 'fre-rust.mods.cancel', defaultMessage: 'Cancel' },
  close: { id: 'fre-rust.mods.close', defaultMessage: 'Close' },
  running: { id: 'fre-rust.mods.running', defaultMessage: 'Close the game before changing mods.' },
  nextLaunch: { id: 'fre-rust.mods.next-launch', defaultMessage: 'Changes apply on the next game launch.' },
})
const installed = ref<ContentMod[]>([])
const settings = ref<ClientMods>(defaults())
const saving = ref(false), busy = ref(false), error = ref(''), notice = ref('')
const query = ref(''), filter = ref('all'), selected = ref<string[]>([])
const sortBy = ref<ContentCardTableSortColumn>('project')
const sortDirection = ref<ContentCardTableSortDirection>('asc')
const catalogVisited = ref(!!props.catalogOpen)
const zoomModal = ref<InstanceType<typeof NewModal>>()
const deleteModal = ref<InstanceType<typeof NewModal>>()
const deleting = ref<string[]>([])
let deletionProfile = '', loadId = 0
const settingsBlocked = computed(() => busy.value || saving.value)
const blocked = computed(() => busy.value || saving.value || !!props.running)
const builtinKeys: ModKey[] = ['zoom', 'fpsCounter', 'coordinates']
function builtinIcon(key: ModKey) {
  const shapes = { zoom: '<circle cx="10.5" cy="10.5" r="5.5"/><path d="m15 15 5 5M8 10.5h5m-2.5-2.5v5"/>', fpsCounter: '<path d="M4 19h16M7 15V9m5 6V4m5 11V7"/>', coordinates: '<path d="M19 10c0 5-7 11-7 11S5 15 5 10a7 7 0 1 1 14 0Z"/><circle cx="12" cy="10" r="2.5"/>' }
  return 'data:image/svg+xml,' + encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect width="24" height="24" rx="6" fill="#263849"/><g fill="none" stroke="#65d8c6" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">${shapes[key]}</g></svg>`)
}
const rows = computed<ContentCardTableItem[]>(() => {
  const included = builtinKeys.map(key => ({
    id: 'builtin:' + key,
    project: { id: key, slug: key, title: t(key === 'zoom' ? m.zoom : key === 'fpsCounter' ? m.fps : m.coords), icon_url: builtinIcon(key) },
    version: { id: key, version_number: '26.3', file_name: t(m.builtinTitle) },
    owner: { id: 'fre', name: 'FRE LAUNCH', type: 'organization' as const },
    enabled: settings.value[key], hideDelete: true, hideSwitchVersion: true, disabled: settingsBlocked.value,
    overflowOptions: key === 'zoom' ? [{ id: 'settings', label: t(m.settings), icon: SettingsIcon, action: () => zoomModal.value?.show() }] : [],
  }))
  const local = installed.value.map(mod => ({
    id: 'local:' + mod.file,
    project: { id: mod.slug, slug: mod.slug, title: mod.title, icon_url: mod.icon_url },
    version: { id: mod.file, version_number: mod.number, file_name: mod.file },
    owner: mod.authors?.length ? { id: mod.slug, name: mod.authors.join(', '), type: 'user' as const } : undefined,
    enabled: mod.enabled === true, hideSwitchVersion: true, disabled: blocked.value,
  }))
  return [...included, ...local]
})
const filtered = computed(() => {
  const words = query.value.toLowerCase().trim().split(/\s+/).filter(Boolean)
  return rows.value.filter(row => (filter.value !== 'builtin' || row.id.startsWith('builtin:')) && (filter.value !== 'local' || row.id.startsWith('local:')) && words.every(word => [row.project.title, row.version?.file_name, row.owner?.name].join(' ').toLowerCase().includes(word)))
    .sort((a, b) => (sortBy.value === 'version' ? (a.version?.version_number || '').localeCompare(b.version?.version_number || '', undefined, { numeric: true }) : a.project.title.localeCompare(b.project.title)) * (sortDirection.value === 'asc' ? 1 : -1))
})
const removable = computed(() => selected.value.filter(id => id.startsWith('local:') && rows.value.some(row => row.id === id)))
async function load(clearFeedback = true) {
  const request = ++loadId, profileId = props.profileId
  busy.value = true
  if (clearFeedback) error.value = ''
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
    selected.value = selected.value.filter(id => rows.value.some(row => row.id === id))
  } catch (e) { if (request === loadId) error.value = String(e) }
  finally { if (request === loadId) busy.value = false }
}
async function saveSettings(next: ClientMods) {
  if (settingsBlocked.value) return
  const profileId = props.profileId, request = loadId
  saving.value = true; error.value = ''; notice.value = ''
  try {
    const saved = await invoke<ClientMods>('plugin:fre-rust|rust_client_mods', { profileId, settings: next })
    if (profileId !== props.profileId || request !== loadId) return
    settings.value = saved; notice.value = t(m.saved)
  } catch (e) { if (profileId === props.profileId && request === loadId) error.value = String(e) }
  finally { if (profileId === props.profileId && request === loadId) saving.value = false }
}
async function setMod(key: ModKey) { await saveSettings({ ...settings.value, [key]: !settings.value[key] }) }
async function saveZoomFactor(value: unknown) {
  const zoomFactor = Number(value)
  if (Number.isInteger(zoomFactor) && zoomFactor >= 2 && zoomFactor <= 10) await saveSettings({ ...settings.value, zoomFactor })
}
async function saveZoomKey(value: unknown) {
  if (value === 'c' || value === 'v' || value === 'z') await saveSettings({ ...settings.value, zoomKey: value })
}
async function setZoomFactor(event: Event) { await saveZoomFactor((event.target as HTMLSelectElement).value) }
async function setZoomKey(event: Event) { await saveZoomKey((event.target as HTMLSelectElement).value) }
async function changeEnabled(id: string, enabled: boolean) {
  const key = id.slice(8) as ModKey
  if (id.startsWith('builtin:') && builtinKeys.includes(key)) { await saveSettings({ ...settings.value, [key]: enabled }); return }
  if (blocked.value) return
  const mod = installed.value.find(mod => 'local:' + mod.file === id)
  if (!mod) return
  const profileId = props.profileId
  busy.value = true; error.value = ''; notice.value = ''
  try { await invoke('plugin:fre-rust|rust_mods', { profileId, action: enabled ? 'enable' : 'disable', project: mod.file }) }
  catch (e) { if (profileId === props.profileId) error.value = String(e) }
  finally { if (profileId === props.profileId) await load(false) }
}
async function bulkToggle(enabled: boolean) {
  const profileId = props.profileId
  for (const id of [...selected.value]) {
    if (profileId !== props.profileId || props.running) return
    if (rows.value.find(row => row.id === id)?.enabled !== enabled) await changeEnabled(id, enabled)
    if (error.value) break
  }
}
async function importFile() {
  if (blocked.value) return
  const profileId = props.profileId
  const result = await open({ directory: false, multiple: true, filters: [{ name: 'Rust content mod', extensions: ['hmod'] }] })
  const paths = typeof result === 'string' ? [result] : Array.isArray(result) ? result : []
  if (!paths.length || profileId !== props.profileId || props.running) return
  busy.value = true; error.value = ''; notice.value = ''
  try { for (const source of paths) { if (profileId !== props.profileId) return; await invoke('plugin:fre-rust|rust_import_mod', { profileId, source }) } }
  catch (e) { if (profileId === props.profileId) error.value = String(e) }
  finally { if (profileId === props.profileId) await load(false) }
}
async function removeMod(filename: string) {
  if (blocked.value) return
  const profileId = props.profileId
  busy.value = true; error.value = ''; notice.value = ''
  try { await invoke('plugin:fre-rust|rust_mods', { profileId, action: 'remove', project: filename }) }
  catch (e) { if (profileId === props.profileId) error.value = String(e) }
  finally { if (profileId === props.profileId) await load(false) }
}
function askRemove(ids: string[]) {
  if (blocked.value) return
  deleting.value = ids.filter(id => id.startsWith('local:') && installed.value.some(mod => 'local:' + mod.file === id))
  if (!deleting.value.length) return
  deletionProfile = props.profileId; deleteModal.value?.show()
}
async function confirmRemove() {
  deleteModal.value?.hide()
  const ids = [...deleting.value], profileId = deletionProfile
  deleting.value = []
  for (const id of ids) { if (profileId !== props.profileId || props.running) return; await removeMod(id.slice(6)); if (error.value) break }
}
async function openFolder() {
  const profileId = props.profileId
  try { await invoke('plugin:fre-rust|rust_open_folder', { profileId, folder: 'mods' }) }
  catch (e) { if (profileId === props.profileId) error.value = String(e) }
}
function openBuiltin(key: ModKey) {
  emit('back')
  query.value = ''; filter.value = 'all'; selected.value = ['builtin:' + key]
  if (key === 'zoom') zoomModal.value?.show()
}
function sort(column: ContentCardTableSortColumn, direction: ContentCardTableSortDirection) { sortBy.value = column; sortDirection.value = direction }
onMounted(() => { void load() })
onUnmounted(() => { loadId++ })
watch(() => props.catalogOpen, open => { if (open) catalogVisited.value = true })
watch(() => props.profileId, () => {
  loadId++; installed.value = []; settings.value = defaults(); selected.value = []; deleting.value = []
  saving.value = false; error.value = ''; notice.value = ''; query.value = ''; filter.value = 'all'
  catalogVisited.value = !!props.catalogOpen; zoomModal.value?.hide(); deleteModal.value?.hide(); void load()
})
</script>

<template>
  <section class="flex flex-col gap-4">
    <RustModCatalog v-if="catalogVisited" v-show="catalogOpen" :profile-id="profileId" :installed-files="installed.map(mod => mod.file)" :disabled="blocked" @close="emit('back')" @installed="load(false)" @builtin="openBuiltin" />
    <div v-show="!catalogOpen" class="flex flex-col gap-4">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <h2 class="m-0 text-xl font-semibold text-contrast">{{ t(m.title) }} <span class="text-base font-normal text-secondary">{{ rows.length }}</span></h2>
        <div class="flex gap-2">
          <Button type="colored" color="brand" @click="emit('browse')"><PlusIcon />{{ t(m.add) }}</Button>
          <TeleportOverflowMenu :label="t(m.settings)" :options="[{ id: 'file', label: t(m.importFile), icon: PlusIcon, disabled: blocked, action: importFile }, { id: 'folder', label: t(m.openFolder), icon: FolderOpenIcon, action: openFolder }, { id: 'refresh', label: t(m.refresh), icon: RefreshCwIcon, disabled: blocked, action: () => load() }]"><MoreVerticalIcon /></TeleportOverflowMenu>
        </div>
      </div>
      <p v-if="error" role="alert" class="m-0 text-sm text-red">{{ error }}</p>
      <p v-if="notice" role="status" class="m-0 text-sm text-brand">{{ notice }}</p>
      <p v-if="running" role="status" class="m-0 text-sm text-secondary">{{ t(m.running) }}</p>
      <div class="flex flex-wrap items-center gap-2">
        <Input v-model="query" :icon="SearchIcon" :placeholder="t(m.search)" :aria-label="t(m.search)" clearable class="min-w-52 flex-1" />
        <Combobox v-model="filter" :options="[{ value: 'all', label: t(m.all) }, { value: 'builtin', label: t(m.builtinTitle) }, { value: 'local', label: t(m.installed) }]" :aria-label="t(m.all)" class="!w-56" />
      </div>
      <div v-if="selected.length" class="flex flex-wrap items-center gap-2 rounded-xl bg-surface-3 p-3">
        <span class="mr-auto text-sm font-semibold">{{ t(m.selected, { count: selected.length }) }}</span>
        <Button :disabled="blocked" @click="bulkToggle(true)">{{ t(m.enable) }}</Button>
        <Button :disabled="blocked" @click="bulkToggle(false)">{{ t(m.disable) }}</Button>
        <Button type="outlined" color="red" :disabled="blocked || !removable.length" @click="askRemove(removable)"><TrashIcon />{{ t(m.remove) }}</Button>
        <Button type="quiet" :aria-label="t(m.cancel)" @click="selected = []"><XIcon /></Button>
      </div>
      <ContentCardTable v-if="filtered.length" v-model:selected-ids="selected" :items="filtered" show-selection sortable :sort-by="sortBy" :sort-direction="sortDirection" :virtualized="false" show-item-actions @sort="sort" @update:enabled="changeEnabled" @delete="id => askRemove([id])" />
      <div v-else class="flex flex-col items-center gap-3 rounded-2xl border border-solid border-surface-4 p-12 text-secondary"><p class="m-0">{{ t(m.empty) }}</p><Button @click="emit('browse')"><PlusIcon />{{ t(m.add) }}</Button></div>
      <p class="m-0 text-xs text-secondary">{{ t(m.nextLaunch) }}</p>
    </div>
    <NewModal ref="zoomModal" :header="t(m.zoom)" max-width="440px">
      <div class="flex flex-col gap-4">
        <label class="flex flex-col gap-2 font-semibold">{{ t(m.zoomFactor) }}<Combobox :model-value="settings.zoomFactor" :options="[2, 4, 6, 8, 10].map(value => ({ value, label: value + '×' }))" :disabled="settingsBlocked" @update:model-value="saveZoomFactor" /></label>
        <label class="flex flex-col gap-2 font-semibold">{{ t(m.zoomKey) }}<Combobox :model-value="settings.zoomKey" :options="['c', 'v', 'z'].map(value => ({ value, label: value.toUpperCase() }))" :disabled="settingsBlocked" @update:model-value="saveZoomKey" /></label>
        <p class="m-0 text-sm text-secondary">{{ t(m.nextLaunch) }}</p>
      </div>
      <template #actions><Button @click="zoomModal?.hide()">{{ t(m.close) }}</Button></template>
    </NewModal>
    <NewModal ref="deleteModal" :header="t(m.confirmRemove)" max-width="500px">
      <p class="m-0">{{ t(m.removeHint) }}</p><ul><li v-for="id in deleting" :key="id">{{ rows.find(row => row.id === id)?.project.title }}</li></ul>
      <template #actions><Button type="outlined" @click="deleteModal?.hide()">{{ t(m.cancel) }}</Button><Button type="colored" color="red" :disabled="blocked" @click="confirmRemove"><TrashIcon />{{ t(m.remove) }}</Button></template>
    </NewModal>
  </section>
</template>
