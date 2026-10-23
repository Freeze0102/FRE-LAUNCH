# FRE LAUNCH — development status

## 0.5.1 Alpha / NIGHT CONTROL (installation reliability)

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
- Launch now verifies cached client JAR, hashed libraries and asset index, and repairs missing/corrupt files instead of launching a broken game.
- Checksum-free Maven artifacts must have a valid JAR header before reuse or installation; cached HTTP error pages are rejected.
- 0.5.2: Full official Mojang client JAR and Maven library download/hash smoke tests, cached-client integrity in version status, dark WebView startup background and upgraded GitHub Actions runtime.
- Modrinth Fabric mods and ZIP resource-pack downloads now enforce streaming byte limits, even if the remote server omits Content-Length, avoiding uncontrolled in-memory buffering.

## 0.5.3 / STARTUP RECOVERY + IN-APP UPDATER

- WebView2: a persistent bootstrap marker detects early process shutdown and uses software compositing on the next launch; marker clears after 20 seconds of responsive UI.
- Zustand persistence keeps player profiles, selected release, RAM and theme but always starts at Home rather than reopening the last crash-prone page.
- Skin Studio / Three.js are lazy-loaded, Java detection runs on a blocking worker.
- Native Rust update backend checks GitHub Releases, validates explicit version and trusted installer origin, downloads with a strict size limit, verifies release SHA-256, and opens an installer **only after user consent**.
- A small update center checks for releases after initial render, shows progress and allows user/admin installation modes.
- Successful main Windows Builds publish GitHub Releases with installer SHA-256 files. Releases must use new SemVer; old releases are immutable.
- Limits: SHA-256 from GitHub Releases is not a separate publisher signature, and actual startup crash resolution on the user's Windows machine is not yet verified.


## 0.5.4 / STARTUP PROBE + PUBLIC RELEASE CHANNEL

- Write early Rust/Tauri/WebView2 breadcrumbs to `%LOCALAPPDATA%\FRE-LAUNCH\startup.log`, even if the UI never opens; diagnostics includes this log. The WebView2 recovery profile added in 0.5.3 remains intact.
- Windows CI checks that the installed executable survives 12 seconds, not only that NSIS copied it into the selected directory. A runner smoke test cannot prove that crashes on the user's Windows GPU are fixed.
- Stage current-user and all-user installers separately by removing the first NSIS output before building the administrator variant.
- For anonymous, in-app updates while the source repository stays private, create an existing **public Releases-only repository** with a main branch, set Actions variable `FRE_LAUNCH_RELEASE_REPO` to `owner/repo` and Actions secret `FRE_RELEASE_TOKEN` with Contents write permission on that public repository. The token is never shipped inside the application.
- Without the public distribution repository, update checks report the private/missing release feed clearly instead of incorrectly saying the client is current. Existing installed versions without an updater still require one manual installation.

## 0.5.5 / Build repair + launcher-style UI

- Fix Rust compile error E0425 in `src-tauri/src/lib.rs`: bind Tauri's event-loop result before logging its exit.
- Replace oversized Home marketing hero and heavy WorldArt import with a focused Play dashboard: selected Minecraft version, active instance, launch button, Java/RAM/account controls, recent official releases, and useful library shortcuts.
- Simplify Ukrainian sidebar navigation, typography, rounded card surfaces and Modrinth-inspired dark/green layout without copying third-party assets.
- Remove empty motivational slogans, including the crash-screen headline; use functional error/help messages instead.
- Maintain existing install, logs, Java manager, update center, skins, backups and instance functionality; version bumped to 0.5.5 for a fresh release.
- CI and Windows executable survival test required before declaring the new build stable. Private GitHub Releases still cannot serve anonymous in-app updates unless public release distribution is configured.

## 0.5.5 startup diagnosis after Windows CI smoke failure

- The NSIS build, custom-folder installation and Rust tests passed, but running the installed binary in the Windows runner terminated with `0xC00000FD` (stack overflow) after `TAURI_SETUP`. The build therefore has NOT been approved for release.
- Record a frontend-mounted breadcrumb to separate WebView2/native failure from UI initialization.
- The updated workflow tests both normal startup and forced software-rendering recovery, prints launcher startup.log and Windows Application crash events, and blocks release if either startup fails. No silent suppression of the crash.

## 0.5.6 / Windows stack overflow mitigation — security hold stays active

- Windows CI run 39985988909 confirmed a real native `0xC00000FD` stack overflow after React mounted, WebView2 requested a resource, and the Mojang manifest returned. The captured ProcDump/WinDbg stack includes `EmbeddedBrowserWebView!FireWebResourceRequestedEvent`; the process stack reached its default 1 MiB reserve.
- Increase the Windows executable main-thread stack reserve to 8 MiB using a Cargo build-script linker argument. Virtual reservation does not mean 8 MiB of immediately committed memory. Add a PE-header CI assertion and retain the 12-second normal/recovery-mode startup checks.
- This is a hypothesis-driven crash fix to validate on the Windows runner; do not claim the crash is resolved until runtime smoke tests pass.
- Version 0.5.6 builds remain subject to Microsoft Defender antivirus checks, and the published-release / downloadable-artifact approval gate remains closed because v0.5.5 administrator installer was detected as Trojan:Win32/Bearfoos.B!ml on a user PC. **Passing Defender scan on one CI runner is not sufficient to declare this incident resolved.**
