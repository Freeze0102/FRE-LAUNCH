<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { Button, Input, NewModal, injectNotificationManager, useVIntl, defineMessages } from '@modrinth/ui'
import { PlusIcon, RefreshCwIcon, SearchIcon } from '@modrinth/assets'
import { computed, onUnmounted, ref, watch } from 'vue'
import WorldItem from '@/components/ui/world/WorldItem.vue'
import { get_server_status, type World, type ServerStatus } from '@/helpers/worlds'
const props = defineProps<{ profileId: string; running?: boolean; starting?: boolean }>()
const emit = defineEmits<{ play: [destination: { world?: string; server?: string }]; stop: [] }>()
const { formatMessage: t } = useVIntl()
const { handleError } = injectNotificationManager()
const m = defineMessages({
 search: { id: 'fre-rust.worlds.search', defaultMessage: 'Search worlds and servers…' },
 add: { id: 'fre-rust.worlds.add', defaultMessage: 'Add server' },
 refresh: { id: 'fre-rust.page.retry', defaultMessage: 'Refresh' },
 empty: { id: 'fre-rust.worlds.empty', defaultMessage: 'No worlds or servers yet. Create a world in the game or add a server here.' },
 name: { id: 'fre-rust.page.rename', defaultMessage: 'Name' },
 address: { id: 'fre-rust.worlds.address', defaultMessage: 'Server address' },
 cancel: { id: 'fre-rust.mods.cancel', defaultMessage: 'Cancel' },
 remove: { id: 'fre-rust.worlds.remove', defaultMessage: 'Remove saved server' },
 all: { id: 'fre-rust.worlds.all', defaultMessage: 'All' },
 online: { id: 'fre-rust.worlds.online', defaultMessage: 'Online' },
 offline: { id: 'fre-rust.worlds.offline', defaultMessage: 'Offline' },
})
const worlds = ref<World[]>([]), query = ref(''), filter = ref('all'), busy = ref(false), saving = ref(false)
const states = ref<Record<string, ServerStatus>>({}), refreshing = ref<Record<string, boolean>>({})
const addModal = ref<InstanceType<typeof NewModal>>(), removeModal = ref<InstanceType<typeof NewModal>>()
const serverName = ref(''), serverAddress = ref(''), removing = ref<World | null>(null)
let generation = 0, disposed = false
const shown = computed(() => worlds.value.filter(world => {
 const matches = (world.name + ' ' + (world.type === 'server' ? world.address : world.path)).toLocaleLowerCase().includes(query.value.trim().toLocaleLowerCase())
 const online = world.type === 'server' && states.value[world.address]?.players != null
 return matches && (filter.value === 'all' || (filter.value === 'online' ? online : !online))
}).slice().sort((a,b) => (b.last_played ?? '').localeCompare(a.last_played ?? '') || a.name.localeCompare(b.name)))
async function ping(world: World, request = generation) {
 if (world.type !== 'server') return
 const address = world.address
 refreshing.value = { ...refreshing.value, [address]: true }
 try {
  const status = await get_server_status(address)
  if (!disposed && request === generation) states.value = { ...states.value, [address]: status }
 } catch { if (request === generation) { const next = { ...states.value }; delete next[address]; states.value = next } }
 finally { if (request === generation) refreshing.value = { ...refreshing.value, [address]: false } }
}
async function load() {
 const request = ++generation, profileId = props.profileId
 busy.value = true
 try {
  const result = await invoke<World[]>('plugin:fre-rust|rust_worlds', { profileId })
  if (disposed || request !== generation || profileId !== props.profileId) return
  worlds.value = result
  // Bound parallel pings so a large saved server list cannot flood the network.
  const servers = result.filter(world => world.type === 'server').slice(0,100)
  void (async () => { for (let i=0; i<servers.length && request===generation && !disposed; i+=4) await Promise.all(servers.slice(i,i+4).map(world => ping(world,request))) })()
 } catch (error) { if (request === generation) handleError(error instanceof Error ? error : new Error(String(error))) }
 finally { if (request === generation) busy.value = false }
}
async function save(action: 'add' | 'remove') {
 if (props.running || saving.value) return
 const chosen = removing.value, profileId = props.profileId
 if (action === 'remove' && (!chosen || chosen.type !== 'server')) return
 saving.value = true
 try {
  await invoke('plugin:fre-rust|rust_server_edit', { profileId, action, name: action === 'add' ? serverName.value.trim() : chosen?.name, address: action === 'add' ? serverAddress.value.trim() : chosen?.type === 'server' ? chosen.address : '' })
  if (profileId !== props.profileId || disposed) return
  addModal.value?.hide(); removeModal.value?.hide(); serverName.value = ''; serverAddress.value = ''; removing.value = null
  await load()
 } catch(error) { handleError(error instanceof Error ? error : new Error(String(error))) }
 finally { saving.value = false }
}
function play(world: World) { emit('play', world.type === 'server' ? { server: world.address } : { world: world.path }) }
function askRemove(world: World) { if (world.type === 'server') { removing.value = world; removeModal.value?.show() } }
function openFolder() { void invoke('plugin:fre-rust|rust_open_folder', { profileId: props.profileId, folder: 'saves' }).catch(handleError) }
watch(() => props.profileId, () => { removing.value = null; serverName.value = ''; serverAddress.value = ''; addModal.value?.hide(); removeModal.value?.hide(); states.value = {}; worlds.value = []; void load() }, { immediate: true })
watch(() => props.running, () => { void load() })
onUnmounted(() => { disposed = true; generation++ })
</script>
<template>
 <div class="flex flex-col gap-4">
  <div class="flex flex-wrap items-center gap-2"><Input v-model="query" :icon="SearchIcon" :placeholder="t(m.search)" clearable class="min-w-52 flex-1" /><Button :disabled="running || saving" @click="addModal?.show()"><PlusIcon />{{ t(m.add) }}</Button></div>
  <div class="flex flex-wrap items-center gap-2"><Button v-for="value in ['all', 'online', 'offline']" :key="value" :type="filter === value ? 'colored' : 'outlined'" :color="filter === value ? 'brand' : undefined" @click="filter = value">{{ t(m[value as 'all' | 'online' | 'offline']) }}</Button><Button class="ml-auto" type="quiet" :disabled="busy" @click="load"><RefreshCwIcon />{{ t(m.refresh) }}</Button></div>
  <WorldItem v-for="world in shown" :key="world.type === 'server' ? world.address : world.path" :world="world" native-instance :playing-instance="running" :starting-instance="starting" :supports-world-quick-play="true" :supports-server-quick-play="true" :server-status="world.type === 'server' ? states[world.address] : undefined" :refreshing="world.type === 'server' && refreshing[world.address]" @play="play(world)" @stop="emit('stop')" @refresh="ping(world)" @delete="askRemove(world)" @open-folder="openFolder" />
  <p v-if="!busy && !shown.length" class="py-12 text-center text-secondary">{{ t(m.empty) }}</p>
  <NewModal ref="addModal" :header="t(m.add)" max-width="480px"><form id="fre-native-add-server" class="flex flex-col gap-4" @submit.prevent="save('add')"><label class="flex flex-col gap-2">{{ t(m.name) }}<Input v-model="serverName" :maxlength="64" /></label><label class="flex flex-col gap-2">{{ t(m.address) }}<Input v-model="serverAddress" placeholder="play.example.org:25565" :maxlength="255" /></label></form><template #actions><Button @click="addModal?.hide()">{{ t(m.cancel) }}</Button><Button type="colored" color="brand" :disabled="!serverName.trim() || !serverAddress.trim() || saving || running" @click="save('add')">{{ t(m.add) }}</Button></template></NewModal>
  <NewModal ref="removeModal" :header="t(m.remove)" max-width="480px"><p>{{ removing?.name }}</p><template #actions><Button @click="removeModal?.hide()">{{ t(m.cancel) }}</Button><Button type="colored" color="red" :disabled="saving || running" @click="save('remove')">{{ t(m.remove) }}</Button></template></NewModal>
 </div>
</template>
