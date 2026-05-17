# Kanari Multi-Node Launcher
param(
    [int]$NodeCount = 3,
    [string]$SourceNodeDataDir = "$env:USERPROFILE\.kanari\kanari-db",
    [string]$ReplicaBaseDataDir = "$env:USERPROFILE\.kanari\node-db",
    [int]$BasePeerPort = 19000,
    [int]$BaseRpcPort = 19001,
    [switch]$ResetReplicaData,
    [switch]$ResetSourceData,
    [switch]$DisableFailFast,
    [switch]$SkipHealthCheck
)

function Get-LanIpAddress {
    $adapters = Get-NetAdapter -ErrorAction SilentlyContinue |
        Where-Object {
            $_.Status -eq 'Up' -and
            $_.InterfaceDescription -notmatch 'Virtual|vEthernet|Hyper-V|Docker|VMware|Loopback'
        }

    foreach ($adapter in $adapters) {
        $ip = Get-NetIPAddress -InterfaceIndex $adapter.ifIndex -AddressFamily IPv4 -ErrorAction SilentlyContinue |
            Where-Object {
                $_.IPAddress -notmatch '^127\.' -and
                $_.IPAddress -notmatch '^169\.254\.'
            } |
            Select-Object -First 1
        if ($ip) {
            return $ip.IPAddress
        }
    }

    return (
        Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue |
            Where-Object {
                $_.IPAddress -notmatch '^127\.' -and
                $_.IPAddress -notmatch '^169\.254\.'
            } |
            Select-Object -First 1
    ).IPAddress
}

function Test-NodeHealth {
    param(
        [string]$RpcUrl,
        [int]$NodeId,
        [switch]$RequireBootstrappedState
    )

    $body = @{
        jsonrpc = "2.0"
        method = "kanari_health"
        params = @{}
        id = 1
    } | ConvertTo-Json -Depth 5

    try {
        $response = Invoke-RestMethod -Uri $RpcUrl -Method Post -Body $body -ContentType "application/json" -TimeoutSec 5
        $health = $response.result

        if (-not $health) {
            Write-Host "Node $NodeId health check returned no result." -ForegroundColor Yellow
            return
        }

        $status = $health.status
        $supplyOk = $health.supply_invariants_ok
        $failFast = $health.fail_fast_enabled

        if ($status -eq "ok" -and $supplyOk) {
            Write-Host "Node $NodeId health OK | fail-fast=$failFast | $RpcUrl" -ForegroundColor Green
        } else {
            $detail = $health.supply_invariant_error
            Write-Host "Node $NodeId health DEGRADED | fail-fast=$failFast | $RpcUrl" -ForegroundColor Yellow
            if ($detail) {
                Write-Host "  Supply invariant error: $detail" -ForegroundColor Yellow
            }
        }

        if ($RequireBootstrappedState) {
            $statsBody = @{
                jsonrpc = "2.0"
                method = "kanari_getStats"
                params = @{}
                id = 2
            } | ConvertTo-Json -Depth 5

            $statsResponse = Invoke-RestMethod -Uri $RpcUrl -Method Post -Body $statsBody -ContentType "application/json" -TimeoutSec 5
            $stats = $statsResponse.result

            if (-not $stats) {
                throw "Node $NodeId stats request returned no result."
            }

            if (($stats.total_supply -as [long]) -le 0) {
                throw "Node $NodeId reports total_supply=0 after startup."
            }

            Write-Host "Node $NodeId stats OK | total_supply=$($stats.total_supply) | accounts=$($stats.total_accounts)" -ForegroundColor Green
        }
    } catch {
        Write-Host "Node $NodeId health check failed at $RpcUrl : $($_.Exception.Message)" -ForegroundColor Yellow
    }
}

Write-Host "Starting Kanari Multi-Node Setup..." -ForegroundColor Green
Write-Host ""

if ($NodeCount -lt 1) {
    Write-Host "NodeCount must be at least 1." -ForegroundColor Red
    exit 1
}

$failFastEnabled = -not $DisableFailFast
$env:KANARI_FAIL_FAST_ON_SUPPLY_MISMATCH = if ($failFastEnabled) { "true" } else { "false" }

Write-Host "Node count: $NodeCount" -ForegroundColor Cyan
Write-Host "Source node data dir (node1): $SourceNodeDataDir" -ForegroundColor Cyan
Write-Host "Replica base data dir (node2..N): $ReplicaBaseDataDir" -ForegroundColor Cyan
Write-Host "Supply fail-fast: $($env:KANARI_FAIL_FAST_ON_SUPPLY_MISMATCH)" -ForegroundColor Cyan
Write-Host ""

if (-not (Test-Path $SourceNodeDataDir)) {
    New-Item -ItemType Directory -Path $SourceNodeDataDir -Force | Out-Null
}
if (-not (Test-Path $ReplicaBaseDataDir)) {
    New-Item -ItemType Directory -Path $ReplicaBaseDataDir -Force | Out-Null
}

if ($ResetSourceData) {
    Write-Host "ResetSourceData enabled: clearing source node database..." -ForegroundColor Yellow
    Get-ChildItem -LiteralPath $SourceNodeDataDir -Force -ErrorAction SilentlyContinue |
        Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
} else {
    Write-Host "Preserving source node database for node1." -ForegroundColor Cyan
}

if ($ResetReplicaData) {
    Write-Host "ResetReplicaData enabled: clearing replica node databases..." -ForegroundColor Yellow
    for ($i = 2; $i -le $NodeCount; $i++) {
        $nodeDir = Join-Path $ReplicaBaseDataDir "node$i"
        if (Test-Path $nodeDir) {
            Remove-Item -LiteralPath $nodeDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
} else {
    Write-Host "Preserving replica node databases. Use -ResetReplicaData for a fresh sync set." -ForegroundColor Cyan
}

$authorities = @()
for ($i = 1; $i -le $NodeCount; $i++) {
    $authorities += "0x$i"
}
$authoritiesStr = $authorities -join ","

$localIp = Get-LanIpAddress
if (-not $localIp) {
    Write-Host "Warning: no LAN IPv4 detected. Bootstrap may fail if peers cannot resolve node1." -ForegroundColor Yellow
}

Write-Host ""
$startNow = Read-Host "Start all $NodeCount nodes now with node1 as source? (Y/N)"
if ($startNow -notmatch '^[Yy]') {
    Write-Host "Aborted." -ForegroundColor Yellow
    exit 0
}

$scriptPath = Join-Path $PSScriptRoot 'start-node.ps1'
$currentPS = (Get-Process -Id $PID).Path

for ($i = 1; $i -le $NodeCount; $i++) {
    $nodeP2pPort = $BasePeerPort + (($i - 1) * 10)
    $nodeRpcPort = $BaseRpcPort + (($i - 1) * 10)

    if ($i -eq 1) {
        $dataDir = $SourceNodeDataDir
        $argString = "-NoExit -ExecutionPolicy Bypass -File `"$scriptPath`" -NodeId $i -DataDir `"$dataDir`" -BasePeerPort $BasePeerPort -BaseRpcPort $BaseRpcPort -Authorities `"$authoritiesStr`""
    } else {
        $dataDir = Join-Path $ReplicaBaseDataDir "node$i"
        $bootstrapAddr = "/ip4/$localIp/tcp/$BasePeerPort"
        $argString = "-NoExit -ExecutionPolicy Bypass -File `"$scriptPath`" -NodeId $i -DataDir `"$dataDir`" -BasePeerPort $BasePeerPort -BaseRpcPort $BaseRpcPort -Authorities `"$authoritiesStr`" -Bootstrap `"$bootstrapAddr`""
    }

    Write-Host "Launching node $i | P2P $nodeP2pPort | RPC $nodeRpcPort | DataDir $dataDir" -ForegroundColor Cyan
    Start-Process -FilePath $currentPS -ArgumentList $argString -WindowStyle Normal

    if ($i -eq 1) {
        Write-Host "Waiting 5 seconds for node1 source to initialize on ${localIp}:$BasePeerPort..." -ForegroundColor Cyan
        Start-Sleep -Seconds 5
    } else {
        Start-Sleep -Milliseconds 600
    }
}

Write-Host "Launched $NodeCount terminals successfully." -ForegroundColor Green
Write-Host "Node1 is the source node. Node2..N bootstrap from node1." -ForegroundColor Green

if (-not $SkipHealthCheck) {
    Write-Host ""
    Write-Host "Checking RPC health endpoints..." -ForegroundColor Cyan
    Start-Sleep -Seconds 3

    for ($i = 1; $i -le $NodeCount; $i++) {
        $nodeRpcPort = $BaseRpcPort + (($i - 1) * 10)
        $rpcUrl = if ($localIp) { "http://$localIp`:$nodeRpcPort" } else { "http://127.0.0.1:$nodeRpcPort" }
        if ($i -eq 1) {
            Test-NodeHealth -RpcUrl $rpcUrl -NodeId $i -RequireBootstrappedState
        } else {
            Test-NodeHealth -RpcUrl $rpcUrl -NodeId $i
        }
    }
}
