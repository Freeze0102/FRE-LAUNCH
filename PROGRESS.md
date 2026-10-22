# FRE LAUNCH — progress

## Current milestone: 0.2.0 Alpha

### Implemented in this branch (UNVERIFIED until CI finishes)
- Repair workflow setup: eliminate npm caching without lockfile; generate application icons from SVG before Tauri compilation.
- Rust Minecraft vanilla installer: official version metadata, client JAR, library downloads, asset index and hashed resources, safe native JAR extraction.
- SHA-1 validation for Mojang artifacts and safe temporary downloads.
- Modern and legacy JVM/game argument preparation (vanilla only, Windows targeted).
- Java PATH detection and basic compatibility check.
- Local offline UUID generation, nickname validation and process spawning.
- Real-time Minecraft logs and installation progress emitted to frontend.
- Version selection, real Install/Play actions, progress/logs panels.
- Persisted local nickname, selected version, and RAM using Zustand local storage.
- Windows GitHub Actions build with NSIS artifact and portable EXE artifact if available.

### Not implemented / limitations
- Microsoft OAuth, entitlement validation, secure token vault.
- Fabric, Forge, NeoForge, Quilt, Modrinth.
- Custom skins and capes.
- Separate configurable instances, backups, servers, updater.
- Java runtime downloader / per-instance Java selection.
- Full language switching and theme system.
- Legacy versions with inherited metadata or missing download fields; old legacy asset behavior remains incomplete.
- No confirmed runtime test on Windows or confirmed successful GitHub Actions build yet.

### Testing status
The execution environment used to author these commits does not have Cargo/Rust or network access to install npm packages; TypeScript and Rust compilation remain UNVERIFIED until GitHub Actions completes. Do NOT advertise a working installer or verified game launch.

### Next tasks
1. Inspect GitHub Actions build and fix any errors until it is green.
2. Install and launch a recent vanilla release on Windows, check logs.
3. Replace hardcoded Java PATH with managed runtimes.
4. Add robust persistent SQLite installation metadata and independent instances.
5. Continue with authenticated profiles, modloaders, skins, and full i18n.
