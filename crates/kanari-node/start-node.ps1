# Start a specific Kanari node
param(
    [Parameter(Mandatory=$true)]
    [int]$NodeId = 1,

    [ValidateSet("mainnet", "testnet", "devnet")]
    [string]$Network = "testnet",
    [string]$DataDir = "",
    [string]$BaseDataDir = "$env:USERPROFILE\.kanari\node-db",
    [int]$BasePeerPort = 19000,
    [int]$BaseRpcPort = 19001,
    [string]$Authorities = "",
    [string]$Bootstrap = ""
)

. (Join-Path $PSScriptRoot 'node-script-common.ps1')

$ports = Get-NodePorts -NodeId $NodeId -BasePeerPort $BasePeerPort -BaseRpcPort $BaseRpcPort
$p2pPort = $ports.P2pPort
$rpcPort = $ports.RpcPort
$dataDir = Get-NodeDataDir -NodeId $NodeId -DataDir $DataDir -BaseDataDir $BaseDataDir
$authId = "0x$NodeId"
$localIp = Get-LanIpAddress
$rpcUrl = Get-NodeRpcUrl -HostIp $localIp -RpcPort $rpcPort

if (-not (Test-Path $dataDir)) {
    New-Item -ItemType Directory -Path $dataDir -Force | Out-Null
}

Write-Host '========================================' -ForegroundColor Cyan
Write-Host 'Starting Kanari Node' $NodeId -ForegroundColor Green
Write-Host '========================================' -ForegroundColor Cyan
Write-Host 'P2P Port:' $p2pPort -ForegroundColor Yellow
Write-Host 'RPC Port:' $rpcPort -ForegroundColor Yellow
Write-Host 'Network:' $Network -ForegroundColor Yellow
Write-Host 'Data Dir:' $dataDir -ForegroundColor Yellow
if ($localIp) {
    Write-Host "RPC URL:  $rpcUrl" -ForegroundColor Yellow
} else {
    Write-Host 'RPC URL:  (no LAN IP detected) RPC will bind to all interfaces' -ForegroundColor DarkYellow
}
Write-Host '========================================' -ForegroundColor Cyan
Write-Host ''

try {
    $exeInfo = Find-KanariNodeExecutable
    $exePath = $exeInfo.Path
    Write-Host $exeInfo.Label -ForegroundColor $exeInfo.Color
} catch {
    Write-Host 'Error: kanari-node executable not found!' -ForegroundColor Red
    exit 1
}

Write-Host ''

if ($Authorities -ne "") {
    if ($Bootstrap -ne "") {
        & $exePath start --network $Network --p2p-port $p2pPort --rpc-port $rpcPort --rpc-host 0.0.0.0 --data-dir $dataDir --authority-id $authId --authorities $Authorities --bootstrap $Bootstrap
    } else {
        & $exePath start --network $Network --p2p-port $p2pPort --rpc-port $rpcPort --rpc-host 0.0.0.0 --data-dir $dataDir --authority-id $authId --authorities $Authorities
    }
} else {
    & $exePath start --network $Network --p2p-port $p2pPort --rpc-port $rpcPort --rpc-host 0.0.0.0 --data-dir $dataDir
}
