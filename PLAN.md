# FRE LAUNCH technical roadmap

## Implemented in code (runtime verification still required)
- [x] Desktop Tauri + Rust + React application, GitHub Windows installer.
- [x] Mojang version discovery, vanilla download/install core with SHA-1.
- [x] Streamed, bounded verified downloads and restricted HTTPS redirects.
- [x] Per-instance isolated natives, libraries and Java selection.
- [x] Windows x64 verified Temurin JRE download.
- [x] SQLite instances with per-instance RAM and Fabric Loader settings.
- [x] Official Fabric Meta loader selection and Maven dependency installation.
- [x] Modrinth Fabric mod installation, required dependencies, SHA-512 and enable/disable/remove.
- [x] Server list with live Minecraft protocol status checks.
- [x] Local offline accounts, skin preview/editor, themes, world backups.
- [x] Safe per-instance screenshot gallery with PNG preview, download and delete.
- [x] Legacy virtual asset directories for selected historical versions.

## Still to build or verify
- [ ] Confirm 0.4 Windows CI fully passes and merge stable changes.
- [ ] Test actual offline-compatible vanilla and Fabric gameplay on Windows.
- [ ] Microsoft OAuth with entitlement verification and safe credential vault.
- [ ] Forge, NeoForge and Quilt.
- [ ] Production-grade rollback, resumable downloads and full repair flows.
- [ ] Update service, full localization, settings import/export and crash telemetry opt-in.
- [ ] Extensive security and accessibility review; stable release.
