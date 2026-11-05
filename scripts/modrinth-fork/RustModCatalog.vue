<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { openUrl } from '@tauri-apps/plugin-opener'
import { Button, defineMessages, useVIntl } from '@modrinth/ui'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'

type File = { name: string; url: string; size: number; sha256: string; signature?: string }
type Version = { version: string; game: string; api: string; published: string; file: File }
type Project = { id: string; name: string; kind: string; summary: string; description: string; authors: string[]; license: string; source: string; icon?: string; categories: string[]; tags: string[]; downloads: number; versions: Version[] }
type Catalog = { projects: Project[]; generated: string; fetchedAt: number; stale: boolean; warning?: string }
type Check = { code: string; state: 'pass' | 'warning' | 'fail' | 'unavailable'; detail: string }
type Report = { projectId: string; version: string; filename: string; sha256: string; checkedAt: number; installable: boolean; checks: Check[]; facts?: { contentOnly: boolean; codeFiles: number; indicators: string[]; hooks?: number } }
const props = defineProps<{ profileId: string; installedFiles?: string[] }>()
const emit = defineEmits<{ installed: [] }>()
const { formatMessage: t } = useVIntl()
const m = defineMessages({
  title: { id: "fre-rust.catalog.title", defaultMessage: "Rust catalog" },
  hint: { id: "fre-rust.catalog.hint", defaultMessage: "Mods from the public registry, with authors, versions and source links." },
  automatic: { id: "fre-rust.catalog.automatic", defaultMessage: "Automatic checks before installation and launch are enabled." },
  limits: { id: "fre-rust.catalog.limits", defaultMessage: "Checks confirm the file and detect known problems. They do not guarantee that native code is safe." },
  search: { id: "fre-rust.catalog.search", defaultMessage: "Name, author or tag…" },
  all: { id: "fre-rust.catalog.all", defaultMessage: "All types" },
  mods: { id: "fre-rust.catalog.mods", defaultMessage: "Mods" },
  native: { id: "fre-rust.catalog.native", defaultMessage: "Native mods" },
  data: { id: "fre-rust.catalog.data", defaultMessage: "Mod data packages" },
  packs: { id: "fre-rust.catalog.packs", defaultMessage: "Packs and shaders" },
  refresh: { id: "fre-rust.catalog.refresh", defaultMessage: "Refresh catalog" },
  loading: { id: "fre-rust.catalog.loading", defaultMessage: "Loading catalog…" },
  empty: { id: "fre-rust.catalog.empty", defaultMessage: "No mods match this search." },
  verify: { id: "fre-rust.catalog.verify", defaultMessage: "Check file" },
  checking: { id: "fre-rust.catalog.checking", defaultMessage: "Checking…" },
  install: { id: "fre-rust.catalog.install", defaultMessage: "Install files" },
  installing: { id: "fre-rust.catalog.installing", defaultMessage: "Installing…" },
  installed: { id: "fre-rust.catalog.installed", defaultMessage: "Files installed" },
  report: { id: "fre-rust.catalog.report", defaultMessage: "Verification results" },
  details: { id: "fre-rust.catalog.details", defaultMessage: "Description and checks" },
  code: { id: "fre-rust.catalog.code", defaultMessage: "Source on GitHub" },
  needs_build: { id: "fre-rust.catalog.needs-build", defaultMessage: "Native code: a reviewed FRE build is required" },
  legacy: { id: "fre-rust.catalog.legacy", defaultMessage: "This game does not run the legacy code format." },
  ready: { id: "fre-rust.catalog.ready", defaultMessage: "File checks passed" },
  not_ready: { id: "fre-rust.catalog.not-ready", defaultMessage: "Installation is not available yet" },
  unverified: { id: "fre-rust.catalog.unverified", defaultMessage: "File not checked yet" },
  stale: { id: "fre-rust.catalog.stale", defaultMessage: "Showing a cached catalog. Installation requires a fresh connection." },
  version: { id: "fre-rust.catalog.version", defaultMessage: "Version" },
  license: { id: "fre-rust.catalog.license", defaultMessage: "License" },
  downloads: { id: "fre-rust.catalog.downloads", defaultMessage: "Downloads" },
  more: { id: "fre-rust.catalog.more", defaultMessage: "Show more" },
  incompatible: { id: "fre-rust.catalog.incompatible", defaultMessage: "No Minecraft 26.3 version" },
  origin: { id: "fre-rust.catalog.origin", defaultMessage: "Download origin" },
  revocation: { id: "fre-rust.catalog.revocation", defaultMessage: "Revoked releases" },
  hash: { id: "fre-rust.catalog.hash", defaultMessage: "Size and SHA-256" },
  signature: { id: "fre-rust.catalog.signature", defaultMessage: "Publisher signature" },
  archive: { id: "fre-rust.catalog.archive", defaultMessage: "Archive structure and limits" },
  antivirus: { id: "fre-rust.catalog.antivirus", defaultMessage: "Microsoft Defender" },
  execution: { id: "fre-rust.catalog.execution", defaultMessage: "Payload type and execution" },
  compatibility: { id: "fre-rust.catalog.compatibility", defaultMessage: "Format compatibility" },
  pass: { id: "fre-rust.catalog.pass", defaultMessage: "Passed" },
  warning: { id: "fre-rust.catalog.warning", defaultMessage: "Warning" },
  fail: { id: "fre-rust.catalog.fail", defaultMessage: "Failed" },
  unavailable: { id: "fre-rust.catalog.unavailable", defaultMessage: "Check incomplete" },
  indicators: { id: "fre-rust.catalog.indicators", defaultMessage: "Source operations requiring manual review:" },
  builtin: { id: "fre-rust.catalog.builtin", defaultMessage: "A FRE implementation is already available under Ready to play." },
  saved: { id: "fre-rust.catalog.saved", defaultMessage: "Files installed. Companion packages may need a native mod; see the description. Enable a compatible package in the game Mods settings." },
  changed: { id: "fre-rust.catalog.changed", defaultMessage: "The version changed. Refresh the catalog before continuing." },
  pack_note: { id: "fre-rust.catalog.pack-note", defaultMessage: "This pack type is listed for browsing; installation here is not supported." },
})
const projects = ref<Project[]>([])
const query = ref('')
const kind = ref('mods')
const loading = ref(false)
const pending = ref('')
const installing = ref(false)
const error = ref('')
const notice = ref('')
const stale = ref(false)
const expanded = ref('')
const visibleCount = ref(40)
const reports = ref<Record<string, Report>>({})
let generation = 0
function version(project: Project) { return project.versions.find(v => v.game === '26.3') }
function key(project: Project) { return project.id + ':' + (version(project)?.file.sha256 || '') }
function report(project: Project) { return reports.value[key(project)] }
function isInstalled(project: Project) {
  const installed = new Set((props.installedFiles || []).map(file => file.toLowerCase()))
  return project.versions.some(v => installed.has(v.file.name.toLowerCase()))
}
function safeSource(value: string) {
  try {
    const url = new URL(value)
    return url.protocol === 'https:' && url.hostname === 'github.com' && !url.username && !url.password && (!url.port || url.port === '443') ? url.href : null
  } catch { return null }
}
function image(value?: string) {
  if (!value) return undefined
  try {
    const url = new URL(value)
    return url.protocol === 'https:' && !url.username && !url.password && (!url.port || url.port === '443') && ['horizon-rust.github.io', 'avatars.githubusercontent.com', 'raw.githubusercontent.com'].includes(url.hostname) ? url.href : undefined
  } catch { return undefined }
}
const filtered = computed(() => {
  const words = query.value.toLowerCase().trim().split(/\s+/).filter(Boolean)
  return projects.value.filter(project => {
    if (kind.value === 'mods' && !['mod', 'native-mod'].includes(project.kind)) return false
    if (kind.value === 'native' && project.kind !== 'native-mod') return false
    if (kind.value === 'data' && project.kind !== 'mod') return false
    if (kind.value === 'packs' && !['resourcepack', 'shader'].includes(project.kind)) return false
    const text = [project.name, project.id, project.summary, ...project.authors, ...project.categories, ...project.tags].join(' ').toLowerCase()
    return words.every(word => text.includes(word))
  })
})
const shown = computed(() => filtered.value.slice(0, visibleCount.value))
const labels = { origin: m.origin, revocation: m.revocation, hash: m.hash, signature: m.signature, archive: m.archive, antivirus: m.antivirus, execution: m.execution, compatibility: m.compatibility }
function checkLabel(check: Check) { return check.code in labels ? t(labels[check.code as keyof typeof labels]) : check.code }
function stateLabel(check: Check) { return t(m[check.state]) }
async function load(force = false) {
  const request = ++generation
  const profileId = props.profileId
  loading.value = true
  error.value = ''
  try {
    const result = await invoke<Catalog>('plugin:fre-rust|rust_catalog', { profileId, force })
    if (request !== generation || props.profileId !== profileId) return
    projects.value = result.projects
    stale.value = result.stale
    visibleCount.value = 40
  } catch (e) { if (request === generation) error.value = String(e) }
  finally { if (request === generation) loading.value = false }
}
async function verify(project: Project) {
  const selected = version(project)
  if (!selected || pending.value || loading.value) return
  const request = generation
  const profileId = props.profileId
  pending.value = project.id
  error.value = ''
  expanded.value = project.id
  try {
    const result = await invoke<Report>('plugin:fre-rust|rust_catalog_check', { profileId, projectId: project.id, expectedSha256: selected.file.sha256 })
    if (request !== generation || profileId !== props.profileId) return
    if (result.sha256 !== selected.file.sha256) throw new Error(t(m.changed))
    reports.value = { ...reports.value, [key(project)]: result }
  } catch (e) { if (request === generation) error.value = String(e) }
  finally { if (request === generation) pending.value = '' }
}
async function install(project: Project) {
  const selected = version(project)
  if (!selected || project.kind !== 'mod' || pending.value || loading.value || stale.value || !report(project)?.installable || isInstalled(project)) return
  const request = generation
  const profileId = props.profileId
  pending.value = project.id
  installing.value = true
  error.value = ''
  notice.value = ''
  try {
    const result = await invoke<Report>('plugin:fre-rust|rust_catalog_install', { profileId, projectId: project.id, expectedSha256: selected.file.sha256 })
    if (request !== generation || profileId !== props.profileId) return
    reports.value = { ...reports.value, [key(project)]: result }
    notice.value = t(m.saved)
    emit('installed')
  } catch (e) { if (request === generation) error.value = String(e) }
  finally { if (request === generation) { pending.value = ''; installing.value = false } }
}
async function details(project: Project) {
  expanded.value = expanded.value === project.id ? '' : project.id
  if (expanded.value && !report(project) && ['mod', 'native-mod'].includes(project.kind)) await verify(project)
}
async function source(project: Project) {
  const url = safeSource(project.source)
  if (url) await openUrl(url)
}
onMounted(() => { void load() })
onUnmounted(() => { generation++ })
watch(() => props.profileId, () => {
  generation++
  projects.value = []
  reports.value = {}
  pending.value = ''
  expanded.value = ''
  notice.value = ''
  installing.value = false
  void load()
})
watch([query, kind], () => { visibleCount.value = 40 })
</script>

<template>
  <section class="flex flex-col gap-4 border-t border-solid border-surface-4 pt-5">
    <div>
      <h3 class="m-0 text-lg font-semibold text-contrast">{{ t(m.title) }}</h3>
      <p class="mb-0 text-sm text-secondary">{{ t(m.hint) }}</p>
    </div>
    <div class="rounded-xl bg-surface-3 p-3 text-sm">
      <p class="m-0 font-semibold text-brand">{{ t(m.automatic) }}</p>
      <p class="mb-0 mt-1 text-secondary">{{ t(m.limits) }}</p>
    </div>
    <div class="flex flex-wrap items-center gap-2">
      <input v-model="query" type="search" :placeholder="t(m.search)" :aria-label="t(m.search)" class="min-w-0 flex-1 rounded-xl border border-solid border-surface-4 bg-surface-3 p-3 text-contrast" />
      <select v-model="kind" class="rounded-xl border border-solid border-surface-4 bg-surface-3 p-3 text-contrast" :aria-label="t(m.all)">
        <option value="mods">{{ t(m.mods) }}</option><option value="native">{{ t(m.native) }}</option>
        <option value="data">{{ t(m.data) }}</option><option value="packs">{{ t(m.packs) }}</option><option value="all">{{ t(m.all) }}</option>
      </select>
      <Button type="outlined" :disabled="loading || !!pending" @click="load(true)">{{ t(m.refresh) }}</Button>
    </div>
    <p v-if="error" role="alert" class="m-0 text-sm text-red-400">{{ error }}</p>
    <p v-if="notice" role="status" class="m-0 text-sm text-brand">{{ notice }}</p>
    <p v-if="stale" role="status" class="m-0 text-sm text-secondary">{{ t(m.stale) }}</p>
    <p v-if="loading" class="m-0 text-sm text-secondary">{{ t(m.loading) }}</p>
    <p v-else-if="!filtered.length" class="m-0 text-sm text-secondary">{{ t(m.empty) }}</p>
    <p v-else class="m-0 text-xs text-secondary">{{ filtered.length }} / {{ projects.length }}</p>
    <article v-for="project in shown" :key="project.id" class="rounded-xl border border-solid border-surface-4 bg-surface-2 p-4">
      <div class="flex items-start gap-3">
        <div class="flex h-12 w-12 shrink-0 items-center justify-center overflow-hidden rounded-xl bg-surface-4 font-bold text-contrast">
          <img v-if="image(project.icon)" :src="image(project.icon)" :alt="project.name" loading="lazy" referrerpolicy="no-referrer" class="h-full w-full object-cover" />
          <span v-else>{{ project.name.slice(0, 2).toUpperCase() }}</span>
        </div>
        <div class="min-w-0 flex-1">
          <h4 class="m-0 text-base font-bold text-contrast">{{ project.name }}</h4>
          <p class="mb-1 mt-1 text-xs text-secondary">{{ project.authors.join(', ') }} · {{ t(m.license) }}: {{ project.license }}</p>
          <p class="m-0 text-sm text-secondary">{{ project.summary }}</p>
          <div class="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-xs text-secondary">
            <span>{{ t(m.version) }}: {{ version(project)?.version || '—' }}</span>
            <span>{{ t(m.downloads) }}: {{ project.downloads.toLocaleString() }}</span>
            <span>{{ project.categories.join(' · ') }}</span>
          </div>
          <p v-if="project.kind === 'native-mod'" class="mb-0 mt-2 text-xs text-secondary">{{ t(m.needs_build) }}</p>
          <p v-if="['zoom','fps_overlay','coords_hud'].includes(project.id)" class="mb-0 mt-1 text-xs text-brand">{{ t(m.builtin) }}</p>
        </div>
      </div>
      <div class="mt-3 flex flex-wrap items-center gap-2">
        <Button type="outlined" :disabled="loading || !!pending" @click="details(project)">{{ t(m.details) }}</Button>
        <Button v-if="['mod','native-mod'].includes(project.kind)" type="outlined" :disabled="loading || !!pending || !version(project)" @click="verify(project)">
          {{ pending === project.id && !installing ? t(m.checking) : t(m.verify) }}
        </Button>
        <Button v-if="project.kind === 'mod'" :disabled="loading || !!pending || stale || !report(project)?.installable || isInstalled(project)" @click="install(project)">
          {{ isInstalled(project) ? t(m.installed) : pending === project.id && installing ? t(m.installing) : t(m.install) }}
        </Button>
        <Button v-if="safeSource(project.source)" type="outlined" @click="source(project)">{{ t(m.code) }}</Button>
        <span v-if="report(project)" class="text-xs" :class="report(project)?.installable ? 'text-brand' : 'text-secondary'">{{ report(project)?.installable ? t(m.ready) : t(m.not_ready) }}</span>
        <span v-else class="text-xs text-secondary">{{ t(m.unverified) }}</span>
      </div>
      <div v-if="expanded === project.id" class="mt-4 flex flex-col gap-3 border-t border-solid border-surface-4 pt-3">
        <p class="m-0 max-h-64 overflow-y-auto whitespace-pre-line text-sm text-secondary">{{ project.description }}</p>
        <p v-if="!['mod','native-mod'].includes(project.kind)" class="m-0 text-sm text-secondary">{{ t(m.pack_note) }}</p>
        <template v-if="report(project)">
          <h5 class="m-0 text-sm font-bold text-contrast">{{ t(m.report) }}</h5>
          <div v-for="item in report(project)?.checks" :key="item.code" class="flex flex-col gap-1 rounded-lg bg-surface-3 p-3 text-sm">
            <div class="flex justify-between gap-3"><span>{{ checkLabel(item) }}</span><span :class="item.state === 'pass' ? 'text-brand' : item.state === 'fail' ? 'text-red-400' : 'text-secondary'">{{ stateLabel(item) }}</span></div>
            <span class="break-words text-xs text-secondary">{{ item.detail }}</span>
          </div>
          <p v-if="report(project)?.facts?.indicators.length" class="m-0 text-sm text-secondary">{{ t(m.indicators) }} {{ report(project)?.facts?.indicators.join(', ') }}</p>
        </template>
      </div>
    </article>
    <Button v-if="filtered.length > visibleCount" type="outlined" :disabled="loading || !!pending" @click="visibleCount += 40">{{ t(m.more) }}</Button>
  </section>
</template>
