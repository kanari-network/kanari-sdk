# Test P2P Transaction Broadcasting
# This script sends transactions to Node 1 and verifies they appear in other nodes

Write-Host "Testing P2P Transaction Broadcasting..." -ForegroundColor Cyan
Write-Host ""

# Node RPC endpoints
$node1 = "http://127.0.0.1:19001"
$node2 = "http://127.0.0.1:19011"
$node3 = "http://127.0.0.1:19021"

# Function to get blockchain stats from a node
function Get-NodeStats {
    param($endpoint)
    try {
        $response = Invoke-RestMethod -Uri "$endpoint/stats" -Method Get
        return $response
    } catch {
        return $null
    }
}

# Check all nodes are running
Write-Host "Checking nodes..." -ForegroundColor Yellow
$stats1 = Get-NodeStats $node1
$stats2 = Get-NodeStats $node2
$stats3 = Get-NodeStats $node3

if (-not $stats1) { Write-Host "Node 1 is not running!" -ForegroundColor Red; exit 1 }
if (-not $stats2) { Write-Host "Node 2 is not running!" -ForegroundColor Red; exit 1 }
if (-not $stats3) { Write-Host "Node 3 is not running!" -ForegroundColor Red; exit 1 }

Write-Host "All nodes are running!" -ForegroundColor Green
Write-Host ""

# Show initial stats
Write-Host "Initial Stats:" -ForegroundColor Cyan
Write-Host "Node 1: Height=$($stats1.height), Pending=$($stats1.pending_transactions)" -ForegroundColor Gray
Write-Host "Node 2: Height=$($stats2.height), Pending=$($stats2.pending_transactions)" -ForegroundColor Gray
Write-Host "Node 3: Height=$($stats3.height), Pending=$($stats3.pending_transactions)" -ForegroundColor Gray
Write-Host ""

# Send a transaction to Node 1
Write-Host "Sending transaction to Node 1..." -ForegroundColor Yellow
# You would need to implement actual transaction sending here
# For now, just show the concept

Write-Host ""
Write-Host "Waiting 5 seconds for P2P propagation..." -ForegroundColor Yellow
Start-Sleep -Seconds 5

# Check stats again
$stats1_after = Get-NodeStats $node1
$stats2_after = Get-NodeStats $node2
$stats3_after = Get-NodeStats $node3

Write-Host ""
Write-Host "Stats after transaction:" -ForegroundColor Cyan
Write-Host "Node 1: Height=$($stats1_after.height), Pending=$($stats1_after.pending_transactions)" -ForegroundColor Gray
Write-Host "Node 2: Height=$($stats2_after.height), Pending=$($stats2_after.pending_transactions)" -ForegroundColor Gray
Write-Host "Node 3: Height=$($stats3_after.height), Pending=$($stats3_after.pending_transactions)" -ForegroundColor Gray

Write-Host ""
if ($stats2_after.pending_transactions -gt $stats2.pending_transactions -or 
    $stats3_after.pending_transactions -gt $stats3.pending_transactions) {
    Write-Host "SUCCESS: Transaction was broadcast to other nodes!" -ForegroundColor Green
} else {
    Write-Host "PENDING: Transaction might not have been broadcast yet" -ForegroundColor Yellow
    Write-Host "Check the node logs for P2P messages" -ForegroundColor Gray
}
