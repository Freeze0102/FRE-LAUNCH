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

## Installation preflight and dedicated download deck (new in main)

- Checks actual Minecraft data directory by creating, writing, syncing and deleting a test file before downloads.
- Disk errors display the actual target path, not just the application's install location.
- Logs metadata, client, library and sampled asset-download milestones for post-crash diagnosis.
- Original six-stage FRE DOWNLOAD ENGINE screen shows per-stage progress, disk-access status and links to logs.
- **Unverified on the affected Windows machine:** a green CI build alone does not prove the reported abrupt close is fixed.

## Minecraft download reliability audit (October 2026)

- Streamed Mojang downloads now retry transient HTTP/network errors and checksum mismatches up to three times without retrying local write failures.
- Disk preflight verifies at least 900 MB available on the **actual Minecraft storage volume**; records begin, success and failure breadcrumbs.
- Asset downloads use four concurrent tasks, capped JSON index size, and sparse progress events; native ZIP extraction limits file counts and expanded size.
- Unexpected interruptions are copied from persisted UI state into native diagnostics on next launch.
- The Windows pipeline now smoke-tests real Minecraft 1.21.10 Mojang metadata and a sample SHA-1 verified asset in addition to existing unit tests.
- **Limitation:** app execution and the original native crash cannot be verified with CI alone. A Windows crash report from the affected PC remains necessary for certainty.

- Diagnostics also inspects only local WER crash reports matching `fre-launch.exe` (fault module and exception signature) when Windows saved such a report; no automatic uploads.
- SVG world art is memoized to avoid unnecessary rerenders during download progress.
