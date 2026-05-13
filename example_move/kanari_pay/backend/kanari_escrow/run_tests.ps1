# Run Move Tests with Clean Output
# Usage: .\run_tests.ps1

Write-Host "🧪 Running Move Unit Tests..." -ForegroundColor Cyan
Write-Host ""

$env:RUST_BACKTRACE = "0"

$result = cargo r -p kanari move test 2>&1 | Select-String -Pattern "PASS|FAIL|Test result" -Context 0,0

$result | ForEach-Object {
    if ($_ -match "PASS") {
        Write-Host $_ -ForegroundColor Green
    } elseif ($_ -match "FAIL") {
        Write-Host $_ -ForegroundColor Red
    } else {
        Write-Host $_ -ForegroundColor Yellow
    }
}

Write-Host ""
if ($result -match "OK") {
    Write-Host "✅ All tests passed!" -ForegroundColor Green
} else {
    Write-Host "❌ Some tests failed!" -ForegroundColor Red
}
