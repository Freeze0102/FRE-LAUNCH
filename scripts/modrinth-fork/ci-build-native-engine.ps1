# Shared local/CI packager. Only MIT binaries and original exporter sources.
param(
    [string]$Revision = '975c07d9151847e6af13afc4b5ab37b6bbe78ef5',
    [string]$PythonExe = 'python'
)
$ErrorActionPreference = 'Stop'
if ($Revision -cne '975c07d9151847e6af13afc4b5ab37b6bbe78ef5') { throw 'Unreviewed engine revision' }
$root = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$source = Join-Path $root 'vendor/fre-rust-source'
$runtime = Join-Path $PSScriptRoot 'fre-rust-runtime'
& $PythonExe (Join-Path $PSScriptRoot 'native_source.py') $source
if ($LASTEXITCODE -ne 0) { throw 'Vendored native source verification failed' }
$sourceMeta = Get-Content -LiteralPath (Join-Path $source 'FRE-VENDORED-SOURCE.json') -Raw | ConvertFrom-Json
$brandingScript = Join-Path $PSScriptRoot 'native_branding.py'
$brandingHash = (& $PythonExe $brandingScript --fingerprint).Trim()
if ($LASTEXITCODE -ne 0 -or $brandingHash -notmatch '^[a-f0-9]{64}$') { throw 'Invalid native branding revision' }
$source = Join-Path $root '.audit/fre-native-source/current'
& $PythonExe $brandingScript $source
if ($LASTEXITCODE -ne 0) { throw 'FRE native branding generation failed' }
$sourceInputs = @('research-lock.json', 'crates/fre-prepare/Cargo.toml', 'crates/fre-prepare/build.rs')
$sourceInputs += @(Get-ChildItem -LiteralPath (Join-Path $source 'crates/fre-prepare/src') -File |
    ForEach-Object { 'crates/fre-prepare/src/' + $_.Name })
$sourceInputs += @(Get-ChildItem -LiteralPath (Join-Path $source 'mods/fre/src/test/java/dev/frelaunch/parity') -File -Recurse |
    ForEach-Object { $_.FullName.Substring($source.Length + 1) })
if (Test-Path -LiteralPath $runtime) {
    $info = Get-Content -LiteralPath (Join-Path $runtime 'BUILD-INFO.json') -Raw | ConvertFrom-Json
    if ($info.source_commit -cne $Revision -or $info.source_tree -cne $sourceMeta.source_tree -or $info.engine_license -cne 'MIT') {
        throw 'Existing runtime provenance differs from the pinned engine; preserve it before rebuilding'
    }
    if ($info.branding_sha256 -cne $brandingHash) {
        $archive = Join-Path $PSScriptRoot ('fre-rust-runtime-archive-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
        if ([IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($runtime)) -ne $PSScriptRoot -or
            [IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($archive)) -ne $PSScriptRoot) {
            throw 'Native runtime archive must stay inside the packaging directory'
        }
        Move-Item -LiteralPath $runtime -Destination $archive
        Write-Host 'Preserved previous runtime; compiling current FRE branding.'
    }
}
if (Test-Path -LiteralPath $runtime) {
    foreach ($item in @(
        @{name='fre-rust-client.exe'; checksum='game_exe_sha256'},
        @{name='fre-rust-prepare.exe'; checksum='prepare_exe_sha256'}
    )) {
        $filename = Join-Path $runtime $item.name
        $expected = [string]$info.($item.checksum)
        if (!(Test-Path -LiteralPath $filename -PathType Leaf) -or $expected -notmatch '^[A-Fa-f0-9]{64}$' -or
            (Get-FileHash -LiteralPath $filename -Algorithm SHA256).Hash -ine $expected) {
            throw "Unverified cached native binary: $($item.name)"
        }
    }
    foreach ($relative in $sourceInputs) {
        $cached = Join-Path $runtime $relative
        if (!(Test-Path -LiteralPath $cached -PathType Leaf) -or
            (Get-FileHash -LiteralPath $cached -Algorithm SHA256).Hash -cne
            (Get-FileHash -LiteralPath (Join-Path $source $relative) -Algorithm SHA256).Hash) {
            throw "Cached exporter source differs from pinned source: $relative"
        }
    }
    if ((Get-FileHash -LiteralPath (Join-Path $runtime 'LICENSE.txt')).Hash -cne
        (Get-FileHash -LiteralPath (Join-Path $source 'LICENSE')).Hash) { throw 'Cached MIT attribution differs' }
    if (Test-Path -LiteralPath (Join-Path $runtime 'resources')) { throw 'Private Minecraft resources in redistributable runtime' }
    Write-Host 'Reused verified native game and preparer. No rebuild or upstream download needed.'
    return
}
$buildTarget = Join-Path $root 'vendor/fre-rust-source/target'
Push-Location $source
try {
    # Reuse registry dependency objects, while compiling the renamed FRE crates.
    cargo build --locked --release --target-dir $buildTarget -p fre-client -p fre-prepare
    if ($LASTEXITCODE -ne 0) { throw 'Native engine compilation failed' }
} finally { Pop-Location }
$stage = "$runtime.pending"
if (Test-Path -LiteralPath $stage) { throw 'Previous native staging exists; inspect it before retrying' }
New-Item -ItemType Directory -Path $stage | Out-Null
foreach ($item in @(
    @{source=(Join-Path $buildTarget 'release/fre-client.exe'); target='fre-rust-client.exe'},
    @{source=(Join-Path $buildTarget 'release/fre-prepare.exe'); target='fre-rust-prepare.exe'},
    @{source=(Join-Path $source 'LICENSE'); target='LICENSE.txt'}
)) {
    Copy-Item -LiteralPath $item.source -Destination (Join-Path $stage $item.target)
}
foreach ($relative in $sourceInputs) {
    $out = Join-Path $stage $relative
    New-Item -ItemType Directory -Force -Path (Split-Path $out -Parent) | Out-Null
    Copy-Item -LiteralPath (Join-Path $source $relative) -Destination $out
}
$info = [ordered]@{
    engine_source = 'https://github.com/Freeze0102/FRE-LAUNCH'
    source_commit = $Revision
    source_tree = $sourceMeta.source_tree
    engine_license = 'MIT'
    branding_sha256 = $brandingHash
    product_name = 'FRE LAUNCH'
    game_exe_sha256 = (Get-FileHash -LiteralPath (Join-Path $stage 'fre-rust-client.exe') -Algorithm SHA256).Hash.ToLowerInvariant()
    prepare_exe_sha256 = (Get-FileHash -LiteralPath (Join-Path $stage 'fre-rust-prepare.exe') -Algorithm SHA256).Hash.ToLowerInvariant()
    proprietary_minecraft_assets_included = $false
}
$info | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $stage 'BUILD-INFO.json') -Encoding utf8
Move-Item -LiteralPath $stage -Destination $runtime
Write-Host "Packaged MIT game + preparer from $Revision. No proprietary Minecraft assets."
