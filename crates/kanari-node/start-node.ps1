# Start a specific Kanari node
param(
    [Parameter(Mandatory=$true)]
    [int]$NodeId = 1,
    
    [string]$DataDir = "",
    [string]$BaseDataDir = "$env:USERPROFILE\.kanari\node-db",
    [int]$BasePeerPort = 19000,
    [int]$BaseRpcPort = 19001,
    [string]$Authorities = "",
    [string]$Bootstrap = "" 
)

$p2pPort = $BasePeerPort + (($NodeId - 1) * 10)
$rpcPort = $BaseRpcPort + (($NodeId - 1) * 10)
$dataDir = if ($DataDir -ne "") { $DataDir } else { "$BaseDataDir\node$NodeId" }
$authId = "0x$NodeId"

# Create data directory if it doesn't exist
if (-not (Test-Path $dataDir)) {
    New-Item -ItemType Directory -Path $dataDir -Force | Out-Null
}

# Detect LAN IPv4 address
$localIp = $null
$adapters = Get-NetAdapter -ErrorAction SilentlyContinue | Where-Object { $_.Status -eq 'Up' -and $_.InterfaceDescription -notmatch 'Virtual|vEthernet|Hyper-V|Docker|VMware|Loopback' }
foreach ($a in $adapters) {
    $ip = Get-NetIPAddress -InterfaceIndex $a.ifIndex -AddressFamily IPv4 -ErrorAction SilentlyContinue | Where-Object { $_.IPAddress -notmatch '^127\.' -and $_.IPAddress -notmatch '^169\.254\.' } | Select-Object -First 1
    if ($ip) { $localIp = $ip.IPAddress; break }
}
if (-not $localIp) {
    $localIp = (Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue | Where-Object { $_.IPAddress -notmatch '^127\.' -and $_.IPAddress -notmatch '^169\.254\.' } | Select-Object -First 1).IPAddress
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
    exit 1
}

Write-Host ''

# Start the node (รองรับการใส่ Bootstrap)
if ($Authorities -ne "") {
    if ($Bootstrap -ne "") {
        & $exePath start --p2p-port $p2pPort --rpc-port $rpcPort --rpc-host 0.0.0.0 --data-dir $dataDir --authority-id $authId --authorities $Authorities --bootstrap $Bootstrap
    } else {
        & $exePath start --p2p-port $p2pPort --rpc-port $rpcPort --rpc-host 0.0.0.0 --data-dir $dataDir --authority-id $authId --authorities $Authorities
    }
} else {
    & $exePath start --p2p-port $p2pPort --rpc-port $rpcPort --rpc-host 0.0.0.0 --data-dir $dataDir
}
