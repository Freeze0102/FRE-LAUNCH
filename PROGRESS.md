# FRE LAUNCH development status

Version: **0.4.0 Alpha** — feature branch `feat/launcher-reliability-fabric`.

## Code completed in this branch (CI verification pending)
- Streamed downloads (SHA-1, size cap, HTTPS host and redirect validation).
- Per-instance native extraction and Java selection based on required game version.
- Legacy virtual/resource asset materialization.
- Fabric Loader integration through documented Fabric Meta profile endpoint.
- Modrinth Fabric mod installation (required dependencies, verified SHA-512), local mod toggles/removal.
- Live Minecraft Java server status protocol and persistent server addresses.
- Per-instance screenshot gallery with safe PNG validation, thumbnails and deletion.
- Portable Temurin JRE installation with SHA-256 checksum validation; versions 8, 11, 17, 21, 25.
- React interface for modloaders, mods, Java installs and server management.

## Existing from 0.3.0 Alpha
- Original dark launcher UI with themes, skin editor, local profiles, SQLite game instances.
- Mojang version manifest, Java process launch, live logs, saved world backups, resource packs.
- Windows GitHub Actions NSIS installer and portable EXE.

## Verification
- **0.3.0 main Windows build passed**, see GitHub Actions.
- **0.4.0:** GitHub Actions in PR #3 will test TypeScript/Rust and Windows packaging.
- Actual Minecraft gameplay has **not** been independently verified in a real Windows graphical session.
- Do not call 0.4 a stable or fully verified game launcher yet.

## Known limitations
- Microsoft OAuth and entitlements are not implemented; offline profiles only.
- Forge/NeoForge/Quilt not implemented.
- Older versions and advanced OS-specific launcher rules may require fixes.
- No robust pause/resume download UI; version installer progress is stage-based.
- Modrinth resources and Minecraft game launch require network, except already cached assets.

## Next priorities
1. Green Windows CI and installer artifacts for 0.4.
2. Real vanilla/Fabric launch smoke tests on Windows.
3. Microsoft accounts and secure authentication.
4. Additional modloaders and version compatibility testing.
5. UI polish, localization and stable release packaging.
