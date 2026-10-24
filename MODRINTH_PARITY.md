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
