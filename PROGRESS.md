# FRE LAUNCH — development status

## 0.5.0 Alpha / FIELD NOTES

### Implemented in repository

- An original editoral / voxel-world UI: custom SVG biome, ink + paper palette, lime accent, condensed typography, custom navigation and launch console, responsive layouts and five saved themes.
- Frontend ErrorBoundary: fatal React render errors show a branded recovery screen and save error text locally; uncaught JS errors are also preserved in local storage.
- Tauri installer command: prevents overlapping version installs and reports ordinary errors and intercepted async panics. Records local install-start, success and error entries; exposes logs through the Diagnostics page.
- Dual Windows NSIS packages:
  - Standard current-user installer without elevation.
  - Alternative administrator installer with per-user/system-wide choice.
  - Embedded WebView2 bootstrapper.
- GitHub Actions checks TypeScript and Rust, builds both installers and portable EXE, and smoke-tests standard installer in a custom Windows directory.
- Vanilla Mojang downloads with hash verification; initial real Java process launch implementation.
- SQLite-backed instances, local offline accounts, Java manager, Fabric/Quilt metadata support, checked Modrinth resource/mod installs, custom skins and 3D preview, server ping, screenshots and ZIP backups.

### Verification

- 0.4 Windows packaging was green in main.
- 0.5 final build and custom-dir NSIS smoke test must be confirmed through the latest GitHub Actions results.
- No independently confirmed Minecraft gameplay session on a Windows desktop yet.
- The exact cause of a crash reported while clicking Install has not been reproduced because no Windows crash log or exception details were supplied. Added diagnostics and reduced common failure paths, but a successful build alone is not proof of resolution.

### Still missing

- Microsoft OAuth + entitlement validation, secure token storage.
- Forge, NeoForge, broad Minecraft version and GPU/Java compatibility coverage.
- Full tests on affected Windows PC and stable release signing/updater.
- Ability to relocate actual Minecraft game data onto D: through the launcher.
- Full offline/online resilience and compatibility for ancient Minecraft versions.

**Logs are local-only; no automatic crash uploads or telemetry.**
