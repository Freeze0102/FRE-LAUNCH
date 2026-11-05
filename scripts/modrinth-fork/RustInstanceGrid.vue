<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { FolderOpenIcon, PlayIcon, SettingsIcon, StopCircleIcon, TrashIcon } from '@modrinth/assets'
import { ContextMenu, defineMessages, IconButton, injectNotificationManager, useVIntl } from '@modrinth/ui'
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRouter } from 'vue-router'

import InstanceCardView from '@/components/ui/library/instance-group/instance-card-view.vue'
import ConfirmDeleteInstanceModal from '@/components/ui/modal/ConfirmDeleteInstanceModal.vue'
import type { GameInstance } from '@/helpers/types'

// Rust has its own real game engine. Only the VISUAL Java card is reused;
// a Rust profile must never be passed to Java's installer, process manager,
// bulk-delete routines or instance query APIs.
type NativeProfile = { id: string; name: string; created: number; modified?: number; iconPath?: string | null }
type Runtime = { installed: boolean; prepared: boolean; version: string; reason: string | null }
type Card = {
  key: string
  id: string
  name: string
  kind: 'native' | 'legacy'
  iconPath?: string | null
  version: string
  ready: boolean
  created?: number
  modified?: number
}

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
const router = useRouter()
const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const messages = defineMessages({
  open: { id: 'fre-rust.grid.open', defaultMessage: 'Open {name}' },
  start: { id: 'fre-rust.grid.start', defaultMessage: 'Play {name}' },
  stop: { id: 'fre-rust.grid.stop', defaultMessage: 'Stop {name}' },
  openFolder: { id: 'fre-rust.grid.open-folder', defaultMessage: 'Open folder' },
  settings: { id: 'fre-rust.grid.settings', defaultMessage: 'Settings' },
  delete: { id: 'fre-rust.page.delete', defaultMessage: 'Delete instance' },
})
const nativeProfiles = ref<NativeProfile[]>([])
const runtime = ref<Runtime | null>(null)
const busy = ref<string | null>(null)
const runningIds = ref<string[]>([])
const isRunning = (card: Card) => runningIds.value.includes(card.id)
let unlistenProcess: (() => void) | null = null
let processListenerDisposed = false
const actionsMenu = ref<InstanceType<typeof ContextMenu>>()
const deleteModal = ref<InstanceType<typeof ConfirmDeleteInstanceModal>>()
const deleteTarget = ref<Card | null>(null)
const deleteInstance = computed(() => deleteTarget.value ? viewInstance(deleteTarget.value) : null)
let requestId = 0

const cards = computed<Card[]>(() => {
  const native = nativeProfiles.value.map(profile => ({
    key: 'rust:' + profile.id,
    id: profile.id,
    name: profile.name,
    kind: 'native' as const,
    iconPath: profile.iconPath,
    version: runtime.value?.version || '26.3',
    ready: runtime.value?.installed === true && runtime.value?.prepared === true,
    created: profile.created,
    modified: profile.modified,
  }))
  const term = props.search.trim().toLocaleLowerCase()
  return native.filter(card =>
    !term || card.name.toLocaleLowerCase().includes(term) ||
    'rust'.includes(term) || 'раст'.includes(term),
  )
})

watch(() => cards.value.length, length => emit('count-change', length), { immediate: true })

async function refresh() {
  const request = ++requestId
  try {
    const [profiles, info, running] = await Promise.all([
      invoke<NativeProfile[]>('plugin:fre-rust|rust_profiles'),
      invoke<Runtime>('plugin:fre-rust|rust_runtime'),
      invoke<string[]>('plugin:fre-rust|rust_running').catch((): string[] => []),
    ])
    if (request !== requestId) return
    nativeProfiles.value = profiles
    runtime.value = info
    runningIds.value = running
  } catch (err) {
    if (request === requestId) handleError(err instanceof Error ? err : new Error(String(err)))
  }
}
async function refreshRunning() {
  try {
    runningIds.value = await invoke<string[]>('plugin:fre-rust|rust_running')
  } catch { /* If the backend is unavailable, keep normal Java cards usable. */ }
}

// InstanceCardView is the SAME card renderer used by Fabric, Forge and
// every Java instance. This data is for display only, not Java operations.
function viewInstance(card: Card): GameInstance {
  return {
    id: card.key,
    name: card.name,
    loader: 'Rust',
    game_version: card.version,
    icon_path: card.iconPath ?? null,
    modified: card.modified ?? card.created ?? 0,
  } as unknown as GameInstance
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
function detailUrl(card: Card) {
  return '/rust-instance/' + encodeURIComponent(card.kind) + '/' + encodeURIComponent(card.id)
}
function openCard(card: Card) {
  void router.push(detailUrl(card))
}
async function openFolder(card: Card) {
  try {
    await invoke('plugin:fre-rust|rust_open_folder', { profileId: card.id, folder: 'root' })
  } catch (err) {
    handleError(err instanceof Error ? err : new Error(String(err)))
  }
}
async function askDelete(card: Card) {
  if (card.kind !== 'native') return
  deleteTarget.value = card
  await nextTick()
  deleteModal.value?.show()
}
async function confirmedDelete() {
  const card = deleteTarget.value
  if (!card || card.kind !== 'native') return
  try {
    await invoke('plugin:fre-rust|rust_delete_profile', { profileId: card.id })
    deleteTarget.value = null
    await refresh()
    window.dispatchEvent(new Event('fre-rust-library-refresh'))
  } catch (err) {
    handleError(err instanceof Error ? err : new Error(String(err)))
  }
}
function openContext(event: MouseEvent, card: Card) {
  event.preventDefault()
  event.stopPropagation()
  actionsMenu.value?.open(event, [
    { id: 'open', label: formatMessage(messages.open, { name: card.name }), action: () => openCard(card) },
    isRunning(card)
      ? { id: 'stop', label: formatMessage(messages.stop, { name: card.name }), icon: StopCircleIcon, action: () => { void stop(card) } }
      : { id: 'play', label: formatMessage(messages.start, { name: card.name }), icon: PlayIcon, action: () => { void play(card) } },
    { id: 'settings', label: formatMessage(messages.settings), icon: SettingsIcon, action: () => openCard(card) },
    { id: 'files', label: formatMessage(messages.openFolder), icon: FolderOpenIcon, action: () => { void openFolder(card) } },
    ...(card.kind === 'native'
      ? [{ id: 'delete', label: formatMessage(messages.delete), icon: TrashIcon, action: () => { void askDelete(card) } }]
      : []),
  ])
}
async function play(card: Card) {
  if (busy.value || isRunning(card)) return
  busy.value = card.key
  try {
    if (!card.ready) {
      try {
        await invoke<string>('plugin:fre-rust|rust_import_resources', { source: '' })
        await refresh()
      } catch (err) {
        openCard(card)
        handleError(err instanceof Error ? err : new Error(String(err)))
        return
      }
    }
    const status = await invoke<Runtime>('plugin:fre-rust|rust_runtime')
    if (!status.installed || !status.prepared) {
      openCard(card)
      return
    }
    await invoke<string>('plugin:fre-rust|rust_launch', { profileId: card.id })
    await refreshRunning()
  } catch (err) {
    handleError(err instanceof Error ? err : new Error(String(err)))
  } finally {
    busy.value = null
  }
}
async function stop(card: Card) {
  if (busy.value || !isRunning(card)) return
  busy.value = card.key
  try {
    await invoke('plugin:fre-rust|rust_stop', { profileId: card.id })
    await refreshRunning()
  } catch (err) {
    handleError(err instanceof Error ? err : new Error(String(err)))
  } finally {
    busy.value = null
  }
}
const onRefresh = () => { void refresh() }
onMounted(() => {
  processListenerDisposed = false
  window.addEventListener('fre-rust-library-refresh', onRefresh)
  window.addEventListener('focus', refreshRunning)
  void listen('fre-rust-process-changed', () => { void refreshRunning() })
    .then(unlisten => {
      if (processListenerDisposed) unlisten()
      else unlistenProcess = unlisten
    }).catch(() => {})
  void refresh()
})
onUnmounted(() => {
  requestId++
  window.removeEventListener('fre-rust-library-refresh', onRefresh)
  window.removeEventListener('focus', refreshRunning)
  processListenerDisposed = true
  unlistenProcess?.()
  unlistenProcess = null
  emit('count-change', 0)
})
</script>

<template>
  <ContextMenu ref="actionsMenu" :label="formatMessage(messages.settings)" />
  <ConfirmDeleteInstanceModal
    ref="deleteModal"
    :instances="deleteInstance ? [deleteInstance] : []"
    @delete="confirmedDelete"
  />
  <div v-for="(card, index) in cards" :key="card.key" :style="cardStyle(index)">
    <InstanceCardView
      class="group/card cursor-pointer -outline-offset-2 focus-visible:!outline-2 hover:brightness-110"
      :instance="viewInstance(card)"
      data-library-instance-card
      :data-instance-id="card.key"
      role="button"
      tabindex="0"
      :aria-label="formatMessage(messages.open, { name: card.name })"
      @click="openCard(card)"
      @contextmenu.prevent="openContext($event, card)"
      @keydown.enter.prevent="openCard(card)"
    >
      <template #leading="{ compact }">
        <div class="relative flex items-center justify-center" :class="compact ? 'size-10' : 'size-12'">
          <IconButton
            v-if="!busy"
            v-tooltip="formatMessage(isRunning(card) ? messages.stop : messages.start, { name: card.name })"
            :label="formatMessage(isRunning(card) ? messages.stop : messages.start, { name: card.name })"
            type="colored"
            color="brand"
            :size="compact ? 'md' : 'lg'"
            class="origin-bottom scale-75 opacity-0 transition-opacity group-hover/card:scale-100 group-hover/card:opacity-100"
            @click.stop="isRunning(card) ? stop(card) : play(card)"
          >
            <StopCircleIcon v-if="isRunning(card)" />
            <PlayIcon v-else class="translate-x-px" />
          </IconButton>
        </div>
      </template>
    </InstanceCardView>
  </div>
</template>
