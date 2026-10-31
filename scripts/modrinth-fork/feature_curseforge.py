"""Install opt-in CurseForge integration into the pinned FRE LAUNCH desktop fork.

Uses CurseForge's authorized third-party API (approved key required).
Does not embed a vendor key, bypass distribution settings or scrape pages.
"""
from pathlib import Path
import shutil

def exactly(path: Path, before: str, after: str):
    text = path.read_text(encoding="utf-8")
    hits = text.count(before)
    if hits != 1:
        raise RuntimeError(f"CurseForge source anchor drift ({hits}): {path}: {before[:90]!r}")
    path.write_text(text.replace(before, after, 1), encoding="utf-8")

def apply(root: Path):
    templates = Path(__file__).resolve().parent
    rust = root / "apps/app/src/api/curseforge.rs"
    vue = root / "apps/app-frontend/src/components/ui/CurseForgeBrowser.vue"
    shutil.copyfile(templates / "curseforge_backend.rs", rust)
    shutil.copyfile(templates / "CurseForgeBrowser.vue", vue)

    module = root / "apps/app/src/api/mod.rs"
    exactly(module,
            "pub mod cache;\npub mod files;",
            "pub mod cache;\npub mod curseforge;\npub mod files;")

    main = root / "apps/app/src/main.rs"
    exactly(main,
            "        .plugin(api::instance::init())\n",
            "        .plugin(api::instance::init())\n        .plugin(api::curseforge::init())\n")

    build = root / "apps/app/build.rs"
    exactly(build,
        '            .plugin(\n                "instance",',
        '''            .plugin(
                "fre-curseforge",
                InlinedPlugin::new()
                    .commands(&["cf_search", "cf_install"])
                    .default_permission(DefaultPermissionRule::AllowAllCommands),
            )
            .plugin(
                "instance",''')

    # A new Tauri plugin needs an explicit capability on the main window.
    # Its default permission allows only cf_search and cf_install.
    capability = root / "apps/app/capabilities/plugins.json"
    exactly(capability,
            '"instance:default",',
            '"instance:default",\n\t\t"fre-curseforge:default",')

    cargo = root / "apps/app/Cargo.toml"
    exactly(cargo, "sha2 = { workspace = true }",
                  "sha1 = { workspace = true }\nsha2 = { workspace = true }")

    browse = root / "apps/app-frontend/src/pages/Browse.vue"
    exactly(browse,
            "import { useAppServerBrowse } from '@/composables/browse/use-app-server-browse'",
            "import CurseForgeBrowser from '@/components/ui/CurseForgeBrowser.vue'\n"
            "import { useAppServerBrowse } from '@/composables/browse/use-app-server-browse'")
    exactly(browse,
            "const projectType = ref<ProjectType>(route.params.projectType as ProjectType)",
            "const projectType = ref<ProjectType>(route.params.projectType as ProjectType)\n"
            "const freContentSource = ref<'modrinth' | 'curseforge'>('modrinth')")
    exactly(browse,
            '''<template>
\t<div class="flex flex-col gap-2 p-6">
\t\t<BrowsePageLayout>''',
            '''<template>
\t<div class="flex flex-col gap-2 p-6">
\t\t<div v-if="projectType === 'mod'" class="flex flex-wrap items-center gap-2 rounded-xl border border-solid border-surface-4 bg-bg-raised p-2">
\t\t\t<span class="mr-2 px-2 text-sm text-secondary">{{ formatMessage(sourceMessages.sourceLabel) }}</span>
\t\t\t<button
\t\t\t\ttype="button"
\t\t\t\t:aria-pressed="freContentSource === 'modrinth'"
\t\t\t\t:class="freContentSource === 'modrinth' ? 'bg-brand text-white' : 'bg-button-bg text-secondary hover:text-contrast'"
\t\t\t\tclass="cursor-pointer rounded-lg border-0 px-4 py-2 text-sm font-semibold transition-colors"
\t\t\t\t@click="freContentSource = 'modrinth'"
\t\t\t>Modrinth</button>
\t\t\t<button
\t\t\t\ttype="button"
\t\t\t\t:aria-pressed="freContentSource === 'curseforge'"
\t\t\t\t:class="freContentSource === 'curseforge' ? 'bg-brand text-white' : 'bg-button-bg text-secondary hover:text-contrast'"
\t\t\t\tclass="cursor-pointer rounded-lg border-0 px-4 py-2 text-sm font-semibold transition-colors"
\t\t\t\t@click="freContentSource = 'curseforge'"
\t\t\t>CurseForge</button>
\t\t</div>
\t\t<CurseForgeBrowser
\t\t\tv-if="projectType === 'mod' && freContentSource === 'curseforge'"
\t\t\t:instance-id="String(route.query.i ?? '')"
\t\t/>
\t\t<BrowsePageLayout v-else>''')
    exactly(browse,
            '<Teleport v-if="browseRouteActive" to="#sidebar-teleport-target">',
            '<Teleport v-if="browseRouteActive && (projectType !== \'mod\' || freContentSource === \'modrinth\')" to="#sidebar-teleport-target">')
    exactly(browse,
            "const freContentSource = ref<'modrinth' | 'curseforge'>('modrinth')",
            '''const freContentSource = ref<'modrinth' | 'curseforge'>('modrinth')
const sourceMessages = defineMessages({
    sourceLabel: { id: 'fre-launch.cf.source', defaultMessage: 'Mod source' },
})''')

    print("FRE LAUNCH CurseForge source selector, approved API search and verified mod installation added.")

if __name__ == "__main__":
    raise SystemExit("Import through prepare.py")
