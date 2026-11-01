# Build FRE LAUNCH's embedded Rust Minecraft client LOCALLY.
# Open-source Rust gameplay implementation: MIT license; retain attribution.
# Proprietary Minecraft resources are imported only from YOUR installed game.
# This script never invokes Horizon Launcher, GitHub Actions, or changes your worlds.
param(
    [Parameter(Mandatory = $true)]
    [string]$MinecraftDir,
    [string]$SourceDir = '',
    [string]$ServerJar = ''
)
$ErrorActionPreference = 'Stop'
$project = Split-Path -Parent $MyInvocation.MyCommand.Path
$revision = '975c07d9151847e6af13afc4b5ab37b6bbe78ef5'
if (-not $SourceDir) { $SourceDir = Join-Path $project 'vendor\fre-rust-source' }
$SourceDir = [System.IO.Path]::GetFullPath($SourceDir)
$MinecraftDir = [System.IO.Path]::GetFullPath($MinecraftDir)
$target = Join-Path $project 'scripts\modrinth-fork\fre-rust-runtime'
$staging = "$target.pending"

if (-not (Test-Path -LiteralPath $MinecraftDir -PathType Container)) {
    throw "Minecraft installation directory not found: $MinecraftDir. Install Minecraft Java 26.3 and run it once."
}
Get-Command git -ErrorAction Stop | Out-Null
Get-Command cargo -ErrorAction Stop | Out-Null

if (-not (Test-Path -LiteralPath $SourceDir)) {
    Write-Host 'Getting pinned MIT-licensed Rust game source (not a launcher)...'
    & git clone --no-checkout https://github.com/horizon-rust/horizon.git $SourceDir
    if ($LASTEXITCODE -ne 0) { throw 'Unable to clone open-source game source.' }
    & git -C $SourceDir checkout --detach $revision
    if ($LASTEXITCODE -ne 0) { throw 'Unable to check out pinned game source.' }
}
$actual = (& git -C $SourceDir rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or $actual -ne $revision) {
    throw "Wrong Rust engine revision: $actual. Expected $revision. Refusing to build unknown code."
}
foreach ($relative in @('LICENSE', 'research-lock.json', 'crates\horizon-client\Cargo.toml',
    'crates\horizon-prepare\Cargo.toml')) {
    if (-not (Test-Path -LiteralPath (Join-Path $SourceDir $relative))) {
        throw "Incomplete open-source checkout: $relative"
    }
}

Write-Host 'Compiling native game and private-resource preparer (no external launcher)...'
Push-Location $SourceDir
try {
    & cargo build --locked --release -p horizon-client -p horizon-prepare
    if ($LASTEXITCODE -ne 0) { throw 'Rust client compilation failed.' }
} finally { Pop-Location }

$client = Join-Path $SourceDir 'target\release\horizon-client.exe'
$preparer = Join-Path $SourceDir 'target\release\horizon-prepare.exe'
if (-not (Test-Path -LiteralPath $client) -or -not (Test-Path -LiteralPath $preparer)) {
    throw 'Compiled Rust game binaries are missing.'
}

$resources = Join-Path $SourceDir '.local\horizon-client\26.3'
$argsPrepare = @('--prepare', '--complete', '--root', $SourceDir,
    '--minecraft-dir', $MinecraftDir, '--output', $resources)
if ($ServerJar) {
    $ServerJar = [System.IO.Path]::GetFullPath($ServerJar)
    if (-not (Test-Path -LiteralPath $ServerJar -PathType Leaf)) {
        throw "Server JAR not found: $ServerJar"
    }
    $argsPrepare += @('--server-jar', $ServerJar)
}
Write-Host 'Preparing PRIVATE resources from your own Minecraft 26.3 files...'
& $preparer @argsPrepare
if ($LASTEXITCODE -ne 0) {
    throw 'Resource preparation failed. Verify installed Minecraft Java 26.3, Java and the pinned server JAR.'
}
& $preparer --check --root $SourceDir --output $resources
if ($LASTEXITCODE -ne 0) { throw 'Resource validation failed; runtime will not be bundled.' }
if (-not (Test-Path -LiteralPath (Join-Path $resources 'horizon-resources.json'))) {
    throw 'Prepared resource manifest is missing.'
}

if (Test-Path -LiteralPath $staging) {
    throw "Staging already exists ($staging). Inspect and remove this temporary build output manually."
}
New-Item -ItemType Directory -Path $staging | Out-Null
try {
    Copy-Item -LiteralPath $client -Destination (Join-Path $staging 'fre-rust-client.exe')
    Copy-Item -LiteralPath (Join-Path $SourceDir 'LICENSE') -Destination (Join-Path $staging 'LICENSE.txt')
    Copy-Item -LiteralPath (Join-Path $SourceDir 'research-lock.json') -Destination (Join-Path $staging 'research-lock.json')
    Copy-Item -LiteralPath $resources -Destination (Join-Path $staging 'resources') -Recurse
    $hash = (Get-FileHash (Join-Path $staging 'fre-rust-client.exe') -Algorithm SHA256).Hash.ToLowerInvariant()
    @{
        engine_source = 'https://github.com/horizon-rust/horizon'
        source_commit = $revision
        engine_license = 'MIT'
        game_exe_sha256 = $hash
        proprietary_assets_origin = 'User-provided local Minecraft Java Edition installation'
        redistribution = 'Do not publicly redistribute the prepared proprietary Minecraft assets'
    } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $staging 'BUILD-INFO.json') -Encoding UTF8
    if (Test-Path -LiteralPath $target) {
        throw "An existing private Rust runtime is present at $target. Remove it manually after making a backup if you intend to replace it."
    }
    Move-Item -LiteralPath $staging -Destination $target
} catch {
    # Only remove our temporary staging folder, not game saves or user data.
    if (Test-Path -LiteralPath $staging) { Remove-Item -LiteralPath $staging -Recurse -Force }
    throw
}
Write-Host "FRE LAUNCH native Rust Minecraft runtime prepared: $target"
Write-Host 'Build the FRE LAUNCH Windows installer with .\FRE-MODRINTH-LOCAL.ps1 -BuildWindows'
Write-Host 'Personal local build only: do not publish the installer with Minecraft proprietary assets.'
