param(
    [ValidateRange(1, 10000000)]
    [int]$Requests = 100000,

    [ValidateRange(1, 10000)]
    [int]$Concurrency = 1024,

    [ValidateRange(1, 120)]
    [int]$TimeoutSec = 5,

    [int]$BaseP2pPort = 19100,

    [int]$BaseRpcPort = 19101,

    [switch]$IncludeMalformed,

    [switch]$IncludeOversized,

    [ValidateRange(0, 1000000000)]
    [double]$MinRps = 0,

    [ValidateRange(0, 3600000)]
    [int]$MaxP99Ms = 0,

    [ValidateRange(0, 100)]
    [double]$MaxClientRejectedPercent = 0,

    [ValidateRange(0, 100)]
    [double]$MaxEndpointImbalancePercent = 0,

    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$nodeExe = Join-Path $repoRoot 'target\release\kanari-node.exe'
$loadScript = Join-Path $repoRoot 'scripts\run-production-rpc-load.ps1'
$runId = Get-Date -Format 'yyyyMMdd-HHmmss'
$runRoot = Join-Path $repoRoot ".codex-runlogs\four-node-rpc-load-$runId"

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

$keysDir = Join-Path $runRoot 'consensus-keys'
$genesis = Join-Path $runRoot 'devnet-genesis.json'
$sourceData = Join-Path $runRoot 'node1-db'
New-Item -ItemType Directory -Force -Path $sourceData | Out-Null

Write-Host "Preparing temporary four-node devnet under $runRoot"
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

    $body = @{ jsonrpc = '2.0'; id = 1; method = 'kanari_health'; params = @{} } | ConvertTo-Json -Compress
    $ready = @{}
    for ($attempt = 1; $attempt -le 120; $attempt++) {
        foreach ($url in $urls) {
            if ($ready[$url]) {
                continue
            }

            try {
                $resp = Invoke-RestMethod -Uri $url -Method Post -ContentType 'application/json' -Body $body -TimeoutSec 2
                if ($resp.result) {
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

    $loadArgs = @{
        RpcUrl = $urls
        Requests = $Requests
        Concurrency = $Concurrency
        TimeoutSec = $TimeoutSec
        SkipBuild = $true
    }
    if ($IncludeMalformed) {
        $loadArgs.IncludeMalformed = $true
    }
    if ($IncludeOversized) {
        $loadArgs.IncludeOversized = $true
    }
    if ($MinRps -gt 0) {
        $loadArgs.MinRps = $MinRps
    }
    if ($MaxP99Ms -gt 0) {
        $loadArgs.MaxP99Ms = $MaxP99Ms
    }
    if ($MaxClientRejectedPercent -gt 0) {
        $loadArgs.MaxClientRejectedPercent = $MaxClientRejectedPercent
    }
    if ($MaxEndpointImbalancePercent -gt 0) {
        $loadArgs.MaxEndpointImbalancePercent = $MaxEndpointImbalancePercent
    }

    & $loadScript @loadArgs
    exit $LASTEXITCODE
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
