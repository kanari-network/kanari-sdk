param(
    [string[]]$RpcUrls = @(
        "http://127.0.0.1:19001",
        "http://127.0.0.1:19011",
        "http://127.0.0.1:19021"
    ),
    [switch]$RequireEqualHeight,
    [switch]$RequireEqualSupply
)

. (Join-Path $PSScriptRoot 'node-script-common.ps1')

$failures = 0
$heights = @()
$supplies = @()

for ($i = 0; $i -lt $RpcUrls.Count; $i++) {
    $rpcUrl = $RpcUrls[$i]
    $nodeId = $i + 1

    try {
        $health = Get-NodeHealthStatus -RpcUrl $rpcUrl
        $stats = Get-NodeStats -RpcUrl $rpcUrl

        if (-not $health -or -not $stats) {
            throw "missing health or stats response"
        }

        $heights += [long]$stats.height
        $supplies += [long]$stats.total_supply

        if ($health.status -ne "ok" -or -not $health.supply_invariants_ok) {
            $failures++
            Write-Host "Node $nodeId unhealthy | url=$rpcUrl | status=$($health.status) | error=$($health.supply_invariant_error)" -ForegroundColor Yellow
            continue
        }

        Write-Host "Node $nodeId healthy | network=$($health.network) | height=$($stats.height) | supply=$($stats.total_supply) | url=$rpcUrl" -ForegroundColor Green
    } catch {
        $failures++
        Write-Host "Node $nodeId check failed | url=$rpcUrl | error=$($_.Exception.Message)" -ForegroundColor Red
    }
}

if ($RequireEqualHeight -and $heights.Count -gt 1) {
    $uniqueHeights = $heights | Sort-Object -Unique
    if ($uniqueHeights.Count -ne 1) {
        $failures++
        Write-Host "Height mismatch across cluster: $($uniqueHeights -join ', ')" -ForegroundColor Yellow
    }
}

if ($RequireEqualSupply -and $supplies.Count -gt 1) {
    $uniqueSupplies = $supplies | Sort-Object -Unique
    if ($uniqueSupplies.Count -ne 1) {
        $failures++
        Write-Host "Supply mismatch across cluster: $($uniqueSupplies -join ', ')" -ForegroundColor Yellow
    }
}

if ($failures -gt 0) {
    Write-Host "Cluster health check failed with $failures issue(s)." -ForegroundColor Red
    exit 1
}

Write-Host "Cluster health check passed." -ForegroundColor Green
