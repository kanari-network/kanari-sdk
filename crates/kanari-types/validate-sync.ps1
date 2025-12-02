#!/usr/bin/env pwsh
# Kanari Types Sync Validation Script

Write-Host "Kanari Types - Move Sync Validation" -ForegroundColor Cyan
Write-Host ("=" * 60)

$ErrorCount = 0

# Test 1: Rust tests
Write-Host "`nRunning Rust tests..." -ForegroundColor Yellow
Push-Location $PSScriptRoot
$Result = cargo test -q 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "PASS: Rust tests" -ForegroundColor Green
} else {
    Write-Host "FAIL: Rust tests" -ForegroundColor Red
    $ErrorCount++
}
Pop-Location

# Test 2: Move tests
Write-Host "`nRunning Move tests..." -ForegroundColor Yellow
$MovePath = Join-Path $PSScriptRoot "..\..\crates\kanari-frameworks\packages\kanari-system"
if (Test-Path $MovePath) {
    Push-Location $MovePath
    $Result = move-cli test 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "PASS: Move tests" -ForegroundColor Green
    } else {
        Write-Host "FAIL: Move tests" -ForegroundColor Red
        $ErrorCount++
    }
    Pop-Location
}

# Summary
Write-Host "`n" + ("=" * 60)
if ($ErrorCount -eq 0) {
    Write-Host "SUCCESS: All checks passed" -ForegroundColor Green
    exit 0
} else {
    Write-Host "FAILED: $ErrorCount errors" -ForegroundColor Red
    exit 1
}
