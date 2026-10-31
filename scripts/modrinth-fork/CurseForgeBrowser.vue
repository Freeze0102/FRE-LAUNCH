<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { defineMessages, useVIntl } from '@modrinth/ui'
import { computed, onMounted, ref, watch } from 'vue'
import { list as listInstances } from '@/helpers/instance'

type GameChoice = { id: string; name: string; game_version: string; loader: string }
type CurseForgeMod = {
  id: number
  name: string
  summary: string
  slug: string
  logo: { thumbnailUrl?: string } | null
  allowModDistribution?: boolean
}

const props = defineProps<{ instanceId?: string }>()
const { formatMessage: t } = useVIntl()
const m = defineMessages({
  intro: { id: 'fre-launch.cf.intro', defaultMessage: 'Mods from CurseForge can be installed into the same game instance as Modrinth mods.' },
  apiKey: { id: 'fre-launch.cf.api-key', defaultMessage: 'Approved CurseForge API key (not saved)' },
  keyHelp: { id: 'fre-launch.cf.key-help', defaultMessage: 'CurseForge requires an approved third-party API key. Without one you can open projects in the browser. The key stays in this session only.' },
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
  browseLink: { id: 'fre-launch.cf.browse-link', defaultMessage: 'Browse on CurseForge without an API key' },
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

const selected = computed(() => instances.value.find((i) => i.id === instanceId.value))
const gameVersion = computed(() => selected.value?.game_version ?? '')
const loader = computed(() => selected.value?.loader ?? '')
const isModded = computed(() => ['fabric', 'forge', 'neoforge', 'quilt'].includes(loader.value.toLowerCase()))

watch(() => props.instanceId, (value) => {
  if (value) instanceId.value = value
})
watch(instanceId, () => {
  results.value = []
  searched.value = false
  error.value = ''
  notice.value = ''
})

onMounted(async () => {
  try {
    const all = await listInstances()
    instances.value = all
      .filter((i) => ['fabric', 'forge', 'neoforge', 'quilt'].includes(String(i.loader).toLowerCase()))
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
  loading.value = true
  error.value = ''
  notice.value = ''
  searched.value = false
  try {
    results.value = await invoke<CurseForgeMod[]>('plugin:fre-curseforge|cf_search', {
      query: searchText.value.trim(),
      gameVersion: isModded.value ? gameVersion.value : null,
      modLoader: isModded.value ? loader.value : null,
      apiKey: apiKey.value.trim() || null,
    })
    searched.value = true
  } catch (err) {
    results.value = []
    error.value = t(m.searchError, { reason: reason(err) })
  } finally {
    loading.value = false
  }
}

async function install(mod: CurseForgeMod) {
  if (!selected.value || !isModded.value || installing.value !== null) return
  installing.value = mod.id
  error.value = ''
  notice.value = ''
  try {
    await invoke<string>('plugin:fre-curseforge|cf_install', {
      modId: mod.id,
      instanceId: instanceId.value,
      apiKey: apiKey.value.trim() || null,
    })
    notice.value = t(m.success, { name: mod.name })
  } catch (err) {
    error.value = t(m.installError, { reason: reason(err) })
  } finally {
    installing.value = null
  }
}

function openProject(mod: CurseForgeMod) {
  if (!/^[a-z0-9-]+$/.test(mod.slug)) return
  void openUrl('https://www.curseforge.com/minecraft/mc-mods/' + mod.slug)
}
function browseWithoutKey() {
  void openUrl('https://www.curseforge.com/minecraft/search?search=' + encodeURIComponent(searchText.value.trim()))
}
function openKeyHelp() {
  void openUrl('https://support.curseforge.com/support/solutions/articles/9000208346-about-the-curseforge-api-and-how-to-apply-for-a-key')
}
</script>

<template>
  <section class="fre-cf space-y-4">
    <div class="rounded-xl border border-solid border-surface-4 bg-bg-raised p-4">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 class="m-0 text-xl font-semibold text-contrast">CurseForge</h2>
          <p class="mt-2 mb-0 text-sm text-secondary">{{ t(m.intro) }}</p>
        </div>
        <span class="rounded-lg border border-solid border-surface-5 px-3 py-1 text-xs text-secondary">CurseForge API</span>
      </div>

      <div class="mt-4 grid gap-3 md:grid-cols-2">
        <label class="flex flex-col gap-2 text-sm text-secondary">
          {{ t(m.instance) }}
          <select v-model="instanceId" class="min-h-10 w-full rounded-lg bg-button-bg p-2 text-contrast">
            <option value="">{{ t(m.selectInstance) }}</option>
            <option v-for="i in instances" :key="i.id" :value="i.id">
              {{ i.name }} · {{ i.game_version }} · {{ i.loader }}
            </option>
          </select>
        </label>
        <label class="flex flex-col gap-2 text-sm text-secondary">
          {{ t(m.apiKey) }}
          <input
            v-model="apiKey"
            type="password"
            autocomplete="off"
            spellcheck="false"
            class="min-h-10 w-full rounded-lg bg-button-bg px-3 text-contrast"
            placeholder="x-api-key"
          />
        </label>
      </div>
      <p class="mt-3 mb-1 text-xs text-secondary">{{ t(m.keyHelp) }}</p>
      <button class="cursor-pointer border-0 bg-transparent p-0 text-sm text-brand hover:underline" @click="openKeyHelp">
        {{ t(m.helpLink) }}
      </button>

      <form class="mt-4 flex flex-wrap gap-2" @submit.prevent="doSearch">
        <input
          v-model="searchText"
          class="min-w-48 flex-1 rounded-lg bg-button-bg p-3 text-contrast"
          :placeholder="t(m.searchPlaceholder)"
          maxlength="100"
        />
        <button type="submit" class="rounded-lg border-0 bg-brand px-5 py-2 font-semibold text-white cursor-pointer disabled:opacity-50" :disabled="loading">
          {{ loading ? t(m.searching) : t(m.searchButton) }}
        </button>
      </form>
      <button class="mt-3 cursor-pointer border-0 bg-transparent p-0 text-sm text-brand hover:underline" @click="browseWithoutKey">
        {{ t(m.browseLink) }}
      </button>

      <p v-if="isModded" class="mt-3 mb-0 text-sm text-secondary">{{ t(m.compatible) }}</p>
      <p v-else class="mt-3 mb-0 text-sm text-secondary">{{ t(m.noInstance) }}</p>
    </div>

    <p v-if="error" role="alert" class="rounded-lg border border-solid border-red-500/50 p-3 text-sm text-red-400">{{ error }}</p>
    <p v-if="notice" role="status" class="rounded-lg border border-solid border-surface-4 p-3 text-sm text-brand">{{ notice }}</p>
    <p class="text-xs text-secondary">{{ t(m.warning) }}</p>

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
          <button class="rounded-lg border border-solid border-surface-5 bg-button-bg px-3 py-2 text-sm text-contrast cursor-pointer" @click="openProject(mod)">
            {{ t(m.openWebsite) }}
          </button>
          <button
            class="rounded-lg border-0 bg-brand px-3 py-2 text-sm font-semibold text-white cursor-pointer disabled:opacity-40"
            :disabled="!isModded || mod.allowModDistribution !== true || installing !== null"
            @click="install(mod)"
          >
            {{ installing === mod.id ? t(m.installing) : t(m.install) }}
          </button>
        </div>
      </article>
    </div>
  </section>
</template>
