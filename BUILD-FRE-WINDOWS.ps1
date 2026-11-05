# Build FRE LAUNCH with the MIT native game and bundled resource preparer.
# Owned Minecraft assets are prepared privately after installation.
# Never triggers GitHub Actions, uploads binaries, or touches Minecraft worlds.
param(
    [switch]$Preview,
    [string]$PythonExe = $env:FRE_LAUNCH_PYTHON
)
$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $env:OS -or $env:OS -ne "Windows_NT") {
    throw "This build needs Windows x64 with Rust MSVC and Visual Studio C++ Build Tools."
}
if (!$PythonExe) { $PythonExe = 'python' }
$rustScript = Join-Path $repo "BUILD-FRE-RUST.ps1"
$launchScript = Join-Path $repo "FRE-MODRINTH-LOCAL.ps1"
$runtime = Join-Path $repo "scripts\modrinth-fork\fre-rust-runtime"
if (!(Test-Path -LiteralPath $rustScript -PathType Leaf) -or
    !(Test-Path -LiteralPath $launchScript -PathType Leaf)) {
    throw "Open this script from a FULL updated FRE-LAUNCH repository checkout."
}
$node = Get-Command node -ErrorAction Stop
$pnpm = Get-Command pnpm.cmd -ErrorAction Stop
$rust = Get-Command cargo -ErrorAction Stop
$null = $node, $pnpm, $rust
$nodeVersion = [version]((& node -p "process.versions.node").Trim())
if ($nodeVersion -lt [version]"24.15.0") {
    throw "Node.js 24.15+ is needed; found $nodeVersion. Upgrade Node.js before building."
}
$pnpmVersion = [version]((& pnpm.cmd --version).Trim())
if ($pnpmVersion -lt [version]"12.11.2") {
    throw "pnpm 12.11.2+ is needed; found $pnpmVersion."
}
$installedRuntime = Join-Path $runtime "fre-rust-client.exe"
& $rustScript -PythonExe $PythonExe
if ($LASTEXITCODE -ne 0) { throw 'Native engine build or integrity verification failed' }
foreach ($file in @("fre-rust-client.exe", "BUILD-INFO.json", "research-lock.json",
    "LICENSE.txt", "fre-rust-prepare.exe", "crates/fre-prepare/src/lib.rs")) {
    if (!(Test-Path -LiteralPath (Join-Path $runtime $file) -PathType Leaf)) {
        throw "Incomplete native Rust runtime: $file. Back it up and rerun BUILD-FRE-RUST.ps1."
    }
}
$buildInfo = Get-Content -LiteralPath (Join-Path $runtime "BUILD-INFO.json") -Raw | ConvertFrom-Json
$calculated = (Get-FileHash -LiteralPath $installedRuntime -Algorithm SHA256).Hash.ToLowerInvariant()
if ($calculated -ne ([string]$buildInfo.game_exe_sha256).ToLowerInvariant()) {
    throw "Rust game failed SHA-256 verification. Do not create an installer."
}
$started = Get-Date
Write-Host "Building and signing FRE LAUNCH Windows installer..." -ForegroundColor Cyan
& $launchScript -BuildWindows -Preview:$Preview -PythonExe $PythonExe
if ($LASTEXITCODE -and $LASTEXITCODE -ne 0) {
    throw "FRE LAUNCH compilation failed."
}
$nsis = Join-Path $repo "FRE-Modrinth-Source\target\release\bundle\nsis"
if (!(Test-Path -LiteralPath $nsis -PathType Container)) {
    throw "No NSIS output directory found. Windows build did not complete."
}
$installers = @(Get-ChildItem -LiteralPath $nsis -File -Filter "*.exe" |
    Where-Object { $_.LastWriteTime -ge $started.AddMinutes(-1) } |
    Sort-Object LastWriteTime -Descending)
if ($installers.Count -lt 1) {
    throw "No newly compiled Windows installer was found. Old installers are not accepted."
}
$installer = $installers[0]
$signature = "$($installer.FullName).sig"
if (!$Preview -and !(Test-Path -LiteralPath $signature -PathType Leaf)) {
    throw "Missing signed updater artifact: $signature. Do not distribute this build."
}
$sha = (Get-FileHash -LiteralPath $installer.FullName -Algorithm SHA256).Hash
Write-Host ""
Write-Host "WINDOWS BUILD COMPLETED (not published)" -ForegroundColor Green
Write-Host "Installer: $($installer.FullName)"
if (!$Preview) { Write-Host "Signature: $signature" }
Write-Host "SHA-256: $sha"
Write-Host "This script did NOT publish an update and did NOT trigger GitHub Actions."
Write-Host "Minecraft assets are not included; the owner prepares them privately inside FRE LAUNCH."
