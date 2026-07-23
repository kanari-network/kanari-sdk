param(
    [ValidateRange(1, 10000000)]
    [int]$Requests = 20000,

    [ValidateRange(1, 10000)]
    [int]$Concurrency = 256,

    [ValidateRange(1, 120)]
    [int]$TimeoutSec = 5,

    [int]$BaseP2pPort = 19700,

    [int]$BaseRpcPort = 19701,

    [ValidateRange(1, 300)]
    [int]$SyncTimeoutSec = 90,

    [ValidateRange(0, 1000000000)]
    [double]$MinRps = 0,

    [ValidateRange(0, 3600000)]
    [int]$MaxP99Ms = 0,

    [ValidateRange(0, 100)]
    [double]$MaxEndpointImbalancePercent = 10,

    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$nodeExe = Join-Path $repoRoot 'target\release\kanari-node.exe'
$loadScript = Join-Path $repoRoot 'scripts\run-production-rpc-load.ps1'
$runId = Get-Date -Format 'yyyyMMdd-HHmmss'
$runRoot = Join-Path $repoRoot ".codex-runlogs\four-node-chaos-$runId"

New-Item -ItemType Directory -Force -Path $runRoot | Out-Null

if (-not $SkipBuild) {
    Push-Location $repoRoot
    try {
        cargo build -p kanari-node --release
        cargo build -p kanari-rpc-loadgen --release
    } finally {
        Pop-Location
    }
}

if (-not (Test-Path $nodeExe)) {
    throw "Release kanari-node not found at $nodeExe"
}
if (-not (Test-Path $loadScript)) {
    throw "RPC load script not found at $loadScript"
}

function Invoke-KanariRpc {
    param(
        [string]$Url,
        [string]$Method,
        [object]$Params = @{},
        [int]$Id = 1,
        [int]$TimeoutSec = 5
    )

    $body = @{
        jsonrpc = '2.0'
        id = $Id
        method = $Method
        params = $Params
    } | ConvertTo-Json -Depth 32 -Compress

    return Invoke-RestMethod -Uri $Url -Method Post -ContentType 'application/json' -Body $body -TimeoutSec $TimeoutSec
}

function Start-KanariChaosNode {
    param([int]$Index)

    $offset = ($Index - 1) * 10
    $p2pPort = $BaseP2pPort + $offset
    $rpcPort = $BaseRpcPort + $offset
    $dataDir = Join-Path $runRoot "node$Index-db"
    New-Item -ItemType Directory -Force -Path $dataDir | Out-Null

    $nodeArgs = @(
        'start',
        '--network', 'devnet',
        '--p2p-port', $p2pPort,
        '--rpc-port', $rpcPort,
        '--rpc-host', '127.0.0.1',
        '--data-dir', $dataDir,
        '--authority-id', "0x$Index",
        '--authorities', '0x1,0x2,0x3,0x4',
        '--genesis', $genesis,
        '--consensus-private-key-file', (Join-Path $keysDir "node$Index-consensus-private-key.key"),
        '--consensus-public-keys', (Join-Path $keysDir 'consensus-public-keys.json')
    )

    if ($Index -ne 1) {
        $nodeArgs += @('--bootstrap', "/ip4/127.0.0.1/tcp/$BaseP2pPort")
    }

    return Start-Process `
        -FilePath $nodeExe `
        -ArgumentList $nodeArgs `
        -WorkingDirectory $repoRoot `
        -RedirectStandardOutput (Join-Path $runRoot "node$Index.out.log") `
        -RedirectStandardError (Join-Path $runRoot "node$Index.err.log") `
        -WindowStyle Hidden `
        -PassThru
}

function Wait-RpcReady {
    param([string[]]$Urls, [int]$Attempts = 120)

    $ready = @{}
    for ($attempt = 1; $attempt -le $Attempts; $attempt++) {
        foreach ($url in $Urls) {
            if ($ready[$url]) {
                continue
            }
            try {
                $health = Invoke-KanariRpc -Url $url -Method 'kanari_health' -Id 1 -TimeoutSec 2
                if ($health.result) {
                    $ready[$url] = $true
                    Write-Host "READY $url"
                }
            } catch {
            }
        }
        if ($ready.Count -eq $Urls.Count) {
            return
        }
        Start-Sleep -Seconds 1
    }

    throw "only $($ready.Count)/$($Urls.Count) RPC endpoints became ready; logs: $runRoot"
}

function Wait-StatsConverged {
    param([string[]]$Urls)

    $deadline = (Get-Date).AddSeconds($SyncTimeoutSec)
    while ((Get-Date) -lt $deadline) {
        $roots = New-Object System.Collections.Generic.HashSet[string]
        $heights = New-Object System.Collections.Generic.HashSet[string]
        $txs = New-Object System.Collections.Generic.HashSet[string]
        $latest = @{}

        foreach ($url in $Urls) {
            $stats = (Invoke-KanariRpc -Url $url -Method 'kanari_getStats' -Id 2 -TimeoutSec 3).result
            $latest[$url] = $stats
            [void]$roots.Add([string]$stats.state_root)
            [void]$heights.Add([string]$stats.height)
            [void]$txs.Add([string]$stats.total_transactions)
        }

        if ($roots.Count -eq 1 -and $heights.Count -eq 1 -and $txs.Count -eq 1) {
            foreach ($url in $Urls) {
                $stats = $latest[$url]
                Write-Host "SYNCED $url height=$($stats.height) txs=$($stats.total_transactions) root=$($stats.state_root)"
            }
            return
        }

        Start-Sleep -Seconds 1
    }

    throw "node stats did not converge within ${SyncTimeoutSec}s; logs: $runRoot"
}

$keysDir = Join-Path $runRoot 'consensus-keys'
$genesis = Join-Path $runRoot 'devnet-genesis.json'
$sourceData = Join-Path $runRoot 'node1-db'
New-Item -ItemType Directory -Force -Path $sourceData | Out-Null

Write-Host "Preparing temporary four-node chaos devnet under $runRoot"
$prepareLog = Join-Path $runRoot 'prepare.out.log'
$previousErrorActionPreference = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
try {
    & $nodeExe consensus-keygen --node-count 4 --output-dir $keysDir --force *> $prepareLog
    $exitCode = $LASTEXITCODE
    if ($exitCode -eq 0) {
        & $nodeExe genesis-export --network devnet --data-dir $sourceData --output $genesis *>> $prepareLog
        $exitCode = $LASTEXITCODE
    }
} finally {
    $ErrorActionPreference = $previousErrorActionPreference
}
if ($exitCode -ne 0) {
    Get-Content $prepareLog -Tail 80
    throw "chaos devnet preparation failed with exit code $exitCode; log: $prepareLog"
}

$processes = @{}
$urls = @(1..4 | ForEach-Object { "http://127.0.0.1:$($BaseRpcPort + (($_ - 1) * 10))" })

try {
    foreach ($i in 1..4) {
        $processes[$i] = Start-KanariChaosNode -Index $i
        Start-Sleep -Milliseconds 900
    }

    Wait-RpcReady -Urls $urls
    Wait-StatsConverged -Urls $urls

    Write-Host "Baseline load across all four nodes"
    $loadArgs = @{
        RpcUrl = $urls
        Requests = $Requests
        Concurrency = $Concurrency
        TimeoutSec = $TimeoutSec
        IncludeMalformed = $true
        IncludeOversized = $true
        SkipBuild = $true
    }
    if ($MinRps -gt 0) { $loadArgs.MinRps = $MinRps }
    if ($MaxP99Ms -gt 0) { $loadArgs.MaxP99Ms = $MaxP99Ms }
    if ($MaxEndpointImbalancePercent -gt 0) { $loadArgs.MaxEndpointImbalancePercent = $MaxEndpointImbalancePercent }
    & $loadScript @loadArgs
    if ($LASTEXITCODE -ne 0) {
        throw "baseline RPC load failed with exit code $LASTEXITCODE"
    }

    Write-Host "Stopping follower node 4 to simulate process loss"
    Stop-Process -Id $processes[4].Id -Force
    $processes.Remove(4)
    Start-Sleep -Seconds 2

    Write-Host "Load across surviving three nodes"
    $survivors = $urls[0..2]
    $survivorLoadArgs = $loadArgs.Clone()
    $survivorLoadArgs.RpcUrl = $survivors
    & $loadScript @survivorLoadArgs
    if ($LASTEXITCODE -ne 0) {
        throw "survivor RPC load failed with exit code $LASTEXITCODE"
    }

    Write-Host "Restarting follower node 4 and waiting for convergence"
    $processes[4] = Start-KanariChaosNode -Index 4
    Wait-RpcReady -Urls @($urls[3]) -Attempts 120
    Wait-StatsConverged -Urls $urls

    Write-Host "Four-node chaos exercise complete"
} finally {
    foreach ($process in $processes.Values) {
        try {
            if (-not $process.HasExited) {
                Stop-Process -Id $process.Id -Force
            }
        } catch {
        }
    }

    Write-Host "Run artifacts: $runRoot"
}
