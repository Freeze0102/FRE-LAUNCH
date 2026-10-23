# FRE LAUNCH — PROGRESS

## 0.3.0 Alpha (candidate, CI verification pending)

### Implemented in repository
- **FRE SCREENSHOT GALLERY:** instance-scoped PNG list/preview, download and confirmed deletion; validates filenames, disallows symlinks and bounds PNG dimensions and size.
- **FRE SERVER HUB (new):** SQLite server favorites, add/edit/delete, true Minecraft Java TCP status handshake with bounded response, 8-second timeout, ping/players/version/MOTD. SRV DNS lookup and direct-connect launch not yet supported.
- Tauri 2 / Rust / React / TypeScript desktop project.
- Windows GitHub Actions with TypeScript, Rust unit tests, icon generation, NSIS installer and portable EXE artifacts.
- Mojang version manifest discovery for release, snapshots, old alpha and beta.
- Vanilla installer: client libraries, resources, SHA-1 download integrity, native extraction.
- Offline-compatible launcher process with Java path and per-instance directories; live log stream and launch status.
- **SQLite instance manager:** create, rename, favorite, independent game folders, per-instance RAM and Java executable.
- **Modrinth catalog:** real searches filtered by version/category. Verified resource-pack ZIP installer into selected instance with SHA-512 validation.
- **FRE SKIN STUDIO:** interactive 3D viewer, rotation, walking/running animation, skin PNG and cape preview; skin PNG saved locally.
- **FRE THEMES:** five live presets persisted across restarts.
- **FRE BACKUPS:** zipped world backups and safe restoration into new world folders without overwriting existing saves.
- **FRE ACCOUNTS:** multiple SQLite-backed offline profiles and UUIDs.
- **FRE JAVA MANAGER:** find locally installed Java installations and select a runtime for any instance.
- Ukrainian interface, local nickname, recent selected version and RAM settings.
- Safety: UUID-based paths, version sanitization, file hashes, no shell execution, HTTPS-only approved download hosts.

### Unsupported / next work
- Mojang/Microsoft OAuth and premium entitlement validation; no Minecraft purchase bypass.
- Forge, Fabric, NeoForge, Quilt, dependency-resolving mod install, import/export modpacks.
- Auto-installation of managed Java, signed updates, complete multilingual translations.
- Server ping, skin upload to licensed account, screenshot gallery.
- Full legacy version launch compatibility (old alpha/beta may need metadata/assets not currently handled).

### Verification status
Windows build in main for 0.2.0 passed; Minecraft gameplay not independently verified.
**0.3.0 feature branch must pass CI and get a successful Windows installer before merge.**

## Next actions
1. Run 0.3.0 CI on PR and fix Rust/TS build failures.
2. Verify installing and launching a recent Java release with compatible Java runtime on Windows.
3. Add managed Java runtime automatic selection and native dependency edge cases.
4. Build full Microsoft account sign-in and integrity-tested modloader support.
