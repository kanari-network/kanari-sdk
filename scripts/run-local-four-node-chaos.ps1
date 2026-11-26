[System.Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidUsingPlainTextForPassword', 'Password', Justification = 'Password is required by the kanari CLI --password argument; kept as a string only at the process boundary.')]
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

    [string]$KeystorePath = 'C:\Users\TaroPukpui\.kanari\kanari_config\kanari.keystore',

    [string]$Password = '@Password12345678',

    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'lib\four-node-common.ps1')

$envConfig = Set-KanariFourNodeEnv -KeystorePath $KeystorePath -Password $Password
$KeystorePath = $envConfig.KeystorePath
$Password = $envConfig.Password

$repoRoot = Resolve-KanariFourNodeRoot -ScriptDirectory $PSScriptRoot
$exePaths = Get-KanariFourNodeExePaths -RepoRoot $repoRoot
$nodeExe = $exePaths.NodeExe
$loadScript = Join-Path $repoRoot 'scripts\run-production-rpc-load.ps1'
$run = New-KanariFourNodeRunRoot -RepoRoot $repoRoot -Prefix 'four-node-chaos'
$runRoot = $run.RunRoot

if (-not $SkipBuild) {
    Invoke-KanariFourNodeBuild -RepoRoot $repoRoot -Packages @('kanari-node', 'kanari-rpc-loadgen') -LogDirectory $runRoot
}

if (-not (Test-Path $nodeExe)) {
    throw "Release kanari-node not found at $nodeExe"
}
if (-not (Test-Path $loadScript)) {
    throw "RPC load script not found at $loadScript"
}

Write-Host "Preparing temporary four-node chaos devnet under $runRoot"
$devnet = New-KanariFourNodeGenesis `
    -NodeExe $nodeExe `
    -RepoRoot $repoRoot `
    -RunRoot $runRoot `
    -PrepareLog (Join-Path $runRoot 'prepare.out.log')
$keysDir = $devnet.KeysDir
$genesis = $devnet.Genesis
$peerAuthorityArg = New-KanariPeerAuthorityMap -NodeExe $nodeExe -RunRoot $runRoot
Stop-KanariStaleFourNodeProcesses -BaseP2pPort $BaseP2pPort -BaseRpcPort $BaseRpcPort

$processes = @{}
$urls = Get-KanariFourNodeUrls -BaseRpcPort $BaseRpcPort

try {
    foreach ($i in 1..4) {
        $rpcPort = $BaseRpcPort + (($i - 1) * 10)
        $processes[$i] = Start-KanariValidator `
            -NodeExe $nodeExe `
            -RepoRoot $repoRoot `
            -RunRoot $runRoot `
            -Index $i `
            -BaseP2pPort $BaseP2pPort `
            -RpcPort $rpcPort `
            -Genesis $genesis `
            -KeysDir $keysDir `
            -PeerAuthorityArg $peerAuthorityArg
        Start-Sleep -Milliseconds 900
    }

    Wait-KanariRpcReady -Urls $urls -RunRoot $runRoot
    Wait-KanariStatsConverged -Urls $urls -TimeoutSec $SyncTimeoutSec -RunRoot $runRoot

    Write-Host "Baseline load across all four nodes"
    $loadArgs = @{
        RpcUrl           = $urls
        Requests         = $Requests
        Concurrency      = $Concurrency
        TimeoutSec       = $TimeoutSec
        IncludeMalformed = $true
        IncludeOversized = $true
        SkipBuild        = $true
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
    $processes[4] = Start-KanariValidator `
        -NodeExe $nodeExe `
        -RepoRoot $repoRoot `
        -RunRoot $runRoot `
        -Index 4 `
        -BaseP2pPort $BaseP2pPort `
        -RpcPort ($BaseRpcPort + 30) `
        -Genesis $genesis `
        -KeysDir $keysDir `
        -PeerAuthorityArg $peerAuthorityArg
    Wait-KanariRpcReady -Urls @($urls[3]) -RunRoot $runRoot
    Wait-KanariStatsConverged -Urls $urls -TimeoutSec $SyncTimeoutSec -RunRoot $runRoot

    Write-Host "Four-node chaos exercise complete"
}
finally {
    Stop-KanariProcesses -Processes $processes
    Write-Host "Run artifacts: $runRoot"
}
