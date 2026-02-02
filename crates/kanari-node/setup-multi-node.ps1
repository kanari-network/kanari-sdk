# Kanari Multi-Node Launcher
# This script launches multiple Kanari nodes with different configurations

Write-Host "Starting Kanari Multi-Node Setup..." -ForegroundColor Green
Write-Host ""

# Configuration
$BaseDataDir = "$env:USERPROFILE\.kanari\node-db"
$BasePeerPort = 19000
$BaseRpcPort = 19001
$NodeCount = 3

# Generate authorities list (0x1, 0x2, ..., 0xN)
$authorities = @()
for ($i = 1; $i -le $NodeCount; $i++) {
    $authorities += "0x$i"
}
$authoritiesStr = $authorities -join ","

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

# Detect LAN IPv4 address by preferring physical, Up interfaces (skip virtual/loopback)
$localIp = $null
$adapters = Get-NetAdapter -ErrorAction SilentlyContinue |
    Where-Object { $_.Status -eq 'Up' -and $_.InterfaceDescription -notmatch 'Virtual|vEthernet|Hyper-V|Docker|VMware|Loopback' }
foreach ($a in $adapters) {
    $ip = Get-NetIPAddress -InterfaceIndex $a.ifIndex -AddressFamily IPv4 -ErrorAction SilentlyContinue |
        Where-Object { $_.IPAddress -notmatch '^127\.' -and $_.IPAddress -notmatch '^169\.254\.' } |
        Select-Object -First 1
    if ($ip) { $localIp = $ip.IPAddress; break }
}
# If not found, try any non-loopback/non-APIPA IPv4
if (-not $localIp) {
    $localIp = (Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue |
        Where-Object { $_.IPAddress -notmatch '^127\.' -and $_.IPAddress -notmatch '^169\.254\.' } |
        Select-Object -First 1).IPAddress
}

for ($i = 1; $i -le $NodeCount; $i++) {
    $p2pPort = $BasePeerPort + (($i - 1) * 10)
    $rpcPort = $BaseRpcPort + (($i - 1) * 10)
    $dataDir = "$BaseDataDir\node$i"
    
    Write-Host ""
    Write-Host "Node ${i}:" -ForegroundColor White
    Write-Host "  P2P Port: $p2pPort" -ForegroundColor Gray
    Write-Host "  RPC Port: $rpcPort" -ForegroundColor Gray
    Write-Host "  Data Dir: $dataDir" -ForegroundColor Gray
    if ($localIp) {
        Write-Host ("  RPC URL:  http://{0}:{1}" -f $localIp, $rpcPort) -ForegroundColor Gray
    } else {
        Write-Host '  RPC URL:  (no LAN IP detected)' -ForegroundColor DarkYellow
    }
}

Write-Host ""
Write-Host "To start each node, run these commands in separate terminals:" -ForegroundColor Yellow
Write-Host ""

for ($i = 1; $i -le $NodeCount; $i++) {
    $p2pPort = $BasePeerPort + (($i - 1) * 10)
    $rpcPort = $BaseRpcPort + (($i - 1) * 10)
    $dataDir = "$BaseDataDir\node$i"
    $authId = "0x$i"
    
    Write-Host "# Terminal ${i} (Node ${i}):" -ForegroundColor Cyan
    # Recommend binding RPC to all interfaces so the node is reachable via the machine IP
    Write-Host "kanari-node start --p2p-port $p2pPort --rpc-port $rpcPort --rpc-host 0.0.0.0 --data-dir `"$dataDir`" --authority-id $authId --authorities $authoritiesStr" -ForegroundColor White
    Write-Host ""
}

Write-Host ""
Write-Host "Or use the start-node.ps1 script to start individual nodes (it will detect and display the machine IP):" -ForegroundColor Yellow
Write-Host "  .\start-node.ps1 -NodeId 1" -ForegroundColor White
Write-Host "  .\start-node.ps1 -NodeId 2" -ForegroundColor White
Write-Host "  .\start-node.ps1 -NodeId 3" -ForegroundColor White
Write-Host ""
Write-Host "" -ForegroundColor Gray
# Prompt to start nodes in separate terminals
$startNow = Read-Host "Start all nodes now in separate terminals? (Y/N)"
if ($startNow -match '^[Yy]') {
    $scriptPath = Join-Path $PSScriptRoot 'start-node.ps1'
    for ($i = 1; $i -le $NodeCount; $i++) {
        $p2pPort = $BasePeerPort + (($i - 1) * 10)
        $rpcPort = $BaseRpcPort + (($i - 1) * 10)
        $dataDir = "$BaseDataDir\node$i"

        # Launch new PowerShell window running the start-node script for this node
        $argString = "-NoExit -ExecutionPolicy Bypass -File `"$scriptPath`" -NodeId $i -Authorities `"$authoritiesStr`""
        Start-Process -FilePath "powershell.exe" -ArgumentList $argString -WindowStyle Normal
        Start-Sleep -Milliseconds 500
    }
    Write-Host "Launched $NodeCount terminals." -ForegroundColor Green
} else {
    Write-Host "Skipped launching terminals. Run start-node.ps1 manually to start nodes." -ForegroundColor Yellow
}
