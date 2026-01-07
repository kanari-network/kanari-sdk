# Start a specific Kanari node
param(
    [Parameter(Mandatory=$true)]
    [int]$NodeId = 1,
    
    [string]$BaseDataDir = "$env:USERPROFILE\.kanari\kanari-db",
    [int]$BasePeerPort = 19000,
    [int]$BaseRpcPort = 19001
)

$p2pPort = $BasePeerPort + (($NodeId - 1) * 10)
$rpcPort = $BaseRpcPort + (($NodeId - 1) * 10)
$dataDir = "$BaseDataDir\node$NodeId"

# Create data directory if it doesn't exist
if (-not (Test-Path $dataDir)) {
    New-Item -ItemType Directory -Path $dataDir -Force | Out-Null
}

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
if (-not $localIp) {
    $localIp = (Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue |
        Where-Object { $_.IPAddress -notmatch '^127\.' -and $_.IPAddress -notmatch '^169\.254\.' } |
        Select-Object -First 1).IPAddress
}

Write-Host '========================================' -ForegroundColor Cyan
Write-Host 'Starting Kanari Node' $NodeId -ForegroundColor Green
Write-Host '========================================' -ForegroundColor Cyan
Write-Host 'P2P Port:' $p2pPort -ForegroundColor Yellow
Write-Host 'RPC Port:' $rpcPort -ForegroundColor Yellow
Write-Host 'Data Dir:' $dataDir -ForegroundColor Yellow
if ($localIp) {
    Write-Host ("RPC URL:  http://{0}:{1}" -f $localIp, $rpcPort) -ForegroundColor Yellow
} else {
    Write-Host 'RPC URL:  (no LAN IP detected) — RPC will bind to all interfaces' -ForegroundColor DarkYellow
}
Write-Host '========================================' -ForegroundColor Cyan
Write-Host ''

# Find kanari-node executable
$exePath = $null

# Check if cargo target exists
$releaseExe = "..\..\target\release\kanari-node.exe"
$debugExe = "..\..\target\debug\kanari-node.exe"

if (Test-Path $releaseExe) {
    $exePath = $releaseExe
    Write-Host 'Using release build:' $releaseExe -ForegroundColor Green
} elseif (Test-Path $debugExe) {
    $exePath = $debugExe
    Write-Host 'Using debug build:' $debugExe -ForegroundColor Yellow
} elseif (Get-Command kanari-node -ErrorAction SilentlyContinue) {
    $exePath = 'kanari-node'
    Write-Host 'Using kanari-node from PATH' -ForegroundColor Green
} else {
    Write-Host 'Error: kanari-node executable not found!' -ForegroundColor Red
    Write-Host 'Please build the project first:' -ForegroundColor Yellow
    Write-Host '  cd ..\..' -ForegroundColor Gray
    Write-Host '  cargo build --release' -ForegroundColor Gray
    Write-Host ''
    Write-Host 'Or use debug build:' -ForegroundColor Yellow
    Write-Host '  cargo build' -ForegroundColor Gray
    exit 1
}

Write-Host ''

# Start the node (bind RPC to all interfaces so it is reachable via the machine's IP)
& $exePath start --p2p-port $p2pPort --rpc-port $rpcPort --rpc-host 0.0.0.0 --data-dir $dataDir

