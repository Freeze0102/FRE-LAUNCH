# FRE LAUNCH desktop source based on upstream GPLv3 Modrinth App.
# No GitHub Actions, no new branches. This script never changes the legacy game files.
param(
    [switch]$BuildWindows,
    [switch]$RunDev,
    [string]$PythonExe = $env:FRE_LAUNCH_PYTHON
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$revision = '4ccf4d51804627cd0687ce7dfd5004f20c267acc'
$upstream = Join-Path $projectRoot 'vendor\modrinth'
$destination = Join-Path $projectRoot 'FRE-Modrinth-Source'
$converter = Join-Path $projectRoot 'scripts\modrinth-fork\prepare.py'
if (-not (Test-Path $converter)) { throw 'Full FRE LAUNCH repository not found.' }
$pythonCommand = $null
$pythonArguments = @()
$pythonCandidates = if ($PythonExe) {
    @(@{ Command = $PythonExe; Arguments = @() })
} else {
    @(@{ Command = 'py'; Arguments = @('-3') }, @{ Command = 'python'; Arguments = @() })
}
foreach ($candidate in $pythonCandidates) {
    try {
        $candidateCommand = (Get-Command $candidate.Command -ErrorAction Stop).Source
        $candidateArguments = $candidate.Arguments
        $pythonMajor = & $candidateCommand @candidateArguments -c 'import sys; print(sys.version_info.major)' 2>$null
        if ($LASTEXITCODE -eq 0 -and "$pythonMajor".Trim() -eq '3') {
            $pythonCommand = $candidateCommand
            $pythonArguments = $candidateArguments
            break
        }
    } catch { }
}
if (-not $pythonCommand) { throw 'Working Python 3 not found. Install Python 3 or pass -PythonExe PATH_TO_PYTHON.exe.' }
if ($BuildWindows -and $RunDev) { throw 'Choose either -BuildWindows or -RunDev.' }
if ($BuildWindows -or $RunDev) {
    Get-Command node -ErrorAction Stop | Out-Null
    Get-Command pnpm.cmd -ErrorAction Stop | Out-Null
    Get-Command cargo -ErrorAction Stop | Out-Null
    $nodeVersionText = (& node -p "process.versions.node").Trim()
    if ($LASTEXITCODE -ne 0 -or [version]$nodeVersionText -lt [version]'24.15.0') {
        throw "Original Modrinth code requires Node.js 24.15+. Detected $nodeVersionText. Upgrade Node.js before building."
    }
    $pnpmVersionText = (& pnpm.cmd --version).Trim()
    if ($LASTEXITCODE -ne 0 -or [version]$pnpmVersionText -lt [version]'12.11.2') {
        throw "Original Modrinth code requires pnpm 12.11.2+. Detected $pnpmVersionText. Run: npm.cmd install -g pnpm@12.11.2"
    }
}
$prepared = Join-Path $destination 'FRE_LAUNCH_FORK.md'
$signaturePath = Join-Path $destination '.fre-launch-source-signature'
# Essential files must survive between runs. In particular, damaged pnpm
# workspaces must never be treated as a usable prepared source tree.
$requiredSourceFiles = @(
    'apps\app-frontend\src\locales\en-US\index.json',
    'apps\app-frontend\src\locales\uk-UA\index.json',
    'packages\ui\src\locales\en-US\index.json',
    'packages\ui\src\locales\uk-UA\index.json',
    'apps\app-frontend\package.json',
    'apps\frontend\package.json',
    'apps\app\package.json',
    'apps\app\src\fre_update_policy.rs',
    'apps\app\src\api\fre_rust.rs',
    'apps\app\src\fre_update_channel.rs',
    'apps\app-frontend\src\helpers\fre-app-updater.ts',
    'apps\app-frontend\src\fre-compat.d.ts',
    'packages\ui\src\utils\fre-query-options.ts',
    'pnpm-workspace.yaml',
    'pnpm-lock.yaml'
)
$patchNames = @('prepare.py','feature_local_accounts.py','feature_ukrainian.py','feature_skin_notice.py','feature_offline_skins.py','feature_local_skin_preview.py','feature_ui_polish.py','feature_guest_launch.py','feature_java_nickname_lookup.py','feature_remove_modrinth_login.py','feature_brand_presence.py','feature_curseforge.py','curseforge_backend.rs','CurseForgeBrowser.vue','feature_horizon.py','horizon_backend.rs','HorizonRustPanel.vue','HorizonLibrary.vue','feature_fre_rust.py','fre_rust_backend.rs','FreRustPanel.vue','FreRustLibrary.vue','verify.py')
$patchNames += @('feature_updates.py','fre-app-updater.ts','fre-update-ui.js','fre_update_policy.rs','fre_updater_impl.rs','fre-update-test.json')
$patchNames += @('feature_frontend_types.py','frontend-type-fixes.json','fre-query-options.ts')
$patchNames += @('fre-compat.d.ts')
$patchNames += @('feature_security_dependencies.py','security-dependencies.json','security-pnpm-lock.yaml')
$patchNames += @('horizon_bootstrap.rs','horizon-setup-messages.json')
# Own native Rust engine is an optional local binary. A cached generated tree
# must be rebuilt when it becomes available or changes. It is never uploaded
# to GitHub and no CI minutes are used by this script.
$rustRuntimeFolder = Join-Path $projectRoot 'scripts\modrinth-fork\fre-rust-runtime'
$rustRuntimeInputs = @('fre-rust-client.exe', 'resources\horizon-resources.json', 'BUILD-INFO.json')
$rustRuntimeSignature = if (Test-Path -LiteralPath $rustRuntimeFolder -PathType Container) {
    ($rustRuntimeInputs | ForEach-Object {
        $file = Join-Path $rustRuntimeFolder $_
        if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
            throw "Native Rust runtime is incomplete: $file. Rebuild it with BUILD-FRE-RUST.ps1."
        }
        (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash
    }) -join ':'
} else { 'fre-rust-not-built' }

$signature = ($revision + ':' + (($patchNames | ForEach-Object {
    $sourceFile = Join-Path $projectRoot ('scripts\modrinth-fork\' + $_)
    if (-not (Test-Path $sourceFile)) { throw "Missing source patcher: $sourceFile" }
    (Get-FileHash $sourceFile -Algorithm SHA256).Hash
}) -join ':') + ':' + (Get-FileHash (Join-Path $projectRoot 'update-channel.json') -Algorithm SHA256).Hash + ':' + $rustRuntimeSignature)
if (Test-Path $destination) {
    $incompletePreparation = -not (Test-Path $prepared)
    if ($incompletePreparation) {
        Write-Warning "Preserving incomplete generated source before rebuilding: $destination"
    }
    $previousSignature = if (Test-Path $signaturePath) { (Get-Content $signaturePath -Raw).Trim() } else { '' }
    $missingSourceFiles = @($requiredSourceFiles | Where-Object {
        -not (Test-Path -LiteralPath (Join-Path $destination $_) -PathType Leaf)
    })
    if ($missingSourceFiles.Count -gt 0) {
        Write-Warning ("Generated source is incomplete: " + ($missingSourceFiles -join ', '))
        Write-Host 'Preserving the incomplete tree and regenerating from pinned upstream.' -ForegroundColor Yellow
    }
    if ($incompletePreparation -or $previousSignature -ne $signature -or $missingSourceFiles.Count -gt 0) {
        $archive = Join-Path $projectRoot ('FRE-Modrinth-Source-archive-' + (Get-Date -Format 'yyyyMMdd-HHmmss'))
        if (Test-Path $archive) { throw "Archive destination exists: $archive" }
        if ([IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($destination)) -ne $projectRoot -or
            [IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($archive)) -ne $projectRoot) {
            throw 'Generated source paths must remain inside the repository.'
        }
        Write-Host 'Archiving existing generated source before regeneration:' $archive -ForegroundColor Yellow
        Move-Item -LiteralPath $destination -Destination $archive
    } else {
        Write-Host 'Using existing generated Modrinth source. Verifying before launch...' -ForegroundColor Cyan
    }
}
if (-not (Test-Path $destination)) {
Get-Command git -ErrorAction Stop | Out-Null
Write-Host 'Fetching pinned original Modrinth source...' -ForegroundColor Cyan
$gitDirectory = Join-Path $projectRoot '.git'
if (Test-Path $gitDirectory) {
    git -C "$projectRoot" submodule update --init --depth 1 vendor/modrinth
    if ($LASTEXITCODE -ne 0) { throw 'Pinned Modrinth Git submodule fetch failed.' }
} else {
    # GitHub's "Download ZIP" excludes submodules. Download the same verified
    # upstream revision directly in that case, with no new branch or Actions.
    if (-not (Test-Path (Join-Path $upstream '.git'))) {
        if (Test-Path $upstream) {
            if (Get-ChildItem -LiteralPath $upstream -Force | Select-Object -First 1) {
                throw 'Existing vendor/modrinth directory is not a Git clone. Refusing to replace its contents.'
            }
            Remove-Item -LiteralPath $upstream -Force
        }
        New-Item -ItemType Directory -Force -Path (Split-Path $upstream -Parent) | Out-Null
        git clone --no-checkout --filter=blob:none https://github.com/modrinth/code.git "$upstream"
        if ($LASTEXITCODE -ne 0) { throw 'Cannot clone pinned Modrinth source.' }
    }
    git -C "$upstream" fetch --no-tags --depth 1 origin $revision
    if ($LASTEXITCODE -ne 0) { throw 'Cannot fetch exact upstream source revision.' }
    git -C "$upstream" checkout --detach $revision
    if ($LASTEXITCODE -ne 0) { throw 'Cannot check out pinned upstream source.' }
}
$actual = (git -C "$upstream" rev-parse HEAD).Trim()
if ($actual -ne $revision) { throw "Refusing unverified Modrinth source: $actual" }
Write-Host 'Copying upstream desktop source without modifying your old launcher...' -ForegroundColor Cyan
Copy-Item -Path $upstream -Destination $destination -Recurse -Force
$copiedGit = Join-Path $destination '.git'
if (Test-Path $copiedGit) { Remove-Item $copiedGit -Force -Recurse }
& $pythonCommand @pythonArguments -m pip install 'Pillow>=11,<13'
if ($LASTEXITCODE -ne 0) { throw 'Pillow dependency installation failed.' }
& $pythonCommand @pythonArguments "$converter" "$destination" --upstream-sha $revision
if ($LASTEXITCODE -ne 0) { throw 'Customization failed, source remains in FRE-Modrinth-Source for inspection.' }
}
$missingPreparedFiles = @($requiredSourceFiles | Where-Object {
    -not (Test-Path -LiteralPath (Join-Path $destination $_) -PathType Leaf)
})
if ($missingPreparedFiles.Count -gt 0) {
    throw ("Pinned source copy is incomplete after preparation: " + ($missingPreparedFiles -join ', '))
}
Write-Host 'Verifying nickname auth, locale coverage and app identity...' -ForegroundColor Cyan
& $pythonCommand @pythonArguments "$projectRoot\scripts\modrinth-fork\verify.py" "$destination"
if ($LASTEXITCODE -ne 0) { throw 'FRE LAUNCH verification failed; refusing Windows compilation.' }
Set-Content -LiteralPath $signaturePath -Value $signature -Encoding UTF8

# Preserve archived source and build caches. They may contain local changes or
# diagnostics; normal startup must never recursively delete a previous tree.
Write-Host "FRE LAUNCH modified Modrinth GPLv3 source: $destination" -ForegroundColor Green
Write-Host 'Separate old FRE LAUNCH profiles and worlds have not been changed.'
if ($BuildWindows -or $RunDev) {
    Push-Location $destination
    try {
        # The pinned upstream lockfile matches the full upstream workspace.
        # If an importer is missing, regenerate the incomplete source checkout
        # instead of silently rewriting the dependency graph.
        Write-Host 'Installing pinned workspace dependencies...' -ForegroundColor Cyan
        pnpm.cmd --filter '@modrinth/app...' install --frozen-lockfile
        if ($LASTEXITCODE -ne 0) { throw 'pnpm install failed' }
        $missingAfterPnpm = @($requiredSourceFiles | Where-Object {
            -not (Test-Path -LiteralPath (Join-Path $destination $_) -PathType Leaf)
        })
        if ($missingAfterPnpm.Count -gt 0) {
            throw ("Required source files disappeared during pnpm installation: " + ($missingAfterPnpm -join ', '))
        }
        # Upstream Modrinth does not commit this generated TypeScript file.
        # It must exist BEFORE checking the Vue frontend or compilation fails.
        pnpm.cmd -w i18n:coverage
        if ($LASTEXITCODE -ne 0) { throw 'Modrinth locale coverage generation failed' }
        pnpm.cmd --filter '@modrinth/app-frontend' run tsc:check
        if ($LASTEXITCODE -ne 0) { throw 'Vue typecheck failed' }
        if ($RunDev) {
            Write-Host 'Starting ORIGINAL Modrinth Vue/Tauri Minecraft launcher...' -ForegroundColor Green
            pnpm.cmd --filter '@modrinth/app' tauri dev
            if ($LASTEXITCODE -ne 0) { throw 'Tauri dev failed' }
        } else {
            $previousSigningKey = $env:TAURI_SIGNING_PRIVATE_KEY
            $localSigningKey = Join-Path $projectRoot '.fre-update-keys\frelaunch.key'
            try {
                if (-not $env:TAURI_SIGNING_PRIVATE_KEY -and (Test-Path -LiteralPath $localSigningKey)) {
                    $env:TAURI_SIGNING_PRIVATE_KEY = $localSigningKey
                }
                if (-not $env:TAURI_SIGNING_PRIVATE_KEY) {
                    throw 'Signed releases require TAURI_SIGNING_PRIVATE_KEY. See UPDATES_UA.md. Never generate a replacement key for an existing channel.'
                }
                pnpm.cmd --filter '@modrinth/app' tauri build --ci --bundles nsis
                if ($LASTEXITCODE -ne 0) { throw 'Tauri build failed' }
            } finally { $env:TAURI_SIGNING_PRIVATE_KEY = $previousSigningKey }
            Write-Host 'Windows NSIS installer generated under FRE-Modrinth-Source/target/release/bundle/nsis' -ForegroundColor Green
        }
    } finally { Pop-Location }
}
