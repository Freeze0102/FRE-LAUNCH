# Build only the redistributable open-source Rust gameplay engine.
# Never include proprietary Mojang assets or private Minecraft resources in CI.
param([string]$Revision = '975c07d9151847e6af13afc4b5ab37b6bbe78ef5')
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$source = Join-Path $root 'vendor\fre-rust-source'
$runtime = Join-Path $root 'scripts\modrinth-fork\fre-rust-runtime'
if (Test-Path -LiteralPath $source) { throw 'Rust source checkout already exists; refusing to overwrite it' }
if (Test-Path -LiteralPath $runtime) { throw 'Rust runtime already exists; refusing to overwrite it' }
git clone --filter=blob:none --no-checkout https://github.com/horizon-rust/horizon.git $source
if ($LASTEXITCODE -ne 0) { throw 'Could not fetch open-source Rust engine' }
git -C $source checkout --detach $Revision
if ($LASTEXITCODE -ne 0) { throw 'Could not resolve pinned source revision' }
$actual = (git -C $source rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $actual -ne $Revision) { throw "Invalid Rust engine revision: $actual" }
# Reject a pinned engine checkout missing the verified gameplay branches.
# This is not a substitute for runtime parity testing on Windows.
$mechanics = @(
    @('crates\horizon-client\src\game\beds.rs', 'destroy_straw_bed'),
    @('crates\horizon-client\src\game\remote.rs', 'release_use_for_keybind_screen'),
    @('crates\horizon-world\src\level\interact.rs', 'Class::Note'),
    @('crates\horizon-world\src\level\redstone.rs', 'note_play'),
    @('crates\horizon-world\src\level\redstone.rs', 'note_trigger_event'),
    @('crates\horizon-world\src\level\redstone.rs', 'note_neighbor_changed')
)
foreach ($check in $mechanics) {
    $file = Join-Path $source $check[0]
    if (!(Test-Path -LiteralPath $file -PathType Leaf) -or
        !(Select-String -LiteralPath $file -SimpleMatch -Quiet -Pattern $check[1])) {
        throw "Pinned Rust gameplay contract changed: $($check[0]) missing $($check[1])."
    }
}
Push-Location $source
try {
    cargo build --locked --release -p horizon-client -p horizon-prepare
    if ($LASTEXITCODE -ne 0) { throw 'Rust engine failed to compile' }
} finally { Pop-Location }
New-Item -ItemType Directory -Path $runtime | Out-Null
$engine = Join-Path $source 'target\release\horizon-client.exe'
if (!(Test-Path $engine)) { throw 'Compiled Rust engine executable missing' }
Copy-Item -LiteralPath $engine -Destination (Join-Path $runtime 'fre-rust-client.exe')
# Include the MIT resource-preparation BINARY and original exporter sources.
# Both operate inside FRE LAUNCH. No Horizon Launcher or Microsoft login
# application is downloaded, started, or required by the native Rust client.
$preparer = Join-Path $source 'target\release\horizon-prepare.exe'
if (!(Test-Path -LiteralPath $preparer -PathType Leaf)) { throw 'Native Rust preparer missing from pinned source build' }
Copy-Item -LiteralPath $preparer -Destination (Join-Path $runtime 'fre-rust-prepare.exe')
$paritySource = Join-Path $source 'mods\horizon\src\test\java\dev\horizonlab\parity'
$parityDestination = Join-Path $runtime 'mods\horizon\src\test\java\dev\horizonlab\parity'
if (!(Test-Path -LiteralPath (Join-Path $paritySource 'VanillaHeightmapOracle.java'))) {
    throw 'Native Rust exporter source missing from pinned checkout'
}
New-Item -ItemType Directory -Force -Path $parityDestination | Out-Null
Copy-Item -Path (Join-Path $paritySource '*') -Destination $parityDestination -Recurse -Force
# The preparer verifies the complete crate source digest, including its
# Cargo.toml, build.rs and every src/*.rs. An isolated exporters.rs is not
# sufficient; it can force a needless rebuild or break resource verification.
$prepareSource = Join-Path $source 'crates\horizon-prepare'
$prepareDestination = Join-Path $runtime 'crates\horizon-prepare'
New-Item -ItemType Directory -Force -Path (Join-Path $prepareDestination 'src') | Out-Null
foreach ($name in @('Cargo.toml', 'build.rs')) {
    Copy-Item -LiteralPath (Join-Path $prepareSource $name) -Destination (Join-Path $prepareDestination $name)
}
Copy-Item -Path (Join-Path $prepareSource 'src\*.rs') -Destination (Join-Path $prepareDestination 'src') -Force
$sourceCount = @(Get-ChildItem -LiteralPath (Join-Path $prepareDestination 'src') -File -Filter '*.rs').Count
if ($sourceCount -lt 10) { throw "Native preparer source snapshot is incomplete: $sourceCount Rust files" }
Copy-Item -LiteralPath (Join-Path $source 'LICENSE') -Destination (Join-Path $runtime 'LICENSE.txt')
Copy-Item -LiteralPath (Join-Path $source 'research-lock.json') -Destination (Join-Path $runtime 'research-lock.json')
$hash = (Get-FileHash -LiteralPath (Join-Path $runtime 'fre-rust-client.exe') -Algorithm SHA256).Hash.ToLowerInvariant()
$prepareHash = (Get-FileHash -LiteralPath (Join-Path $runtime 'fre-rust-prepare.exe') -Algorithm SHA256).Hash.ToLowerInvariant()
$metadata = [ordered]@{
    engine_source = 'https://github.com/horizon-rust/horizon'
    source_commit = $Revision
    engine_license = 'MIT'
    game_exe_sha256 = $hash
    prepare_exe_sha256 = $prepareHash
    proprietary_assets_origin = 'Not included in this public installer'
    redistribution = 'Open-source MIT game + preparer; fetch owned Minecraft assets locally in FRE LAUNCH'
}
$metadata | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $runtime 'BUILD-INFO.json') -Encoding UTF8
if (Test-Path -LiteralPath (Join-Path $runtime 'resources')) { throw 'Unexpected proprietary resource directory in CI output' }
Write-Host "Built pinned MIT Rust client SHA-256: $hash"
Write-Host "Bundled independent MIT resource preparer SHA-256: $prepareHash"
