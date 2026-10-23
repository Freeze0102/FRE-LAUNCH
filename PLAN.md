# FRE LAUNCH roadmap

## Developed
- [x] Windows Tauri 2 + Rust/React launcher and GitHub Actions builds.
- [x] Real Mojang downloads, asset checks, launch process and live logs.
- [x] SQLite instances, settings, offline accounts, skins, Modrinth, server status, screenshots, world backups.
- [x] Fabric and experimental Quilt metadata integration.
- [x] Managed Java / installed Java detection.
- [x] Unique FRE FIELD NOTES interface (no recycled glassmorphism template).
- [x] Local install diagnostics, async panic handling, install concurrency guard and React error boundary.
- [x] Separate current-user/admin NSIS build paths and custom-directory install smoke-test code.

## Verification required
- [ ] Confirm both 0.5 Windows NSIS installers and custom-directory CI smoke test green.
- [ ] Reproduce and fix remaining crash, if any, on the actual affected Windows machine.
- [ ] Launch a modern Vanilla Minecraft world on Windows, verify Java, natives, assets, game logs.
- [ ] Fabric and Quilt real-game smoke tests, version matrix.

## After reliability milestone
- [ ] Microsoft OAuth + entitlement, secure credential storage.
- [ ] Forge and NeoForge.
- [ ] Configurable Minecraft game data directory / safe migration to D:.
- [ ] Resume/pause downloads, updater signing, UI accessibility, complete i18n, user-facing release notes.

**No "fully ready" claim without actual gameplay verification.**
