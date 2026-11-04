# One-time import of the complete pinned MIT Rust Minecraft game into FRE LAUNCH.
# After committing vendor/fre-rust-source, builds do not clone Horizon upstream.
# This imports OPEN SOURCE only; never Mojang jars, login tokens, or game caches.
# Does not run GitHub Actions, build a program, or push/commit on the user's behalf.
param([string]$Revision = '975c07d9151847e6af13afc4b5ab37b6bbe78ef5')
$ErrorActionPreference = 'Stop'
$expected = '975c07d9151847e6af13afc4b5ab37b6bbe78ef5'
if ($Revision -cne $expected) { throw 'Unreviewed Rust source revision. Update pinning and security checks before using another commit.' }
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$destination = Join-Path $repo 'vendor\fre-rust-source'
if (Test-Path -LiteralPath $destination) {
    throw 'Source destination already exists. No file will be overwritten or deleted.'
}
if (!(Get-Command git -ErrorAction SilentlyContinue) -or !(Get-Command tar.exe -ErrorAction SilentlyContinue)) {
    throw 'Requires Git and the Windows tar.exe command. No separate game launcher is needed.'
}
$stage = Join-Path $env:TEMP ('FRE-Native-Rust-Snapshot-' + [Guid]::NewGuid().ToString('N'))
try {
    git clone --filter=blob:none --no-checkout https://github.com/horizon-rust/horizon.git $stage
    if ($LASTEXITCODE -ne 0) { throw 'Could not retrieve the pinned MIT source snapshot' }
    git -C $stage checkout --detach $Revision
    if ($LASTEXITCODE -ne 0) { throw 'Could not select the approved source revision' }
    $actual = (git -C $stage rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $actual -cne $expected) { throw "Unexpected source revision: $actual" }
    $license = Get-Content -LiteralPath (Join-Path $stage 'LICENSE') -Raw
    if ($license -notmatch 'MIT License') { throw 'Source MIT license did not match the approved import' }
    $archive = Join-Path $stage 'FRE-Rust-Source.tar'
    git -C $stage archive --format=tar -o $archive $Revision
    if ($LASTEXITCODE -ne 0 -or !(Test-Path -LiteralPath $archive -PathType Leaf)) {
        throw 'Failed to create a complete pinned source archive'
    }
    New-Item -ItemType Directory -Force -Path $destination | Out-Null
    & tar.exe -xf $archive -C $destination
    if ($LASTEXITCODE -ne 0) { throw 'Could not unpack pinned Rust source. Check destination before retrying.' }
    foreach ($required in @(
        'Cargo.toml', 'Cargo.lock', 'LICENSE', 'research-lock.json',
        'crates/horizon-client/src/main.rs', 'crates/horizon-prepare/src/main.rs',
        'mods/horizon/src/test/java/dev/horizonlab/parity/VanillaHeightmapOracle.java'
    )) {
        if (!(Test-Path -LiteralPath (Join-Path $destination $required) -PathType Leaf)) {
            throw "The native source snapshot is incomplete: $required"
        }
    }
    $files = @(Get-ChildItem -LiteralPath $destination -Recurse -File)
    if ($files.Count -lt 2500) { throw "Native source snapshot incomplete: $($files.Count) files" }
    $manifest = [ordered]@{
        source_commit = $expected
        engine_license = 'MIT'
        source_origin = 'https://github.com/horizon-rust/horizon'
        imported_by = 'FRE LAUNCH native Rust source vendor tool'
        proprietary_minecraft_assets_included = $false
    }
    $manifest | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $destination 'FRE-VENDORED-SOURCE.json') -Encoding UTF8
    Write-Host "Pinned MIT Rust Minecraft source imported into $destination"
    Write-Host "Tracked files: $($files.Count). No game assets and no foreign launcher application were installed."
    Write-Host 'When satisfied, commit vendor/fre-rust-source as part of your FRE LAUNCH Git repository.'
    Write-Host 'Future native Windows builds reuse this local source without contacting Horizon.'
} finally {
    if (Test-Path -LiteralPath $stage) {
        Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue
    }
}
