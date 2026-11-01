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
Push-Location $source
try {
    cargo build --locked --release -p horizon-client -p horizon-prepare
    if ($LASTEXITCODE -ne 0) { throw 'Rust engine failed to compile' }
} finally { Pop-Location }
New-Item -ItemType Directory -Path $runtime | Out-Null
$engine = Join-Path $source 'target\release\horizon-client.exe'
if (!(Test-Path $engine)) { throw 'Compiled Rust engine executable missing' }
Copy-Item -LiteralPath $engine -Destination (Join-Path $runtime 'fre-rust-client.exe')
Copy-Item -LiteralPath (Join-Path $source 'LICENSE') -Destination (Join-Path $runtime 'LICENSE.txt')
Copy-Item -LiteralPath (Join-Path $source 'research-lock.json') -Destination (Join-Path $runtime 'research-lock.json')
$hash = (Get-FileHash -LiteralPath (Join-Path $runtime 'fre-rust-client.exe') -Algorithm SHA256).Hash.ToLowerInvariant()
$metadata = [ordered]@{
    engine_source = 'https://github.com/horizon-rust/horizon'
    source_commit = $Revision
    engine_license = 'MIT'
    game_exe_sha256 = $hash
    proprietary_assets_origin = 'Not included in this public installer'
    redistribution = 'Open-source MIT engine only; import owned Minecraft assets locally'
}
$metadata | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $runtime 'BUILD-INFO.json') -Encoding UTF8
if (Test-Path -LiteralPath (Join-Path $runtime 'resources')) { throw 'Unexpected proprietary resource directory in CI output' }
Write-Host "Built pinned MIT Rust client SHA-256: $hash"
