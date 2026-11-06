<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { ConsolePageLayout, provideConsoleManager, injectNotificationManager } from '@modrinth/ui'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
const props = defineProps<{ profileId: string; running?: boolean }>()
const { handleError } = injectNotificationManager()
type Line = { text: string; level: 'error' | 'warn' | 'info' | 'debug' | 'trace' | null }
const logLines = ref<Line[]>([])
const names = ref<string[]>([])
const selected = ref(0)
const loading = ref(false)
const sources = computed(() => [{ id: 'latest.log', name: 'Live Log', live: true }, ...names.value.filter(name => name !== 'latest.log').map(name => ({ id: name, name, live: false }))])
const activeName = computed(() => sources.value[selected.value]?.id ?? 'latest.log')
let generation = 0, disposed = false, fetching = false, cleared = '', latestRaw = '', lastError = ''
let timer: ReturnType<typeof setInterval> | undefined
function parse(text: string): Line[] {
  return text.split(/\r?\n/).filter(Boolean).slice(-12000).map(text => {
    const match = text.match(/(?:\/|\b)(ERROR|FATAL|WARN|INFO|DEBUG|TRACE)(?:\]|\b)/i)
    const level = match?.[1].toLowerCase()
    return { text, level: level === 'fatal' ? 'error' : (level as Line['level'] ?? null) }
  })
}
async function refresh(list = false) {
  if (fetching || disposed) return
  const request = generation, profileId = props.profileId, filename = activeName.value
  fetching = true
  try {
    if (list) { const result = await invoke<string[]>('plugin:fre-rust|rust_log_sources', { profileId }); if (request === generation && !disposed) names.value = result }
    const text = await invoke<string>('plugin:fre-rust|rust_logs', { profileId, filename })
    if (disposed || request !== generation || profileId !== props.profileId || filename !== activeName.value) return
    latestRaw = text
    logLines.value = parse(cleared && text.startsWith(cleared) ? text.slice(cleared.length) : text)
    lastError = ''
  } catch (error) {
    if (request === generation && !disposed && String(error) !== lastError) { lastError = String(error); handleError(error instanceof Error ? error : new Error(String(error))) }
  } finally { fetching = false; if (request === generation) loading.value = false }
}
provideConsoleManager({
  logLines, logSources: sources, activeLogSourceIndex: selected, loading,
  showCommandInput: false, shareDisabled: computed(() => true),
  deleteDisabled: computed(() => selected.value === 0 && !!props.running),
  onClear: () => { cleared = latestRaw; logLines.value = [] },
  emptyStateType: 'instance',
})
watch(() => [props.profileId, activeName.value], () => { generation++; cleared = ''; logLines.value = []; loading.value = true; void refresh(true) })
onMounted(() => { void refresh(true); timer = setInterval(() => { if (selected.value === 0) void refresh(true) }, 1000) })
onUnmounted(() => { disposed = true; generation++; if (timer) clearInterval(timer) })
</script>
<template><div class="flex h-full min-h-0 flex-col gap-4"><ConsolePageLayout /></div></template>
