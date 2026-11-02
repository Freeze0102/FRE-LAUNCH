<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { list as listInstances } from '@/helpers/instance'

type GameChoice = { id: string; name: string; game_version: string; loader: string }
type CurseForgeMod = {
  id: number
  name: string
  summary: string
  slug: string
  logo: { thumbnailUrl?: string } | null
  allowModDistribution?: boolean
  isAvailable?: boolean
}

const props = defineProps<{ instanceId?: string; contentType?: 'mod' | 'modpack' }>()
const { formatMessage: t } = useVIntl()
const m = defineMessages({
  intro: { id: 'fre-launch.cf.intro', defaultMessage: 'Search CurseForge directly in FRE LAUNCH using an approved API key, and install permitted mods into your Java instance.' },
  packIntro: { id: 'fre-launch.cf.pack-intro', defaultMessage: 'Search CurseForge modpacks inside FRE LAUNCH. Installing ZIP modpacks from CurseForge is not yet supported.' },
  packSearch: { id: 'fre-launch.cf.pack-search', defaultMessage: 'Find modpacks' },
  apiKey: { id: 'fre-launch.cf.api-key', defaultMessage: 'Approved CurseForge API key (not saved)' },
  keyHelp: { id: 'fre-launch.cf.key-help', defaultMessage: 'Website search needs no key. Only enable API mode if you have approved access; the key is not saved.' },
  directIntro: { id: 'fre-launch.cf.direct-intro', defaultMessage: 'Search on the official CurseForge website without an API key. Downloads are handled by your browser.' },
  modeApi: { id: 'fre-launch.cf.mode-api', defaultMessage: 'Use in-app API search (advanced, requires approved access)' },
  siteSearch: { id: 'fre-launch.cf.site-search', defaultMessage: 'Search CurseForge website' },
  siteOpened: { id: 'fre-launch.cf.site-opened', defaultMessage: 'Opened search results in your browser. Download a compatible file from CurseForge, then import the JAR here.' },
  packOpened: { id: 'fre-launch.cf.pack-opened', defaultMessage: 'Opened CurseForge modpacks in your browser. Importing full modpacks is not supported in this panel.' },
  importFile: { id: 'fre-launch.cf.import-file', defaultMessage: 'Import downloaded JAR to this instance' },
  importingFile: { id: 'fre-launch.cf.importing-file', defaultMessage: 'Importing file...' },
  importedFile: { id: 'fre-launch.cf.imported-file', defaultMessage: 'JAR imported. Check the Minecraft version, loader and dependencies before playing.' },
  importFileError: { id: 'fre-launch.cf.import-file-error', defaultMessage: 'Cannot import downloaded JAR: {reason}' },
  importFileHelp: { id: 'fre-launch.cf.import-file-help', defaultMessage: 'Download the correct JAR on CurseForge, then import it to the selected Java instance. This does not check dependencies or mod compatibility.' },
  helpLink: { id: 'fre-launch.cf.key-link', defaultMessage: 'How to request access' },
  instance: { id: 'fre-launch.cf.instance', defaultMessage: 'Install to game instance' },
  selectInstance: { id: 'fre-launch.cf.select-instance', defaultMessage: 'Select a game instance' },
  search: { id: 'fre-launch.cf.search', defaultMessage: 'Search CurseForge' },
  searchPlaceholder: { id: 'fre-launch.cf.placeholder', defaultMessage: 'Find a Minecraft mod...' },
  searchButton: { id: 'fre-launch.cf.search-button', defaultMessage: 'Find mods' },
  searching: { id: 'fre-launch.cf.searching', defaultMessage: 'Searching...' },
  compatible: { id: 'fre-launch.cf.compatible', defaultMessage: 'Only compatible files will be installed for the selected Minecraft version and mod loader.' },
  noInstance: { id: 'fre-launch.cf.no-instance', defaultMessage: 'Create a Fabric, Forge, NeoForge or Quilt instance to install mods.' },
  results: { id: 'fre-launch.cf.results', defaultMessage: 'Results' },
  noResults: { id: 'fre-launch.cf.no-results', defaultMessage: 'No mods found. Try another search or version.' },
  searchError: { id: 'fre-launch.cf.search-error', defaultMessage: 'CurseForge search is unavailable: {reason}' },
  install: { id: 'fre-launch.cf.install', defaultMessage: 'Install to instance' },
  installing: { id: 'fre-launch.cf.installing', defaultMessage: 'Installing...' },
  success: { id: 'fre-launch.cf.success', defaultMessage: 'Installed {name} into the selected instance. Check required dependencies before playing.' },
  installError: { id: 'fre-launch.cf.install-error', defaultMessage: 'Install failed: {reason}' },
  openWebsite: { id: 'fre-launch.cf.website', defaultMessage: 'Open on CurseForge' },
  distribution: { id: 'fre-launch.cf.distribution', defaultMessage: 'Author has disabled third-party distribution' },
  warning: { id: 'fre-launch.cf.warning', defaultMessage: 'Different sources can share one mods folder, but dependencies and mod conflicts are not automatically resolved. Back up your instance first.' },
  browseLink: { id: 'fre-launch.cf.browse-link', defaultMessage: 'Open CurseForge catalog' },
})

const instances = ref<GameChoice[]>([])
const instanceId = ref(props.instanceId ?? '')
const searchText = ref('')
const apiKey = ref('')
const results = ref<CurseForgeMod[]>([])
const searched = ref(false)
const loading = ref(false)
const installing = ref<number | null>(null)
const notice = ref('')
const error = ref('')
let searchSequence = 0

const isModpack = computed(() => props.contentType === 'modpack')
const selected = computed(() => instances.value.find((i) => i.id === instanceId.value))
const gameVersion = computed(() => selected.value?.game_version ?? '')
const loader = computed(() => selected.value?.loader ?? '')
const isModded = computed(() => ['fabric', 'forge', 'neoforge', 'quilt'].includes(loader.value.toLowerCase()))

watch(() => props.instanceId, (value) => {
  if (value) instanceId.value = value
})
watch([instanceId, searchText, apiKey], () => {
  searchSequence++
  loading.value = false
  results.value = []
  searched.value = false
  error.value = ''
  notice.value = ''
}, { flush: 'sync' })
onUnmounted(() => { searchSequence++ })

onMounted(async () => {
  try {
    if (isModpack.value) return
    const all = await listInstances()
    instances.value = all
      .filter((i) => !i.quarantined && !i.shared_instance && ['fabric', 'forge', 'neoforge', 'quilt'].includes(String(i.loader).toLowerCase()))
      .map((i) => ({ id: i.id, name: i.name, game_version: i.game_version, loader: String(i.loader) }))
    if (!instances.value.some((i) => i.id === instanceId.value)) {
      instanceId.value = instances.value[0]?.id ?? ''
    }
  } catch (err) {
    error.value = String(err)
  }
})

function reason(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

async function doSearch() {
  const sequence = ++searchSequence
  loading.value = true
  error.value = ''
  notice.value = ''
  searched.value = false
  try {
    const found = await invoke<CurseForgeMod[]>('plugin:fre-curseforge|cf_search', {
      query: searchText.value.trim(),
      gameVersion: !isModpack.value && isModded.value ? gameVersion.value : null,
      modLoader: !isModpack.value && isModded.value ? loader.value : null,
      contentType: isModpack.value ? 'modpack' : 'mod',
      apiKey: apiKey.value.trim() || null,
    })
    if (sequence !== searchSequence) return
    results.value = found
    searched.value = true
  } catch (err) {
    if (sequence !== searchSequence) return
    results.value = []
    error.value = t(m.searchError, { reason: reason(err) })
  } finally {
    if (sequence === searchSequence) loading.value = false
  }
}

async function install(mod: CurseForgeMod) {
  if (isModpack.value || !selected.value || !isModded.value || installing.value !== null
    || mod.isAvailable === false || mod.allowModDistribution !== true) return
  const target = selected.value
  installing.value = mod.id
  error.value = ''
  notice.value = ''
  try {
    await invoke<string>('plugin:fre-curseforge|cf_install', {
      modId: mod.id,
      instanceId: target.id,
      apiKey: apiKey.value.trim() || null,
    })
    notice.value = `${target.name}: ${t(m.success, { name: mod.name })}`
  } catch (err) {
    error.value = t(m.installError, { reason: reason(err) })
  } finally {
    installing.value = null
  }
}

</script>

<template>
  <section class="fre-cf space-y-4">
    <div class="rounded-xl border border-solid border-surface-4 bg-bg-raised p-4">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 class="m-0 text-xl font-semibold text-contrast">CurseForge</h2>
          <p class="mt-2 mb-0 text-sm text-secondary">{{ t(isModpack ? m.packIntro : m.intro) }}</p>
        </div>
        <span class="rounded-lg border border-solid border-surface-5 px-3 py-1 text-xs text-secondary">CurseForge API</span>
      </div>

      <label v-if="!isModpack" class="mt-4 flex flex-col gap-2 text-sm text-secondary">
        {{ t(m.instance) }}
        <select v-model="instanceId" :disabled="installing !== null" class="min-h-10 w-full rounded-lg bg-button-bg p-2 text-contrast">
          <option value="">{{ t(m.selectInstance) }}</option>
          <option v-for="i in instances" :key="i.id" :value="i.id">
            {{ i.name }} · {{ i.game_version }} · {{ i.loader }}
          </option>
        </select>
      </label>
      <details class="mt-3 rounded-lg border border-solid border-surface-5 p-3">
        <summary class="cursor-pointer text-sm text-secondary">Налаштування доступу CurseForge</summary>
        <label class="mt-3 flex flex-col gap-2 text-sm text-secondary">
          {{ t(m.apiKey) }}
          <input v-model="apiKey" type="password" autocomplete="off" spellcheck="false"
            class="min-h-10 w-full rounded-lg bg-button-bg px-3 text-contrast" placeholder="x-api-key" />
        </label>
        <p class="mt-3 mb-1 text-xs text-secondary">
          CurseForge вимагає схвалений API-ключ для сторонніх застосунків.
          Якщо ключ задано через FRE_CURSEFORGE_API_KEY, поле можна залишити порожнім.
          Без доступу використовуй каталог Modrinth поруч — він працює прямо в лаунчері.
        </p>
      </details>

      <form class="mt-4 flex flex-wrap gap-2" @submit.prevent="doSearch">
        <input
          v-model="searchText"
          class="min-w-48 flex-1 rounded-lg bg-button-bg p-3 text-contrast"
          :placeholder="t(m.searchPlaceholder)"
          maxlength="100"
        />
        <button type="submit" class="rounded-lg border-0 bg-brand px-5 py-2 font-semibold text-white cursor-pointer disabled:opacity-50" :disabled="loading">
          {{ loading ? t(m.searching) : t(isModpack ? m.packSearch : m.searchButton) }}
        </button>
      </form>
      <p v-if="isModpack" class="mt-3 mb-0 text-sm text-secondary">{{ t(m.packIntro) }}</p>
      <p v-else-if="isModded" class="mt-3 mb-0 text-sm text-secondary">{{ t(m.compatible) }}</p>
      <p v-else class="mt-3 mb-0 text-sm text-secondary">{{ t(m.noInstance) }}</p>
    </div>

    <p v-if="error" role="alert" class="rounded-lg border border-solid border-red-500/50 p-3 text-sm text-red-400">{{ error }}</p>
    <p v-if="notice" role="status" class="rounded-lg border border-solid border-surface-4 p-3 text-sm text-brand">{{ notice }}</p>
    <p v-if="!isModpack" class="text-xs text-secondary">{{ t(m.warning) }}</p>

    <div v-if="searched" class="space-y-3">
      <h3 class="m-0 font-semibold text-contrast">{{ t(m.results) }} · {{ results.length }}</h3>
      <p v-if="!results.length" class="text-secondary">{{ t(m.noResults) }}</p>
      <article
        v-for="mod in results"
        :key="mod.id"
        class="flex flex-wrap items-center gap-4 rounded-xl border border-solid border-surface-4 bg-bg-raised p-4"
      >
        <img
          v-if="mod.logo?.thumbnailUrl"
          :src="mod.logo.thumbnailUrl"
          :alt="mod.name"
          class="size-14 shrink-0 rounded-lg object-cover"
        />
        <div class="min-w-48 flex-1">
          <h4 class="m-0 font-semibold text-contrast">{{ mod.name }}</h4>
          <p class="mt-1 mb-0 text-sm text-secondary">{{ mod.summary }}</p>
          <p v-if="mod.allowModDistribution === false" class="mt-1 mb-0 text-xs text-secondary">{{ t(m.distribution) }}</p>
        </div>
        <div class="flex flex-wrap gap-2">
          <button
            v-if="!isModpack"
            class="rounded-lg border-0 bg-brand px-3 py-2 text-sm font-semibold text-white cursor-pointer disabled:opacity-40"
            :disabled="!isModded || mod.isAvailable === false || mod.allowModDistribution !== true || installing !== null"
            @click="install(mod)"
          >
            {{ installing === mod.id ? t(m.installing) : t(m.install) }}
          </button>
        </div>
      </article>
    </div>
  </section>
</template>
