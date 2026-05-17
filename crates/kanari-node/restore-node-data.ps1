param(
    [Parameter(Mandatory=$true)]
    [string]$BackupDir,
    [Parameter(Mandatory=$true)]
    [string]$TargetDataDir,
    [switch]$Force
)

$resolvedBackup = Resolve-Path -LiteralPath $BackupDir -ErrorAction Stop
$backupDataDir = Join-Path $resolvedBackup.Path "data"

if (-not (Test-Path $backupDataDir)) {
    Write-Host "Backup data directory not found: $backupDataDir" -ForegroundColor Red
    exit 1
}

if ((Test-Path $TargetDataDir) -and -not $Force) {
    Write-Host "Target data directory already exists. Re-run with -Force to replace it." -ForegroundColor Yellow
    exit 1
}

if (Test-Path $TargetDataDir) {
    Remove-Item -LiteralPath $TargetDataDir -Recurse -Force
}

New-Item -ItemType Directory -Path $TargetDataDir -Force | Out-Null
Copy-Item -Path (Join-Path $backupDataDir '*') -Destination $TargetDataDir -Recurse -Force

Write-Host "Restore completed to $TargetDataDir" -ForegroundColor Green
