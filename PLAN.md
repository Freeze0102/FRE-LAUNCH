# FRE LAUNCH development roadmap

- [x] Base Tauri/React/Rust UI and GitHub Windows installer.
- [x] Official Minecraft version discovery.
- [x] Vanilla client/library/resource installer and preliminary offline launch engine.
- [x] Live logs, real progress and SHA-1 verification.
- [x] SQLite database for separated instances, preferences and per-instance Java/RAM.
- [x] Verified resource pack installation and Modrinth discovery.
- [x] Interactive skin preview and imported PNG capes.
- [x] Persistent UI color themes.
- [x] SQLite offline account manager with multiple local nicknames.
- [x] ZIP world backups with non-overwriting restoration.
- [x] Discover installed Java runtimes and configure each instance.
- [ ] Verify Windows 0.3.0 CI with tests; fix all failures.
- [ ] Verify actual game launch on a Windows PC and cover older versions.
- [ ] Java runtime download and automatic path selection.
- [ ] Microsoft OAuth and Minecraft ownership checks.
- [ ] Fabric/Forge/NeoForge/Quilt and compatible mod installation.
- [x] Server library and status ping (no SRV DNS or direct-connect yet).
- [x] Safe instance-specific screenshot gallery with preview and delete.
- [ ] Updater, complete translations, logging/privacy hardening.
- [ ] Release verified stable installer.

Current policy: never claim runtime verification solely because GitHub Actions builds the executable.
