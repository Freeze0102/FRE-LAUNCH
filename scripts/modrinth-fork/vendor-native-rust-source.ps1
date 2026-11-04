# One-time complete MIT source import. Subsequent builds use only this repository.
param(
    [string]$Revision = '975c07d9151847e6af13afc4b5ab37b6bbe78ef5',
    [string]$PythonExe = 'python'
)
$ErrorActionPreference = 'Stop'
if ($Revision -cne '975c07d9151847e6af13afc4b5ab37b6bbe78ef5') { throw 'Unreviewed source revision' }
$repo = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$destination = Join-Path $repo 'vendor/fre-rust-source'
if (Test-Path -LiteralPath $destination) { throw 'Source destination already exists; no files were overwritten' }
$stage = Join-Path $repo ('.audit/source-import-' + [Guid]::NewGuid().ToString('N'))
try {
    git clone --filter=blob:none --no-checkout https://github.com/horizon-rust/horizon.git $stage
    if ($LASTEXITCODE -ne 0) { throw 'Source download failed' }
    git -C $stage checkout --detach $Revision
    if ($LASTEXITCODE -ne 0) { throw 'Pinned source checkout failed' }
    & $PythonExe (Join-Path $PSScriptRoot 'native_source.py') $destination --import-checkout $stage
    if ($LASTEXITCODE -ne 0) { throw 'Complete source import or Git tree verification failed' }
} finally {
    $resolved = [IO.Path]::GetFullPath($stage)
    if (!$resolved.StartsWith((Join-Path $repo '.audit') + [IO.Path]::DirectorySeparatorChar)) {
        throw 'Temporary source path escaped repository'
    }
    if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
