<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { openUrl } from '@tauri-apps/plugin-opener'
import { CheckIcon, DownloadIcon, GridIcon, LeftArrowIcon, ListIcon, RefreshCwIcon, SearchIcon, ShieldCheckIcon, SpinnerIcon } from '@modrinth/assets'
import { Button, Combobox, defineMessages, IconButton, Input, NavTabs, Pagination, ProjectCard, ProjectCardList, useVIntl } from '@modrinth/ui'
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'

type File = { name: string; url: string; size: number; sha256: string; signature?: string }
type Version = { version: string; game: string; api: string; published: string; file: File }
type Project = { id: string; name: string; kind: string; summary: string; description: string; authors: string[]; license: string; source: string; icon?: string; categories: string[]; tags: string[]; downloads: number; versions: Version[] }
type Catalog = { projects: Project[]; generated: string; fetchedAt: number; stale: boolean; warning?: string }
type Check = { code: string; state: 'pass' | 'warning' | 'fail' | 'unavailable'; detail: string }
type Report = { projectId: string; version: string; filename: string; sha256: string; checkedAt: number; installable: boolean; checks: Check[]; facts?: { contentOnly: boolean; codeFiles: number; indicators: string[]; hooks?: number } }
const props = defineProps<{ profileId: string; installedFiles?: string[]; disabled?: boolean }>()
const emit = defineEmits<{ installed: []; close: []; builtin: [key: 'zoom' | 'fpsCounter' | 'coordinates'] }>()
const { formatMessage: t } = useVIntl()
const m = defineMessages({
  buildProgress: { id: 'fre-rust.catalog.build-progress', defaultMessage: 'Building your modded game:' },
  compiler: { id: 'fre-rust.catalog.compiler', defaultMessage: 'Preparing the private Rust compiler…' },
  dependencies: { id: 'fre-rust.catalog.dependencies', defaultMessage: 'Downloading pinned build dependencies…' },
  applying: { id: 'fre-rust.catalog.applying', defaultMessage: 'Combining the selected mods…' },
  compiling: { id: 'fre-rust.catalog.compiling', defaultMessage: 'Compiling the game. The first build can take a while…' },
  scanning: { id: 'fre-rust.catalog.scanning', defaultMessage: 'Checking the compiled game with Defender…' },
  back: { id: 'fre-rust.catalog.back', defaultMessage: 'Back to installed mods' },
  backResults: { id: 'fre-rust.catalog.back-results', defaultMessage: 'Back to results' },
  sort: { id: 'fre-rust.catalog.sort', defaultMessage: 'Sort by' },
  popular: { id: 'fre-rust.catalog.popular', defaultMessage: 'Downloads' },
  alphabetical: { id: 'fre-rust.catalog.alphabetical', defaultMessage: 'Name' },
  view: { id: 'fre-rust.catalog.view', defaultMessage: 'View' },
  layout: { id: 'fre-rust.catalog.layout', defaultMessage: 'Change layout' },
  description: { id: 'fre-rust.catalog.description', defaultMessage: 'Description' },
  versions: { id: 'fre-rust.catalog.versions', defaultMessage: 'Versions' },
  inInstance: { id: 'fre-rust.catalog.in-instance', defaultMessage: 'In this instance' },
  requiresBuild: { id: 'fre-rust.catalog.requires-build', defaultMessage: 'FRE build required' },
  autoBrief: { id: 'fre-rust.catalog.auto-brief', defaultMessage: 'File checks are automatic when installing.' },
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
const page = ref(1), pageSize = ref(20), order = ref('downloads'), layout = ref<'list' | 'grid'>('list'), detailTab = ref(0)
const buildStage = ref('')
let unlistenBuild: (() => void) | undefined
let disposed = false
const justInstalled = ref<string[]>([])
const reports = ref<Record<string, Report>>({})
let generation = 0
function version(project: Project) { return project.versions.find(v => v.game === '26.3') }
function key(project: Project) { return project.id + ':' + (version(project)?.file.sha256 || '') }
function report(project: Project) { return reports.value[key(project)] }
function isInstalled(project: Project) {
  const installed = new Set((props.installedFiles || []).map(file => file.toLowerCase()))
  return justInstalled.value.includes(project.id) || project.versions.some(v => installed.has(v.file.name.toLowerCase()))
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
  }).sort((a,b) => order.value === 'name' ? a.name.localeCompare(b.name) : b.downloads - a.downloads || a.name.localeCompare(b.name))
})
const pageCount = computed(() => Math.max(1, Math.ceil(filtered.value.length / pageSize.value)))
const shown = computed(() => filtered.value.slice((page.value - 1) * pageSize.value, page.value * pageSize.value))
const selectedProject = computed(() => projects.value.find(project => project.id === expanded.value))
const detailTabs = computed(() => [{ label: t(m.description), href: 'description' }, { label: t(m.versions), href: 'versions' }, { label: t(m.report), href: 'checks' }])
function switchPage(value: number) { if (Number.isInteger(value) && value >= 1 && value <= pageCount.value) page.value = value }
// The registry's old launcher/build paragraph conflicts with FRE's Play workflow.
// Keep the remaining author text, license, source URL and verified metadata intact.
function descriptionForFre(project: Project) {
  return project.description.split(/(?:\r?\n){2,}/)
    .filter(paragraph => !/^\s*(?:\*\*)?A native Horizon mod(?:\*\*)?\s*:/i.test(paragraph))
    .join('\n\n')
}
function builtinKey(project: Project) { return ({ zoom: 'zoom', fps_overlay: 'fpsCounter', coords_hud: 'coordinates' } as const)[project.id as 'zoom' | 'fps_overlay' | 'coords_hud'] }
function openBuiltin(project: Project) { const key = builtinKey(project); if (key) emit('builtin', key) }
async function selectDetailTab(index: number) {
  detailTab.value = index
  if (index === 2 && selectedProject.value && !report(selectedProject.value)) await verify(selectedProject.value)
}
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
    page.value = Math.min(page.value, pageCount.value)
  } catch (e) { if (request === generation) error.value = String(e) }
  finally { if (request === generation) loading.value = false }
}
async function verify(project: Project, showReport = true) {
  const selected = version(project)
  if (!selected || pending.value || loading.value) return
  const request = generation
  const profileId = props.profileId
  pending.value = project.id
  error.value = ''
  if (showReport) { expanded.value = project.id; detailTab.value = 2 }
  try {
    const result = await invoke<Report>('plugin:fre-rust|rust_catalog_check', { profileId, projectId: project.id, expectedSha256: selected.file.sha256 })
    if (request !== generation || profileId !== props.profileId) return
    if (result.sha256 !== selected.file.sha256) throw new Error(t(m.changed))
    reports.value = { ...reports.value, [key(project)]: result }
    return result
  } catch (e) { if (request === generation) error.value = String(e) }
  finally { if (request === generation) pending.value = '' }
}
async function install(project: Project) {
  const selected = version(project)
  buildStage.value = ''
  if (!selected || !['mod', 'native-mod'].includes(project.kind) || pending.value || loading.value || stale.value || props.disabled || isInstalled(project)) return
  const request = generation
  const profileId = props.profileId
  const checked = report(project)?.installable ? report(project) : await verify(project, false)
  if (request !== generation || profileId !== props.profileId || props.disabled) return
  if (!checked?.installable) { expanded.value = project.id; detailTab.value = 2; return }
  pending.value = project.id
  installing.value = true
  error.value = ''
  notice.value = ''
  try {
    const result = await invoke<Report>('plugin:fre-rust|rust_catalog_install', { profileId, projectId: project.id, expectedSha256: selected.file.sha256 })
    if (request !== generation || profileId !== props.profileId) return
    if (!result.installable) throw new Error(t(m.not_ready))
    reports.value = { ...reports.value, [key(project)]: result }
    justInstalled.value = [...justInstalled.value, project.id]
    notice.value = t(m.saved)
    emit('installed')
  } catch (e) { if (request === generation) error.value = String(e) }
  finally { if (request === generation) { pending.value = ''; installing.value = false; buildStage.value = '' } }
}
function details(project: Project) { expanded.value = project.id; detailTab.value = 0 }
async function source(project: Project) {
  const url = safeSource(project.source)
  if (url) await openUrl(url)
}
onMounted(() => {
  void load()
  void listen<{ profileId: string; stage: string }>('fre-rust-mod-build', event => {
    if (event.payload.profileId === props.profileId) buildStage.value = event.payload.stage
  }).then(unlisten => { if (disposed) unlisten(); else unlistenBuild = unlisten }).catch(() => {})
})
onUnmounted(() => { disposed = true; generation++; unlistenBuild?.() })
watch(() => props.profileId, () => {
  generation++
  projects.value = []
  reports.value = {}
  pending.value = ''
  expanded.value = ''
  notice.value = ''
  installing.value = false
  justInstalled.value = []
  void load()
})
watch([query, kind, order, pageSize], () => { page.value = 1 })
watch(pageCount, count => { if (page.value > count) page.value = count })
</script>

<template>
  <section class="flex flex-col gap-4">
    <div class="flex flex-wrap items-center justify-between gap-3">
      <Button type="quiet" @click="selectedProject ? expanded = '' : emit('close')"><LeftArrowIcon />{{ selectedProject ? t(m.backResults) : t(m.back) }}</Button>
      <div class="flex items-center gap-2 text-xs text-secondary"><ShieldCheckIcon class="size-4 text-brand" />{{ t(m.autoBrief) }}</div>
    </div>
    <p v-if="installing && buildStage" role="status" class="m-0 rounded-xl bg-surface-3 p-4 text-brand">{{ t(m.buildProgress) }} {{ t(m[buildStage as keyof typeof m] ?? m.compiling) }}</p>
    <p v-if="error" role="alert" class="m-0 text-sm text-red">{{ error }}</p>
    <p v-if="notice" role="status" class="m-0 text-sm text-brand">{{ notice }}</p>
    <p v-if="stale" role="status" class="m-0 text-sm text-secondary">{{ t(m.stale) }}</p>
    <template v-if="selectedProject">
      <ProjectCard layout="list" :title="selectedProject.name" :icon-url="image(selectedProject.icon)" :summary="selectedProject.summary" :author="{ name: selectedProject.authors.join(', ') }" :downloads="selectedProject.downloads">
        <template #actions>
          <Button v-if="builtinKey(selectedProject)" type="outlined" @click="openBuiltin(selectedProject)"><CheckIcon />{{ t(m.inInstance) }}</Button>
          <Button v-else-if="isInstalled(selectedProject)" type="outlined" @click="emit('close')"><CheckIcon />{{ t(m.inInstance) }}</Button>
          <Button v-else-if="['mod', 'native-mod'].includes(selectedProject.kind)" type="colored" color="brand" :disabled="!!pending || stale || disabled || !version(selectedProject)" @click="install(selectedProject)"><SpinnerIcon v-if="pending === selectedProject.id" class="animate-spin" /><DownloadIcon v-else />{{ pending === selectedProject.id ? installing ? t(m.installing) : t(m.checking) : t(m.install) }}</Button>
          <Button v-else type="outlined" disabled>{{ t(m.not_ready) }}</Button>
        </template>
      </ProjectCard>
      <NavTabs mode="local" :links="detailTabs" :active-index="detailTab" @tab-click="selectDetailTab" />
      <div v-if="detailTab === 0" class="flex flex-col gap-4 rounded-2xl bg-surface-3 p-5">
        <p v-if="builtinKey(selectedProject)" class="m-0 text-sm text-brand">{{ t(m.builtin) }}</p>
        <p v-else-if="selectedProject.kind === 'native-mod'" class="m-0 text-sm text-secondary">{{ t(m.needs_build) }}</p>
        <p class="m-0 whitespace-pre-line text-sm leading-6 text-primary">{{ descriptionForFre(selectedProject) }}</p>
        <p class="m-0 text-xs text-secondary">{{ t(m.license) }}: {{ selectedProject.license }} · {{ selectedProject.categories.join(' · ') }}</p>
        <Button v-if="safeSource(selectedProject.source)" type="outlined" class="self-start" @click="source(selectedProject)">{{ t(m.code) }}</Button>
        <p v-if="!['mod','native-mod'].includes(selectedProject.kind)" class="m-0 text-sm text-secondary">{{ t(m.pack_note) }}</p>
      </div>
      <div v-else-if="detailTab === 1" class="overflow-hidden rounded-2xl border border-solid border-surface-4">
        <div v-for="release in selectedProject.versions.filter(v => v.game === '26.3')" :key="release.file.sha256" class="flex flex-wrap justify-between gap-3 border-0 border-b border-solid border-surface-4 bg-surface-3 p-4">
          <span class="font-semibold">{{ release.version }}</span><span class="text-sm text-secondary">Minecraft {{ release.game }}</span><span class="text-sm text-secondary">{{ release.published.slice(0, 10) }}</span>
        </div>
      </div>
      <div v-else class="flex flex-col gap-3">
        <p class="m-0 text-sm text-secondary">{{ t(m.limits) }}</p>
        <Button v-if="['mod','native-mod'].includes(selectedProject.kind)" type="outlined" class="self-start" :disabled="!!pending || loading" @click="verify(selectedProject)"><ShieldCheckIcon />{{ pending === selectedProject.id ? t(m.checking) : t(m.verify) }}</Button>
        <template v-if="report(selectedProject)">
          <div v-for="item in report(selectedProject)?.checks" :key="item.code" class="flex flex-col gap-1 rounded-xl bg-surface-3 p-3 text-sm">
            <div class="flex justify-between gap-3"><span class="font-semibold">{{ checkLabel(item) }}</span><span :class="item.state === 'pass' ? 'text-brand' : item.state === 'fail' ? 'text-red' : 'text-secondary'">{{ stateLabel(item) }}</span></div>
            <span class="break-words text-xs text-secondary">{{ item.detail }}</span>
          </div>
          <p v-if="report(selectedProject)?.facts?.indicators.length" class="m-0 text-sm text-secondary">{{ t(m.indicators) }} {{ report(selectedProject)?.facts?.indicators.join(', ') }}</p>
        </template>
      </div>
    </template>
    <template v-else>
      <Input v-model="query" :icon="SearchIcon" type="search" :placeholder="t(m.search)" :aria-label="t(m.search)" clearable />
      <div class="flex flex-wrap items-center gap-2">
        <Combobox v-model="kind" :options="[{ value: 'mods', label: t(m.mods) }, { value: 'native', label: t(m.native) }, { value: 'data', label: t(m.data) }, { value: 'packs', label: t(m.packs) }, { value: 'all', label: t(m.all) }]" :aria-label="t(m.all)" class="!w-48" />
        <Combobox v-model="order" :options="[{ value: 'downloads', label: t(m.popular) }, { value: 'name', label: t(m.alphabetical) }]" :aria-label="t(m.sort)" class="!w-56 max-w-full" />
        <Combobox v-model="pageSize" :options="[20, 40, 80].map(value => ({ value, label: String(value) }))" :aria-label="t(m.view)" class="!w-40 min-w-max"><template #prefix><span class="font-semibold">{{ t(m.view) }}</span></template></Combobox>
        <IconButton :label="t(m.layout)" @click="layout = layout === 'list' ? 'grid' : 'list'"><GridIcon v-if="layout === 'list'" /><ListIcon v-else /></IconButton>
        <IconButton :label="t(m.refresh)" :disabled="loading || !!pending" @click="load(true)"><RefreshCwIcon /></IconButton>
        <Pagination class="ml-auto" :page="page" :count="pageCount" @switch-page="switchPage" />
      </div>
      <p v-if="loading" class="m-0 py-8 text-center text-sm text-secondary">{{ t(m.loading) }}</p>
      <p v-else-if="!filtered.length" class="m-0 py-12 text-center text-sm text-secondary">{{ t(m.empty) }}</p>
      <template v-else>
        <p class="m-0 text-xs text-secondary">{{ filtered.length }} / {{ projects.length }}</p>
        <ProjectCardList :layout="layout">
          <ProjectCard v-for="project in shown" :key="project.id" :layout="layout" :link="() => details(project)" :title="project.name" :icon-url="image(project.icon)" :summary="project.summary" :author="{ name: project.authors.join(', ') }" :downloads="project.downloads">
            <template #actions>
              <Button v-if="builtinKey(project)" type="outlined" @click.stop="openBuiltin(project)"><CheckIcon />{{ t(m.inInstance) }}</Button>
              <Button v-else-if="isInstalled(project)" type="outlined" @click.stop="emit('close')"><CheckIcon />{{ t(m.inInstance) }}</Button>
              <Button v-else-if="['mod', 'native-mod'].includes(project.kind)" type="colored" color="brand" :disabled="!!pending || stale || disabled || !version(project)" @click.stop="install(project)"><SpinnerIcon v-if="pending === project.id" class="animate-spin" /><DownloadIcon v-else />{{ pending === project.id ? installing ? t(m.installing) : t(m.checking) : t(m.install) }}</Button>
              <Button v-else type="outlined" disabled>{{ t(m.not_ready) }}</Button>
            </template>
          </ProjectCard>
        </ProjectCardList>
        <Pagination class="self-end" :page="page" :count="pageCount" @switch-page="switchPage" />
      </template>
    </template>
  </section>
</template>
