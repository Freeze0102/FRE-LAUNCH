# FRE LAUNCH security notice — 23 October 2026

## Windows installer v0.5.5 under review

A user reported Microsoft Defender detecting **Trojan:Win32/Bearfoos.B!ml** on `FRE-LAUNCH-Administrator-Setup.exe` after starting the installer with administrator privileges. We have **not** confirmed whether the reported sample was a false positive or contained malware. A green GitHub Actions build does not mean the installer has passed a malware review.

**Do not download, install, run, restore from quarantine, or add an antivirus exception for v0.5.5.** The v0.5.5 release has been placed on security hold pending an independent analysis of the exact binary.

If you have executed the installer:

1. Keep the detected file quarantined and do not start FRE LAUNCH again until reviewed.
2. Update Microsoft Defender protection definitions, then run a full scan and an offline scan from Windows Security. Save documents before the offline restart.
3. Review Windows Security → Protection history. If additional malware is found, consider the PC compromised until it has been professionally cleaned; use another trusted device to secure important accounts.
4. Do not upload your quarantined file publicly or disable real-time protection.

### What changed in CI

- A dedicated job attempts to mark the flagged v0.5.5 release as **draft** so it is not available through normal releases.
- Windows CI now requires an active Microsoft Defender engine. It updates definitions and scans the launcher executable, non-admin installer and admin installer; any detection blocks the build.
- Production npm dependencies are audited for known high/critical vulnerabilities.
- Public installer releases are withheld unless the GitHub Actions variable `FRE_LAUNCH_SECURITY_APPROVED` has deliberately been set to `true` **after** incident review. A private GitHub Actions artifact named `FRE-LAUNCH-0.5.6-SECURITY-REVIEW-ONLY` may contain the runner-scanned v0.5.6 binaries together with SHA256 checksums, **for independent security analysis only and not for installation**. This variable alone does not guarantee safety; an antivirus scan, binary provenance and publisher authentication are all separate checks.
- Code signing and independent external AV verification are still **not implemented**; therefore these builds are **not attested to be free of malware**.

### Verify the exact file

The v0.5.5 administrator installer uploaded on GitHub has GitHub-reported SHA-256:
`c49ce8c30705e063f50ec0eac306fad26247578c194e4d0ef9c6c81b07f471c6`.

This is **not yet verified to match the user's quarantined file**. Do not restore the quarantined file just to compute a hash. A hash match would establish identity with the published artifact, **not** safety.

The publisher can submit the original trusted-source build for Microsoft's suspected false-positive investigation at https://www.microsoft.com/wdsi/filesubmission, and compare results with an independent scanner. Do not weaken security rules to make the warning disappear.

### Status

- Root cause of the Defender alert: **not determined**.
- v0.5.5: **security hold — not approved for installation**.
- A new reviewed build must have a **new version and new SHA-256**; do not silently replace old assets.

### v0.5.6 development status

- Windows CI run `39999387669` completed successfully: compiled the executable, checked an 8 MiB Windows main-thread stack reserve, tested installation plus 12-second normal and recovery startup, and scanned both installer flavors with Microsoft Defender.
- Its installer artifacts were absent only because all uploads were explicitly gated by the security-approval variable. A new CI artifact is restricted to the **private** GitHub source repository, clearly labelled `SECURITY-REVIEW-ONLY`, and includes SHA-256 hashes and warning text. **This is not clearance to install it**.
- Public GitHub Release publication is still blocked. Do not change `FRE_LAUNCH_SECURITY_APPROVED` until the incident has been investigated and a distribution decision is documented.

### v0.5.7 status

- Private GitHub Actions security-review ZIPs for v0.5.7 are explicitly labeled `SECURITY-REVIEW-ONLY` and are **not installation approval**. The v0.5.5 malware alert has not been independently classified. Public release and silent self-update publication remain on hold.

### v0.5.8 local Modrinth-fork review — 1 November 2026

- The current Vue/Tauri fork now has a separate signed updater channel. Its
  Minisign signature authenticates updater bytes; it is **not Windows
  Authenticode publisher signing or malware clearance**.
- The exact pinned source, Python patches, strict frontend build and native
  regressions have been checked locally. Production npm audit reports zero
  known advisories; the desktop dependency closure including development tools
  has zero high/critical and two open moderate advisories. See
  [AUDIT_UA.md](AUDIT_UA.md) and
  [SECURITY_DEPENDENCIES.md](scripts/modrinth-fork/SECURITY_DEPENDENCIES.md).
- The build, signature and Defender results for the exact new artifacts are
  recorded in the audit report. A clean local scan does not independently
  classify the reported v0.5.5 sample. The previous incident review and an
  isolated Windows install/update/restart test remain release requirements.
- No production release or silent-update manifest has been published in this
  local review. The existing installed launcher, profiles and worlds were not
  used for testing. No GitHub Actions were run.

### v0.5.9 — maintainer-requested update delivery test

The maintainer has requested testing the signed update from their installed
0.5.8 to 0.5.9. This is a **separate newly built artifact**, from source commit
`b83f78f822612d6a288f172c89686bdc38a4720f`, with the same updater key and app
identity as 0.5.8. The local Vue/Rust/NSIS build, regression gates, PE checks,
actual updater signature and Defender scans have passed. The draft's uploaded
asset SHA-256 digests were checked against these exact local files.

Distribution decision: release 0.5.9 for this explicitly requested update
delivery test, with its source, checksums, scan provenance and limitations
published together. This is not independent external AV clearance, Windows
Authenticode signing, or a claim that installation/restart has already been
verified on the maintainer's installed client. That transition remains the
next test; it must be reported separately from successful download/signature
verification. No user profiles or worlds are used by the automated tests.

The v0.5.5 alert is **still unclassified**. Its release remains a draft/on hold;
its binaries must not be restored or reused. This decision for newly reviewed
0.5.9 does not reclassify that sample or approve the old 0.5.6/0.5.7 review ZIPs.
The legacy CI approval variable and security protections were not changed;
GitHub Actions were not run. See the exact 0.5.9 results in [AUDIT_UA.md](AUDIT_UA.md).
