# Build a PRIVATE FRE LAUNCH Windows installer with the native Rust game.
# Runs on the owner's Windows PC using a locally owned Minecraft Java copy.
# Never triggers GitHub Actions, uploads binaries, or touches Minecraft worlds.
param(
    [string]$MinecraftDir = "",
    [string]$ServerJar = ""
)
$ErrorActionPreference = "Stop"
$repo = Split-Path -Parent $MyInvocation.MyCommand.Path
if (-not $env:OS -or $env:OS -ne "Windows_NT") {
    throw "This build needs Windows x64 with Rust MSVC and Visual Studio C++ Build Tools."
}
if (-not $MinecraftDir) {
    $MinecraftDir = Join-Path $env:APPDATA ".minecraft"
}
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
$hasRuntime = Test-Path -LiteralPath $runtime -PathType Container
if (-not $hasRuntime) {
    Write-Host "Preparing the Rust client and your private Minecraft resources..." -ForegroundColor Cyan
    $rustArgs = @{ MinecraftDir = $MinecraftDir }
    if ($ServerJar) { $rustArgs.ServerJar = $ServerJar }
    & $rustScript @rustArgs
    if ($LASTEXITCODE -and $LASTEXITCODE -ne 0) {
        throw "Rust game setup failed."
    }
} else {
    Write-Host "Found an existing private Rust runtime; checking it, not overwriting it." -ForegroundColor Cyan
}
foreach ($file in @("fre-rust-client.exe", "BUILD-INFO.json", "research-lock.json",
    "LICENSE.txt", "resources\horizon-resources.json")) {
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
& $launchScript -BuildWindows
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
if (!(Test-Path -LiteralPath $signature -PathType Leaf)) {
    throw "Missing signed updater artifact: $signature. Do not distribute this build."
}
$sha = (Get-FileHash -LiteralPath $installer.FullName -Algorithm SHA256).Hash
Write-Host ""
Write-Host "PRIVATE WINDOWS BUILD COMPLETED" -ForegroundColor Green
Write-Host "Installer: $($installer.FullName)"
Write-Host "Signature: $signature"
Write-Host "SHA-256: $sha"
Write-Host "This script did NOT publish an update and did NOT trigger GitHub Actions."
Write-Host "Do NOT publish a build containing private Minecraft game assets."
