"""Complete Rust integration with the pinned Java visual components.

Native Rust records remain native; never pass their IDs to Theseus Java
install/remove APIs. All patches are one-hit and fail closed on upstream drift.
"""
from pathlib import Path

def exactly(file: Path, before: str, after: str) -> None:
    text = file.read_text(encoding="utf-8")
    matches = text.count(before)
    if matches != 1:
        raise RuntimeError(f"Rust polish source anchor drift ({matches}): {file}: {before[:90]!r}")
    file.write_text(text.replace(before, after, 1), encoding="utf-8")

def apply(root: Path) -> None:
    frontend = root / "apps/app-frontend/src"
    quick = frontend / "components/ui/QuickInstanceSwitcher.vue"

    exactly(quick, "import { useQuery } from '@tanstack/vue-query'",
            "import { invoke } from '@tauri-apps/api/core'\nimport { useQuery } from '@tanstack/vue-query'")
    exactly(quick, "import NavButton from '@/components/ui/NavButton.vue'",
            "import NavButton from '@/components/ui/NavButton.vue'\n"
            "import ConfirmDeleteInstanceModal from '@/components/ui/modal/ConfirmDeleteInstanceModal.vue'")
    exactly(quick, "const runningInstances = ref([])",
            """const runningInstances = ref([])
const freRustInstances = ref([])
const freRustDeleteCandidate = ref(null)
const freRustDeleteModal = ref(null)
let freRustQueryId = 0
async function refreshFreRustInstances() {
    const request = ++freRustQueryId
    const native = []
    try {
        const [profiles, status] = await Promise.all([
            invoke('plugin:fre-rust|rust_profiles'),
            invoke('plugin:fre-rust|rust_runtime'),
        ])
        for (const profile of profiles) native.push({
            id: 'rust:' + profile.id,
            freRustId: profile.id,
            freRustKind: 'native',
            name: profile.name,
            loader: 'Rust',
            game_version: status.version || '26.3',
            icon_path: profile.iconPath ?? null,
            created: new Date(profile.created * 1000).toISOString(),
            last_played: null,
            install_stage: 'installed',
            quarantined: false,
        })
    } catch { /* Still show ordinary Java profiles without a Rust backend. */ }
    try {
        const exePath = await invoke('plugin:fre-horizon|horizon_detect_launcher')
        if (exePath) {
            const status = await invoke('plugin:fre-horizon|horizon_status', { exePath })
            for (const profile of status.instances) {
                if (profile.id !== 'horizon' &&
                    !/^[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}$/.test(profile.id)) continue
                native.push({
                    id: 'horizon:' + profile.id,
                    freRustId: profile.id,
                    freRustKind: 'horizon',
                    name: profile.name,
                    loader: 'Rust',
                    game_version: profile.version || status.selectedVersion || '26.3',
                    icon_path: null,
                    created: new Date(0).toISOString(),
                    last_played: null,
                    install_stage: 'installed',
                    quarantined: false,
                })
            }
        }
    } catch { /* A removed Horizon.exe cannot hide Java or native Rust. */ }
    if (request === freRustQueryId) freRustInstances.value = native
}
function freRustDetailUrl(instance) {
    return '/rust-instance/' + instance.freRustKind + '/' + encodeURIComponent(instance.freRustId)
}
async function playFreRustQuick(instance) {
    try {
        if (instance.freRustKind === 'native') {
            let status = await invoke('plugin:fre-rust|rust_runtime')
            if (!status.prepared) {
                try {
                    await invoke('plugin:fre-rust|rust_import_resources', { source: '' })
                    status = await invoke('plugin:fre-rust|rust_runtime')
                } catch { /* Open the profile for an actionable missing-input message. */ }
            }
            if (!status.installed || !status.prepared) {
                await router.push(freRustDetailUrl(instance))
                return
            }
            await invoke('plugin:fre-rust|rust_launch', { profileId: instance.freRustId })
        } else {
            const exePath = await invoke('plugin:fre-horizon|horizon_detect_launcher')
            if (!exePath) throw new Error('Horizon executable not found')
            await invoke('plugin:fre-horizon|horizon_direct_play', {
                exePath, name: instance.freRustId,
            })
        }
    } catch (error) {
        handleError(error instanceof Error ? error : new Error(String(error)))
    }
}
function requestFreRustQuickDelete(instance) {
    if (instance.freRustKind !== 'native') return
    freRustDeleteCandidate.value = instance
    freRustDeleteModal.value?.show()
}
async function deleteFreRustQuick() {
    const instance = freRustDeleteCandidate.value
    if (!instance || instance.freRustKind !== 'native') return
    try {
        await invoke('plugin:fre-rust|rust_delete_profile', { profileId: instance.freRustId })
        freRustDeleteCandidate.value = null
        await refreshFreRustInstances()
        window.dispatchEvent(new Event('fre-rust-library-refresh'))
    } catch (error) {
        handleError(error instanceof Error ? error : new Error(String(error)))
    }
}""")
    exactly(quick,
            "(instancesQuery.data.value ?? []).slice().sort((a, b) => {",
            "[...(instancesQuery.data.value ?? []), ...freRustInstances.value].sort((a, b) => {")
    exactly(quick,
            "onMounted(() => {\n\tresizeObserver = new ResizeObserver(updateMaxAuto)",
            """onMounted(() => {
    window.addEventListener('fre-rust-library-refresh', refreshFreRustInstances)
    window.addEventListener('fre-horizon-library-refresh', refreshFreRustInstances)
    void refreshFreRustInstances()
\tresizeObserver = new ResizeObserver(updateMaxAuto)""")
    exactly(quick,
            "onUnmounted(() => {\n\tresizeObserver?.disconnect()",
            """onUnmounted(() => {
    freRustQueryId++
    window.removeEventListener('fre-rust-library-refresh', refreshFreRustInstances)
    window.removeEventListener('fre-horizon-library-refresh', refreshFreRustInstances)
\tresizeObserver?.disconnect()""")
    exactly(quick,
            "async function playInstance(instance) {\n\tif (instance.quarantined",
            """async function playInstance(instance) {
    if (instance.freRustKind) return playFreRustQuick(instance)
\tif (instance.quarantined""")
    exactly(quick,
            "async function stopInstance(instance) {\n\tawait kill(instance.id)",
            """async function stopInstance(instance) {
    if (instance.freRustKind) return
\tawait kill(instance.id)""")
    exactly(quick,
            "function openContextMenu(event, instance) {\n\tconst playing =",
            """function openContextMenu(event, instance) {
    if (instance.freRustKind) {
        instanceOptions.value?.open(event, [
            {
                id: 'play',
                label: formatMessage(commonMessages.playButton),
                icon: PlayIcon,
                action: () => void playFreRustQuick(instance),
            },
            { type: 'divider' },
            {
                id: 'open-instance',
                label: formatMessage(messages.viewInstance),
                icon: EyeIcon,
                action: () => void router.push(freRustDetailUrl(instance)),
            },
            {
                id: 'open-folder',
                label: formatMessage(commonMessages.openFolderButton),
                icon: FolderOpenIcon,
                action: async () => {
                    try {
                        if (instance.freRustKind === 'native') {
                            await invoke('plugin:fre-rust|rust_open_folder', {
                                profileId: instance.freRustId, folder: 'root',
                            })
                        } else {
                            const exePath = await invoke('plugin:fre-horizon|horizon_detect_launcher')
                            if (!exePath) throw new Error('Horizon executable not found')
                            await invoke('plugin:fre-horizon|horizon_open_folder', {
                                exePath, name: instance.freRustId, folder: 'root',
                            })
                        }
                    } catch (error) {
                        handleError(error instanceof Error ? error : new Error(String(error)))
                    }
                },
            },
            ...(instance.freRustKind === 'native' ? [{
                id: 'delete',
                label: formatMessage(commonMessages.deleteLabel),
                tone: 'red',
                action: () => requestFreRustQuickDelete(instance),
            }] : []),
        ])
        return
    }
\tconst playing =""")
    exactly(quick,
            "<template>\n\t<div ref=\"container\"",
            """<template>
    <ConfirmDeleteInstanceModal
        ref="freRustDeleteModal"
        :instances="freRustDeleteCandidate ? [freRustDeleteCandidate] : []"
        @delete="deleteFreRustQuick"
    />
\t<div ref="container\"""")
    exactly(quick,
            ':to="`/instance/${encodeURIComponent(instance.id)}`"',
            ':to="instance.freRustKind ? freRustDetailUrl(instance) : `/instance/${encodeURIComponent(instance.id)}`"')

    # The main library's native cards use the actual shared Java selection
    # state. The special ID namespace is crucial: Java bulk operations cannot
    # silently receive a native Rust profile ID.
    grid = frontend / "components/ui/library/instance-group/RustInstanceGrid.vue"
    exactly(grid,
            "import { FolderOpenIcon, PlayIcon, SettingsIcon, TrashIcon } from '@modrinth/assets'",
            "import { CheckIcon, FolderOpenIcon, PlayIcon, SettingsIcon, TrashIcon } from '@modrinth/assets'")
    exactly(grid,
            "import InstanceCardView from '@/components/ui/library/instance-group/instance-card-view.vue'",
            "import InstanceCardView from '@/components/ui/library/instance-group/instance-card-view.vue'\n"
            "import { getLibraryInstanceSelectionKey, useLibrary } from '@/components/ui/library/use-library'")
    exactly(grid,
            "  startIndex: number\n",
            "  startIndex: number\n  groupId: string\n")
    exactly(grid, "const router = useRouter()",
            """const router = useRouter()
const {
    selectedLibraryInstances,
    isLibraryInstanceSelectionActive,
    toggleLibraryInstanceSelection,
} = useLibrary()
function selectionKey(card: Card) {
    return getLibraryInstanceSelectionKey({
        instanceId: card.key,
        groupId: props.groupId,
    })
}
function isSelected(card: Card) {
    return selectedLibraryInstances.value.has(selectionKey(card))
}
function selectCard(card: Card) {
    if (card.kind !== 'native') return
    toggleLibraryInstanceSelection({
        instanceId: card.key, groupId: props.groupId,
    })
}
function activateCard(event: MouseEvent, card: Card) {
    if (card.kind === 'native' && (isLibraryInstanceSelectionActive.value || event.shiftKey)) {
        selectCard(card)
    } else {
        openCard(card)
    }
}""")
    exactly(grid,
            ':instance="viewInstance(card)"',
            ':instance="viewInstance(card)"\n      :selected="isSelected(card)"')
    exactly(grid,
            '@click="openCard(card)"',
            '@click="activateCard($event, card)"')
    exactly(grid,
            '@keydown.enter.prevent="openCard(card)"',
            '@keydown.enter.prevent="isLibraryInstanceSelectionActive && card.kind === \'native\' ? selectCard(card) : openCard(card)"')
    exactly(grid,
            "      @contextmenu.prevent=\"openContext($event, card)\"",
            "      @contextmenu.prevent=\"openContext($event, card)\"")
    exactly(grid,
            "    >\n      <template #leading=",
            """    >
      <template #overlay="{ compact }">
        <button
          v-if="card.kind === 'native'"
          type="button"
          class="selection-button group/selection absolute z-[2] flex size-[50px] cursor-pointer items-start justify-center border-0 bg-transparent p-0 pt-4"
          :class="compact ? '-right-1 -top-1' : 'right-2 top-1.5'"
          :aria-label="isSelected(card) ? 'Deselect instance' : 'Select instance'"
          :aria-pressed="isSelected(card)"
          @click.stop="selectCard(card)"
        >
          <span
            class="relative flex items-center justify-center rounded-full opacity-0 transition-opacity duration-200 ease-out group-hover/card:opacity-100 group-hover/selection:brightness-125"
            :class="{
              'size-[20px]': compact,
              'size-[24px]': !compact,
              'border-0 !opacity-100': isSelected(card),
              'border-2 border-solid border-primary bg-transparent': !isSelected(card),
            }"
          >
            <span v-if="isSelected(card)" class="absolute inset-0 rounded-full bg-contrast" />
            <CheckIcon v-if="isSelected(card)" class="relative size-4 invert [stroke-width:3]" />
          </span>
        </button>
      </template>
      <template #leading=""")
    group = frontend / "components/ui/library/instance-group/index.vue"
    exactly(group, ':start-index="instanceGroup.instances.length"',
                  ':start-index="instanceGroup.instances.length"\n\t\t\t\t\t\t:group-id="instanceGroup.id"')

    # When Rust cards are selected, bulk actions must route them to the
    # validated native backend, never call Java's remove() on a Rust ID.
    bar = frontend / "components/ui/library/LibrarySelectionActionBar.vue"
    exactly(bar, "import { MinusIcon, SquarePlusIcon, TrashIcon } from '@modrinth/assets'",
            "import { invoke } from '@tauri-apps/api/core'\n"
            "import { MinusIcon, SquarePlusIcon, TrashIcon } from '@modrinth/assets'")
    exactly(bar, "import { computed, ref } from 'vue'",
            "import { computed, onMounted, onUnmounted, ref } from 'vue'")
    exactly(bar, "const deleting = ref(false)",
            """const deleting = ref(false)
const freRustProfiles = ref([])
let freRustRequest = 0
async function refreshNativeRustForSelection() {
    const request = ++freRustRequest
    try {
        const profiles = await invoke('plugin:fre-rust|rust_profiles')
        if (request === freRustRequest) freRustProfiles.value = profiles
    } catch { if (request === freRustRequest) freRustProfiles.value = [] }
}
onMounted(() => {
    window.addEventListener('fre-rust-library-refresh', refreshNativeRustForSelection)
    void refreshNativeRustForSelection()
})
onUnmounted(() => {
    freRustRequest++
    window.removeEventListener('fre-rust-library-refresh', refreshNativeRustForSelection)
})
const hasRustSelection = computed(() =>
    [...selectedLibraryInstances.value.values()].some(
        selection => selection.instanceId.startsWith('rust:'),
    ),
)""")
    exactly(bar,
            'v-if="displayState.group === \'Group\'"',
            'v-if="displayState.group === \'Group\' && !hasRustSelection"')
    exactly(bar,
            "const selectedInstances = computed(() =>\n\tinstances.value.filter((instance) => selectedInstanceIds.value.has(instance.id)),\n)",
            """const selectedInstances = computed(() => [
    ...instances.value.filter(instance => selectedInstanceIds.value.has(instance.id)),
    ...freRustProfiles.value
        .filter(profile => selectedInstanceIds.value.has('rust:' + profile.id))
        .map(profile => ({
            id: 'rust:' + profile.id,
            name: profile.name,
            icon_path: profile.iconPath ?? null,
            loader: 'Rust',
            game_version: '26.3',
        })),
])""")
    exactly(bar,
            "const instanceIds = selectedInstanceIds.value\n\tclearLibraryInstanceSelection()",
            "const instanceIds = new Set([...selectedInstanceIds.value].filter(id => !id.startsWith('rust:')))\n\tclearLibraryInstanceSelection()")
    exactly(bar,
            "const results = await Promise.allSettled(instanceIds.map((instanceId) => remove(instanceId)))",
            """const results = await Promise.allSettled(instanceIds.map(instanceId =>
        instanceId.startsWith('rust:')
            ? invoke('plugin:fre-rust|rust_delete_profile', { profileId: instanceId.slice(5) })
            : remove(instanceId),
    ))""")
    exactly(bar,
            "setSelectedLibraryInstances(\n\t\t[...selectedLibraryInstances.value.values()].filter(\n\t\t\t(selection) => !deletedInstanceIds.has(selection.instanceId),\n\t\t),\n\t)\n\tdeleting.value = false",
            """setSelectedLibraryInstances(
        [...selectedLibraryInstances.value.values()].filter(
            selection => !deletedInstanceIds.has(selection.instanceId),
        ),
    )
    if ([...deletedInstanceIds].some(id => id.startsWith('rust:'))) {
        window.dispatchEvent(new Event('fre-rust-library-refresh'))
    }
\tdeleting.value = false""")
