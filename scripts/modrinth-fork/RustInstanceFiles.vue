<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import {
  defineMessages, FilePageLayout, injectNotificationManager,
  provideFileManager, useVIntl,
} from '@modrinth/ui'
import type { EditingFile, FileItem } from '@modrinth/ui'
import { computed, onMounted, ref, watch } from 'vue'

type NativeFile = { name: string; bytes: number; isDirectory: boolean }
type Folder = 'root' | 'mods' | 'resourcepacks' | 'shaderpacks' | 'saves' | 'logs' | 'screenshots'
const allowed: readonly string[] = ['root', 'mods', 'resourcepacks', 'shaderpacks', 'saves', 'logs', 'screenshots']
const props = defineProps<{
  kind: 'native' | 'horizon'
  profileId: string
  horizonExecutable: string
  initialFolder?: Folder
}>()
const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()
const messages = defineMessages({
  readOnly: {
    id: 'fre-rust.files.read-only',
    defaultMessage: 'Native Rust files are currently managed using your file explorer.',
  },
})
const items = ref<FileItem[]>([])
const loading = ref(false)
const error = ref<Error | null>(null)
const currentPath = ref(props.initialFolder && props.initialFolder !== 'root' ? props.initialFolder : '')
const editingFile = ref<EditingFile | null>(null)
const basePath = ref('')
let request = 0

function folderForPath(path: string) {
  const normalized = path.replace(/^\/+|\/+$/g, '')
  const folder = normalized || 'root'
  if (!allowed.includes(folder)) {
    throw new Error('Use Open folder to browse nested Rust folders securely.')
  }
  return folder as Folder
}

async function refresh() {
  const current = ++request
  loading.value = true
  error.value = null
  try {
    const folder = folderForPath(currentPath.value)
    const results = props.kind === 'native'
      ? await invoke<NativeFile[]>('plugin:fre-rust|rust_list_files', {
        profileId: props.profileId, folder,
      })
      : await invoke<NativeFile[]>('plugin:fre-horizon|horizon_list_files', {
        exePath: props.horizonExecutable, name: props.profileId, folder,
      })
    if (current !== request) return
    items.value = results.map(file => ({
      name: file.name,
      path: folder === 'root' ? file.name : folder + '/' + file.name,
      type: file.isDirectory ? 'directory' : 'file',
      size: file.isDirectory ? null : file.bytes,
      modified: 0,
      created: 0,
      readOnly: true,
    }))
  } catch (err) {
    if (current !== request) return
    error.value = err instanceof Error ? err : new Error(String(err))
    items.value = []
  } finally {
    if (current === request) loading.value = false
  }
}
function navigateTo(path: string) {
  try {
    const folder = folderForPath(path)
    currentPath.value = folder === 'root' ? '' : folder
    void refresh()
  } catch (err) {
    handleError(err instanceof Error ? err : new Error(String(err)))
  }
}
const readOnlyMessage = () => new Error(formatMessage(messages.readOnly))
const unsupported = async (): Promise<void> => { throw readOnlyMessage() }
function notifyUnsupported() { handleError(readOnlyMessage()) }
async function openInExplorer() {
  try {
    const folder = folderForPath(currentPath.value)
    if (props.kind === 'native') {
      await invoke('plugin:fre-rust|rust_open_folder', {
        profileId: props.profileId, folder,
      })
    } else {
      await invoke('plugin:fre-horizon|horizon_open_folder', {
        exePath: props.horizonExecutable, name: props.profileId, folder,
      })
    }
  } catch (err) {
    handleError(err instanceof Error ? err : new Error(String(err)))
  }
}
watch(() => [props.kind, props.profileId, props.horizonExecutable, props.initialFolder], () => {
  currentPath.value = props.initialFolder && props.initialFolder !== 'root' ? props.initialFolder : ''
  void refresh()
})
onMounted(() => { void refresh() })

// Share the REAL Modrinth Files tab component and its file manager contract.
// Writes intentionally remain blocked until the native backend supports safe
// atomic, symlink-resistant edits; Open folder remains a working alternative.
provideFileManager({
  items,
  loading,
  error,
  currentPath,
  navigateTo,
  editingFile,
  startEditing: () => notifyUnsupported(),
  stopEditing: () => { editingFile.value = null },
  createItem: unsupported,
  renameItem: unsupported,
  moveItem: unsupported,
  deleteItem: unsupported,
  readFile: async () => { throw readOnlyMessage() },
  readFileAsBlob: async () => { throw readOnlyMessage() },
  writeFile: unsupported,
  downloadFile: unsupported,
  uploadFiles: () => notifyUnsupported(),
  refresh: () => { void refresh() },
  isReadOnly: () => true,
  readOnlyReason: computed(() => formatMessage(messages.readOnly)),
  basePath,
  openInFolder: () => { void openInExplorer() },
})
</script>

<template>
  <div class="flex flex-col gap-4">
    <p class="m-0 text-sm text-secondary">{{ formatMessage(messages.readOnly) }}</p>
    <FilePageLayout :show-refresh-button="true" />
  </div>
</template>
