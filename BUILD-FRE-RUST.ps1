# Builds the MIT game + resource preparer from FRE LAUNCH's vendored source.
# Minecraft assets are downloaded privately by the installed app after account verification.
param([string]$PythonExe = $env:FRE_LAUNCH_PYTHON)
$ErrorActionPreference = 'Stop'
if (!$PythonExe) { $PythonExe = 'python' }
& (Join-Path $PSScriptRoot 'scripts/modrinth-fork/ci-build-native-engine.ps1') -PythonExe $PythonExe
if ($LASTEXITCODE -ne 0) { throw 'Native engine build failed' }
Write-Host 'Native FRE LAUNCH engine and preparer ready. No Minecraft assets were packaged.'
