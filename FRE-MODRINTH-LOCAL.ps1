# FRE LAUNCH desktop source based on upstream GPLv3 Modrinth App.
# No GitHub Actions, no new branches. This script never changes the legacy game files.
param(
    [switch]$BuildWindows
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$revision = '4ccf4d51804627cd0687ce7dfd5004f20c267acc'
$upstream = Join-Path $projectRoot 'vendor\modrinth'
$destination = Join-Path $projectRoot 'FRE-Modrinth-Source'
$converter = Join-Path $projectRoot 'scripts\modrinth-fork\prepare.py'
if (-not (Test-Path $converter)) { throw 'Full FRE LAUNCH repository not found.' }
if (Test-Path $destination) { throw "Refusing to overwrite $destination. Rename your old generated source first." }
Get-Command git -ErrorAction Stop | Out-Null
Get-Command py -ErrorAction Stop | Out-Null
Write-Host 'Fetching pinned original Modrinth source via Git submodule...' -ForegroundColor Cyan
git -C "$projectRoot" submodule update --init --depth 1 vendor/modrinth
if ($LASTEXITCODE -ne 0) { throw 'Modrinth source fetch failed.' }
$actual = (git -C "$upstream" rev-parse HEAD).Trim()
if ($actual -ne $revision) { throw "Refusing unverified Modrinth source: $actual" }
Write-Host 'Copying upstream desktop source without modifying your old launcher...' -ForegroundColor Cyan
Copy-Item -Path $upstream -Destination $destination -Recurse -Force
$copiedGit = Join-Path $destination '.git'
if (Test-Path $copiedGit) { Remove-Item $copiedGit -Force -Recurse }
py -3 -m pip install 'Pillow>=11,<13'
if ($LASTEXITCODE -ne 0) { throw 'Pillow dependency installation failed.' }
py -3 "$converter" "$destination" --upstream-sha $revision
if ($LASTEXITCODE -ne 0) { throw 'Customization failed, source remains in FRE-Modrinth-Source for inspection.' }
Write-Host 'Verifying nickname auth, locale coverage and app identity...' -ForegroundColor Cyan
py -3 "$projectRoot\scripts\modrinth-fork\verify.py" "$destination"
if ($LASTEXITCODE -ne 0) { throw 'FRE LAUNCH verification failed; refusing Windows compilation.' }
Write-Host "FRE LAUNCH modified Modrinth GPLv3 source: $destination" -ForegroundColor Green
Write-Host 'Separate old FRE LAUNCH profiles and worlds have not been changed.'
if ($BuildWindows) {
    Get-Command pnpm -ErrorAction Stop | Out-Null
    Get-Command cargo -ErrorAction Stop | Out-Null
    Push-Location $destination
    try {
        pnpm install --frozen-lockfile
        if ($LASTEXITCODE -ne 0) { throw 'pnpm install failed' }
        pnpm --filter '@modrinth/app-frontend' run tsc:check
        if ($LASTEXITCODE -ne 0) { throw 'Vue typecheck failed' }
        pnpm --filter '@modrinth/app' tauri build --bundles nsis
        if ($LASTEXITCODE -ne 0) { throw 'Tauri build failed' }
    } finally { Pop-Location }
}
