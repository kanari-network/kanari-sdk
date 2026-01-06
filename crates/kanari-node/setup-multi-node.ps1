# Kanari Multi-Node Launcher
# This script launches multiple Kanari nodes with different configurations

Write-Host "Starting Kanari Multi-Node Setup..." -ForegroundColor Green
Write-Host ""

# Configuration
$BaseDataDir = "$env:USERPROFILE\.kanari\kanari-db"
$BasePeerPort = 19000
$BaseRpcPort = 19001
$NodeCount = 3

# Create data directories
Write-Host "Creating data directories..." -ForegroundColor Yellow
for ($i = 1; $i -le $NodeCount; $i++) {
    $dataDir = "$BaseDataDir\node$i"
    if (-not (Test-Path $dataDir)) {
        New-Item -ItemType Directory -Path $dataDir -Force | Out-Null
        Write-Host "  Created: $dataDir" -ForegroundColor Gray
    }
}

Write-Host ""
Write-Host "Node Configuration:" -ForegroundColor Cyan
Write-Host "==================" -ForegroundColor Cyan

for ($i = 1; $i -le $NodeCount; $i++) {
    $p2pPort = $BasePeerPort + (($i - 1) * 10)
    $rpcPort = $BaseRpcPort + (($i - 1) * 10)
    $dataDir = "$BaseDataDir\node$i"
    
    Write-Host ""
    Write-Host "Node ${i}:" -ForegroundColor White
    Write-Host "  P2P Port: $p2pPort" -ForegroundColor Gray
    Write-Host "  RPC Port: $rpcPort" -ForegroundColor Gray
    Write-Host "  Data Dir: $dataDir" -ForegroundColor Gray
    Write-Host "  RPC URL:  http://127.0.0.1:$rpcPort" -ForegroundColor Gray
}

Write-Host ""
Write-Host "To start each node, run these commands in separate terminals:" -ForegroundColor Yellow
Write-Host ""

for ($i = 1; $i -le $NodeCount; $i++) {
    $p2pPort = $BasePeerPort + (($i - 1) * 10)
    $rpcPort = $BaseRpcPort + (($i - 1) * 10)
    $dataDir = "$BaseDataDir\node$i"
    
    Write-Host "# Terminal ${i} (Node ${i}):" -ForegroundColor Cyan
    Write-Host "kanari-node start --p2p-port $p2pPort --rpc-port $rpcPort --data-dir `"$dataDir`"" -ForegroundColor White
    Write-Host ""
}

Write-Host ""
Write-Host "Or use the start-node.ps1 script to start individual nodes:" -ForegroundColor Yellow
Write-Host "  .\start-node.ps1 -NodeId 1" -ForegroundColor White
Write-Host "  .\start-node.ps1 -NodeId 2" -ForegroundColor White
Write-Host "  .\start-node.ps1 -NodeId 3" -ForegroundColor White
Write-Host ""
Write-Host "Press any key to exit..." -ForegroundColor Gray
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
