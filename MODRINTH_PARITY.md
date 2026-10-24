## Source-code comparison (24 October 2026)

Studied public Modrinth App source files directly, not merely screenshots:

- [Theseus .mrpack installer](https://github.com/modrinth/code/blob/main/packages/app-lib/src/api/pack/install_mrpack.rs) and [pack format types](https://github.com/modrinth/code/blob/main/packages/app-lib/src/api/pack/install_from.rs): manifest, environment-specific content, verified downloads, safe extraction, failure handling.
- [Minecraft skin management](https://github.com/modrinth/code/blob/main/packages/app-lib/src/api/minecraft_skins.rs) and [Mojang skin API](https://github.com/modrinth/code/blob/main/packages/app-lib/src/state/minecraft_skins/mojang_api.rs): local saved skins are distinct from the real Mojang skin, which is changed using authenticated Minecraft Services requests.
- [Official .mrpack format](https://support.modrinth.com/en/articles/8802351-modrinth-modpack-format-mrpack).

Implemented in FRE LAUNCH with independent source files:
- `src-tauri/src/mrpack.rs`: download a published Modrinth .mrpack, validate format v1/index/ZIP paths, SHA-512 and optional SHA-1, skip client-unsupported entries, honor Minecraft and Fabric/Quilt loader dependencies, stage downloads safely, and open a new instance. Allowed official Modrinth/GitHub/GitLab hosts; external hosts fail instead of being fetched without verification.
- `src/ModsPanel.tsx`: real "Install modpack" controls for listed projects and selected releases, error/progress feedback, opening the new instance.
- `src-tauri/src/microsoft.rs`: Minecraft Services skin PNG multipart upload, Classic/Slim model, and default-skin reset without sending tokens to JavaScript.
- `src/SkinStudio.tsx`: explicit local preview versus real account upload; real upload requires active authenticated premium Minecraft session and a permitted Microsoft client registration.

**Limits**: no `.mrpack` file-picker/import-from-disk yet, Forge and NeoForge modpacks rejected explicitly, client-optional files currently installed automatically, server-only files skipped, large packs and unsupported external hosts rejected for security, no complete rollback guarantee after unusual filesystem failures. Do not treat this as full functional parity with Modrinth. **Windows Rust+TypeScript compilation and live modpack/skin tests have not yet run**, and the new code may require fixes in the local build.

## New video-based parity pass — 24 October 2026

Reference: user-supplied Windows recording of Modrinth App 0.22.0 (not a website screenshot). FRE LAUNCH keeps its own brand, code and data directories. **"Without hosts" excludes paid/commercial server hosting, not multiplayer BedWars/Survival servers.**

- [x] Replaced the older home dashboard with the current Modrinth-like unified **Play**: Jump In list with real Play actions, instance-library grid/list, search and sorting, color/icon customizer, local groups, right onboarding/account pane.
- [x] The Discover view now has a right-hand category-filter sidebar driven by validated Modrinth search facets, the five content tabs plus a shortcut to ordinary multiplayer servers, and a real project-detail view.
- [x] Instance detail follows the video: large title, loader/RAM subline, native Play button, settings shortcut, pill-like Content/Files/Worlds/Logs/Settings tabs and real file toggles.
- [x] The instance-creation dialog now asks how to create a profile and supports real Vanilla/Fabric/Quilt setup using loaders returned by the Rust backend.
- [x] Settings are a centered category-driven dialog; its appearance, RAM and visible-tab switches are real persisted controls. Sections with unsupported operations do not pretend to work.
- [x] Vanilla multiplayer server IP/status/favorites list remains available; no commercial hosting UI.
- [x] Kept the local skin gallery, 3D preview and editor as FRE-owned features.

**Remaining differences from Modrinth:** pixel-perfect Windows screenshot comparison across all screens, real .mrpack import/install, all official default skin variants and licensed Minecraft cape selection, cloud sync, complete language translations, fully authorized persistent Microsoft accounts, automatic mod update replacement, Forge/NeoForge installers, exact file upload/drag-drop behavior, and full native Windows controls.

**Build status:** Changes have been committed to GitHub but **not compiled or launched on the user's Windows machine in this edit**. A prior Cargo resolver error caused by the nonexistent reqwest "form" feature was fixed in Cargo.toml; a new local Rust check is required before calling this working. The previously reported Defender detection on v0.5.5 remains unresolved; do not disable Defender or restore flagged installers.

# FRE LAUNCH / Modrinth App visual and functional parity

Our goal is a close reproduction of the **Modrinth App desktop interaction model** in independent, editable React + Tauri/Rust code, without embedding Modrinth's app source or impersonating its brand. Reference: https://modrinth.com/app and https://github.com/modrinth/code. Never claim that unfinished functions are operational.

## Layout implemented in source

- [x] Narrow dark sidebar with Home, Discover, Skins, Library, quick instance shortcuts, and a Minecraft multiplayer-server list (IP, favorite, Java status ping). No commercial hosting section.
- [x] Home cards of actual popular Modrinth modpacks and mods, with project-detail deep links and download counts.
- [x] Discover tabs for Modpacks, Mods, Resource Packs and Shaders, sorting, compatible game-version filtering, project listings with icon, title, author and tags.
- [x] Project details: icon, text, download count, version list, description/versions tabs, safe Markdown formatting.
- [x] Library grid/list layout, search, sorting, favorites, create-profile dialog, independent game directories.
- [x] Per-profile tabs for Content, Worlds, Files, Logs and Settings.
- [x] Per-profile Content table: mod / resourcepack / shader filters, enable/disable, delete, browse content, open folder.
- [x] Real read-only world discovery from each instance's `saves` folder with modified timestamps; world backups via existing commands.
- [x] Skin preview + locally saved PNG skins gallery, standard skin, local pixel editor.
- [x] Prepared Microsoft device-code login flow with native Rust Xbox/Minecraft verification, requiring an app registration approved for Minecraft Services.

## Not yet equivalent to Modrinth

- [ ] **Pixel-perfect 1:1 verification:** screenshots at each viewport resolution have not been manually compared in a running Windows app. Spacing, animations, icons, fonts, and modal microinteractions still need visual QA.
- [ ] Modpack creation/installation and `.mrpack` import/export.
- [ ] Forge / NeoForge instance installation.
- [ ] Complete dependency resolver and atomic in-place mod updates.
- [ ] Microsoft persistent multi-account sign-in and refresh token storage; a properly authorized Microsoft client ID is required to make login operational.
- [ ] Skins uploaded to a real Minecraft profile or synchronized to a Microsoft account; skins here are only local preview/editor assets.
- [ ] Automatic launcher updates from unmerged source commits. The built-in updater checks published GitHub releases only.
- [ ] Modrinth cloud functions, cross-device settings synchronization, friends, **commercial server hosting**, app-account functionality.
- [ ] Full rich world settings, screenshot annotations and every optional preference present in current Modrinth App.

## How to verify each new build without GitHub Actions

Use `FRE-LAUNCH-LOCAL.cmd` and choose **1**. It runs TypeScript checks before the development build. Alternatively execute:

```powershell
npm install --no-audit --no-fund
npm run check
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri dev
```

Run these locally after copying fresh source into a **retained working directory**; preserve `node_modules` and `src-tauri\target` to avoid redownloading gigabytes. This version has **not** been validated by a fresh Windows CI run. Make a backup of existing Minecraft worlds before trying experimental versions.

Older 0.5.5 installer was reported by Windows Defender as `Trojan:Win32/Bearfoos.B!ml`. Root cause is still undetermined and that installer must remain on security hold. Local compilation does not constitute a malware clearance or authorization to bypass Windows Security.
