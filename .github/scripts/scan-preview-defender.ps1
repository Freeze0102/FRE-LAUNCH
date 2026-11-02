#Requires -Version 7.0
<#
  Scan Windows preview binaries with the actual Microsoft Defender engine.
  MpCmdRun is Microsoft's supported automation interface. Fail closed on
  missing/inactive Defender, scanner errors, any detection, file changes,
  or a missing file. In particular, exit 0 alone cannot establish safety
  if Defender remediated something: -DisableRemediation and threat checks
  prevent an infected binary from silently passing.
#>
param(
    [Parameter(Mandatory = $true)]
    [ValidateNotNullOrEmpty()]
    [string[]] $Paths
)
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false

if (-not (Get-Command Get-MpComputerStatus -ErrorAction SilentlyContinue)) {
    throw 'Microsoft Defender is not installed. Preview cannot be distributed.'
}
$status = Get-MpComputerStatus -ErrorAction Stop
if (-not $status.AMServiceEnabled -or -not $status.AntivirusEnabled) {
    throw 'Microsoft Defender Antivirus is not active. Preview cannot be distributed.'
}

$platform = Join-Path $env:ProgramData 'Microsoft/Windows Defender/Platform'
$scanner = $null
if (Test-Path -LiteralPath $platform -PathType Container) {
    $versions = @(Get-ChildItem -LiteralPath $platform -Directory |
        Sort-Object -Property Name -Descending)
    foreach ($version in $versions) {
        $candidate = Join-Path $version.FullName 'MpCmdRun.exe'
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            $scanner = $candidate
            break
        }
    }
}
if (-not $scanner) {
    $candidate = Join-Path $env:ProgramFiles 'Windows Defender/MpCmdRun.exe'
    if (Test-Path -LiteralPath $candidate -PathType Leaf) {
        $scanner = $candidate
    }
}
if (-not $scanner) {
    throw 'Microsoft Defender command-line scanner MpCmdRun.exe was not found.'
}

foreach ($path in $Paths) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Missing binary to scan: $path"
    }
    $file = (Resolve-Path -LiteralPath $path).ProviderPath
    $before = (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash

    # -DisableRemediation makes detection non-destructive. Microsoft documents
    # exit 2 for a detection requiring action OR for a scanner error: both fail.
    # -File selects exactly the supplied executable or installer archive.
    $messages = @(& $scanner -Scan -ScanType 3 -File $file -DisableRemediation 2>&1)
    $scanExit = $LASTEXITCODE
    foreach ($line in $messages) {
        Write-Host $line
    }
    if ($scanExit -ne 0) {
        throw "Microsoft Defender scan failed or detected malware (exit $scanExit) in $file"
    }
    if (-not (Test-Path -LiteralPath $file -PathType Leaf)) {
        throw "Defender removed or quarantined $file"
    }
    $after = (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash
    if ($after -ne $before) {
        throw "Executable changed while being scanned: $file"
    }
    # Catch prior or otherwise asynchronously reported detections too.
    $detections = @(Get-MpThreatDetection -ErrorAction Stop | Where-Object { $null -ne $_ })
    if ($detections.Count -gt 0) {
        throw "Defender reported $($detections.Count) malware detection(s). Preview distribution blocked."
    }
    Write-Host "Defender custom scan passed: $file (SHA-256: $after)"
}
