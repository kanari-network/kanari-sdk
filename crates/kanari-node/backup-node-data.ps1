param(
    [Parameter(Mandatory=$true)]
    [string]$SourceDataDir,
    [string]$BackupRoot = "$env:USERPROFILE\.kanari\backups",
    [string]$Label = "node-backup"
)

$resolvedSource = Resolve-Path -LiteralPath $SourceDataDir -ErrorAction Stop
$sourceRoot = $resolvedSource.Path
$skippedFiles = @()

if (-not (Test-Path $BackupRoot)) {
    New-Item -ItemType Directory -Path $BackupRoot -Force | Out-Null
}

$timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
$backupDir = Join-Path $BackupRoot "$Label-$timestamp"

Write-Host "Creating backup from $resolvedSource" -ForegroundColor Cyan
Write-Host "Backup directory: $backupDir" -ForegroundColor Cyan

New-Item -ItemType Directory -Path $backupDir -Force | Out-Null
$backupDataDir = Join-Path $backupDir "data"
New-Item -ItemType Directory -Path $backupDataDir -Force | Out-Null

Get-ChildItem -LiteralPath $sourceRoot -Recurse -Force | ForEach-Object {
    $relativePath = $_.FullName.Substring($sourceRoot.Length).TrimStart('\')
    $targetPath = Join-Path $backupDataDir $relativePath

    if ($_.PSIsContainer) {
        New-Item -ItemType Directory -Path $targetPath -Force | Out-Null
        return
    }

    $parentDir = Split-Path -Parent $targetPath
    if ($parentDir -and -not (Test-Path $parentDir)) {
        New-Item -ItemType Directory -Path $parentDir -Force | Out-Null
    }

    try {
        Copy-Item -LiteralPath $_.FullName -Destination $targetPath -Force -ErrorAction Stop
    } catch {
        $skippedFiles += $relativePath
        Write-Host "Skipping locked/unreadable file: $relativePath" -ForegroundColor Yellow
    }
}

$metadata = @{
    created_at = (Get-Date).ToString("o")
    source_data_dir = $sourceRoot
    label = $Label
    host = $env:COMPUTERNAME
    skipped_files = $skippedFiles
} | ConvertTo-Json -Depth 4

$metadata | Set-Content -LiteralPath (Join-Path $backupDir "backup-metadata.json")

Write-Host "Backup completed: $backupDir" -ForegroundColor Green
if ($skippedFiles.Count -gt 0) {
    Write-Host "Skipped $($skippedFiles.Count) locked/unreadable file(s). See backup-metadata.json for details." -ForegroundColor Yellow
}
