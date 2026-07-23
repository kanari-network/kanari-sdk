param(
    [string]$From = "0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146",

    [string]$To = "0x3141a487d7a5382bb435c0ad39a6060067765e60e45b50953a0050bcf24b03a3",

    [Parameter(Mandatory = $true)]
    [string]$Password,

    [ValidateRange(0.000000001, 1000000000)]
    [double]$Amount = 0.000000001,

    [ValidateRange(1, 1000000)]
    [int]$Count = 10,

    [int]$BaseP2pPort = 19300,

    [int]$BaseRpcPort = 19301,

    [ValidateRange(1, 300)]
    [int]$RootSyncTimeoutSec = 60,

    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$nodeExe = Join-Path $repoRoot 'target\release\kanari-node.exe'
$kanariExe = Join-Path $repoRoot 'target\release\kanari.exe'
$runId = Get-Date -Format 'yyyyMMdd-HHmmss'
$runRoot = Join-Path $repoRoot ".codex-runlogs\four-node-tx-load-$runId"

New-Item -ItemType Directory -Force -Path $runRoot | Out-Null

if (-not $SkipBuild) {
    Push-Location $repoRoot
    try {
        cargo build -p kanari-node --release
        cargo build -p kanari --release
    } finally {
        Pop-Location
    }
}

if (-not (Test-Path $nodeExe)) {
    throw "Release kanari-node not found at $nodeExe"
}
if (-not (Test-Path $kanariExe)) {
    throw "Release kanari CLI not found at $kanariExe"
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

$keysDir = Join-Path $runRoot 'consensus-keys'
$genesis = Join-Path $runRoot 'devnet-genesis.json'
$sourceData = Join-Path $runRoot 'node1-db'
New-Item -ItemType Directory -Force -Path $sourceData | Out-Null

Write-Host "Preparing temporary four-node transaction devnet under $runRoot"
$prepareLog = Join-Path $runRoot 'prepare.out.log'
$previousErrorActionPreference = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
try {
    & $nodeExe consensus-keygen --node-count 4 --output-dir $keysDir --force *> $prepareLog
    $exitCode = $LASTEXITCODE
} finally {
    $ErrorActionPreference = $previousErrorActionPreference
}
if ($exitCode -ne 0) {
    Get-Content $prepareLog -Tail 80
    throw "consensus-keygen failed with exit code $exitCode; log: $prepareLog"
}

$previousErrorActionPreference = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
try {
    & $nodeExe genesis-export --network devnet --data-dir $sourceData --output $genesis *>> $prepareLog
    $exitCode = $LASTEXITCODE
} finally {
    $ErrorActionPreference = $previousErrorActionPreference
}
if ($exitCode -ne 0) {
    Get-Content $prepareLog -Tail 80
    throw "genesis-export failed with exit code $exitCode; log: $prepareLog"
}

$authorities = '0x1,0x2,0x3,0x4'
$processes = @()
$urls = @()

try {
    for ($i = 1; $i -le 4; $i++) {
        $offset = ($i - 1) * 10
        $p2pPort = $BaseP2pPort + $offset
        $rpcPort = $BaseRpcPort + $offset
        $dataDir = Join-Path $runRoot "node$i-db"
        New-Item -ItemType Directory -Force -Path $dataDir | Out-Null

        $nodeArgs = @(
            'start',
            '--network', 'devnet',
            '--p2p-port', $p2pPort,
            '--rpc-port', $rpcPort,
            '--rpc-host', '127.0.0.1',
            '--data-dir', $dataDir,
            '--authority-id', "0x$i",
            '--authorities', $authorities,
            '--genesis', $genesis,
            '--consensus-private-key-file', (Join-Path $keysDir "node$i-consensus-private-key.key"),
            '--consensus-public-keys', (Join-Path $keysDir 'consensus-public-keys.json')
        )

        if ($i -ne 1) {
            $nodeArgs += @('--bootstrap', "/ip4/127.0.0.1/tcp/$BaseP2pPort")
        }

        $out = Join-Path $runRoot "node$i.out.log"
        $err = Join-Path $runRoot "node$i.err.log"
        $processes += Start-Process `
            -FilePath $nodeExe `
            -ArgumentList $nodeArgs `
            -WorkingDirectory $repoRoot `
            -RedirectStandardOutput $out `
            -RedirectStandardError $err `
            -WindowStyle Hidden `
            -PassThru

        $urls += "http://127.0.0.1:$rpcPort"
        Start-Sleep -Milliseconds 900
    }

    $ready = @{}
    for ($attempt = 1; $attempt -le 120; $attempt++) {
        foreach ($url in $urls) {
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

        if ($ready.Count -eq $urls.Count) {
            break
        }

        foreach ($process in $processes) {
            if ($process.HasExited) {
                throw "node process exited early pid=$($process.Id) code=$($process.ExitCode); logs: $runRoot"
            }
        }

        Start-Sleep -Seconds 1
    }

    if ($ready.Count -ne $urls.Count) {
        throw "only $($ready.Count)/$($urls.Count) RPC endpoints became ready; logs: $runRoot"
    }

    $before = @{}
    foreach ($url in $urls) {
        $before[$url] = (Invoke-KanariRpc -Url $url -Method 'kanari_getStats' -Id 2).result
    }

    Write-Host "Submitting transaction load through $($urls[0])"
    $txArgs = @(
        'client', 'stress-test',
        '--from', $From,
        '--to', $To,
        '--amount', $Amount,
        '--count', $Count,
        '--password', $Password,
        '--rpc', $urls[0]
    )
    $txLog = Join-Path $runRoot 'tx-load.out.log'
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        & $kanariExe @txArgs *> $txLog
        $exitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    if ($exitCode -ne 0) {
        Get-Content $txLog -Tail 80
        throw "kanari client stress-test failed with exit code $exitCode; log: $txLog"
    }
    Get-Content $txLog -Tail 20

    $finalStats = $null
    $deadline = (Get-Date).AddSeconds($RootSyncTimeoutSec)
    while ((Get-Date) -lt $deadline) {
        $statsByUrl = @{}
        $roots = New-Object System.Collections.Generic.HashSet[string]
        $heights = New-Object System.Collections.Generic.HashSet[string]
        $txs = New-Object System.Collections.Generic.HashSet[string]

        foreach ($url in $urls) {
            $stats = (Invoke-KanariRpc -Url $url -Method 'kanari_getStats' -Id 3).result
            $statsByUrl[$url] = $stats
            [void]$roots.Add([string]$stats.state_root)
            [void]$heights.Add([string]$stats.height)
            [void]$txs.Add([string]$stats.total_transactions)
        }

        if ($roots.Count -eq 1 -and $heights.Count -eq 1 -and $txs.Count -eq 1) {
            $height = [int64]($statsByUrl[$urls[0]].height)
            $txCount = [int64]($statsByUrl[$urls[0]].total_transactions)
            $beforeTx = [int64]($before[$urls[0]].total_transactions)
            if ($txCount -ge ($beforeTx + $Count) -and $height -gt [int64]($before[$urls[0]].height)) {
                $finalStats = $statsByUrl
                break
            }
        }

        Start-Sleep -Seconds 1
    }

    if (-not $finalStats) {
        Write-Host "Final stats did not converge before timeout:" -ForegroundColor Yellow
        foreach ($url in $urls) {
            $stats = (Invoke-KanariRpc -Url $url -Method 'kanari_getStats' -Id 4).result
            Write-Host "  $url height=$($stats.height) txs=$($stats.total_transactions) root=$($stats.state_root)"
        }
        throw "four-node transaction state root did not converge within ${RootSyncTimeoutSec}s"
    }

    Write-Host "Four-node transaction load complete"
    foreach ($url in $urls) {
        $stats = $finalStats[$url]
        $beforeStats = $before[$url]
        $deltaTx = [int64]$stats.total_transactions - [int64]$beforeStats.total_transactions
        Write-Host "  $url height=$($stats.height) txs=$($stats.total_transactions) delta_txs=$deltaTx root=$($stats.state_root)"
    }
} finally {
    foreach ($process in $processes) {
        try {
            if (-not $process.HasExited) {
                Stop-Process -Id $process.Id -Force
            }
        } catch {
        }
    }

    Write-Host "Run artifacts: $runRoot"
}
