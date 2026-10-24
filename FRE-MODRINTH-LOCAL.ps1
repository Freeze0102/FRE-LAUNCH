# FRE LAUNCH desktop source fork based on official GPLv3 Modrinth App.
# Generates source only. Does not execute or download third-party game installers.
param(
    [switch]$BuildWindows
)
$ErrorActionPreference = 'Stop'
$sourceSha = '4ccf4d51804627cd0687ce7dfd5004f20c267acc'
$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$converter = Join-Path $projectRoot 'scripts\modrinth-fork\prepare.py'
$destination = Join-Path $projectRoot 'FRE-Modrinth-Source'
if (!(Test-Path $converter)) {
    throw 'Download or clone the complete FRE-LAUNCH GitHub repository first.'
}
if (Test-Path $destination) {
    throw "Folder already exists: $destination. Refusing to overwrite worlds or custom source."
}
Get-Command git -ErrorAction Stop | Out-Null
Get-Command py -ErrorAction Stop | Out-Null
Write-Host 'Cloning official Modrinth desktop source at pinned commit...' -ForegroundColor Cyan
git clone --no-checkout --filter=blob:none --depth=1 https://github.com/modrinth/code.git "$destination"
if ($LASTEXITCODE -ne 0) { throw 'Git clone failed' }
git -C "$destination" fetch --no-tags --depth=1 origin $sourceSha
if ($LASTEXITCODE -ne 0) { throw 'Pinned upstream commit fetch failed' }
git -C "$destination" checkout --detach $sourceSha
if ($LASTEXITCODE -ne 0) { throw 'Upstream checkout failed' }
Write-Host 'Generating original FRE LAUNCH branding and disabling hosting/ads...' -ForegroundColor Cyan
py -3 -m pip install 'Pillow>=11,<13'
if ($LASTEXITCODE -ne 0) { throw 'Pillow dependency installation failed' }
py -3 "$converter" "$destination" --upstream-sha $sourceSha
if ($LASTEXITCODE -ne 0) { throw 'Customization failed; original FRE LAUNCH was not changed' }
Write-Host "Modified GPLv3 source ready at $destination" -ForegroundColor Green
Write-Host 'Your existing FRE LAUNCH and Minecraft data have not been modified.'
if ($BuildWindows) {
    Get-Command pnpm -ErrorAction Stop | Out-Null
    Get-Command cargo -ErrorAction Stop | Out-Null
    Push-Location $destination
    try {
        pnpm install --frozen-lockfile
        if ($LASTEXITCODE -ne 0) { throw 'Workspace dependency installation failed' }
        pnpm --filter '@modrinth/app-frontend' run tsc:check
        if ($LASTEXITCODE -ne 0) { throw 'Type check failed' }
        pnpm --filter '@modrinth/app' tauri build --bundles nsis
        if ($LASTEXITCODE -ne 0) { throw 'Windows installer compilation failed' }
    } finally { Pop-Location }
}
