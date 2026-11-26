[System.Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidUsingPlainTextForPassword', 'Password', Justification = 'Password is required by the kanari CLI --password argument; kept as a string only at the process boundary.')]
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
$run = New-KanariFourNodeRunRoot -RepoRoot $repoRoot -Prefix 'four-node-rpc-load'
$runRoot = $run.RunRoot

if (-not $SkipBuild) {
    Invoke-KanariFourNodeBuild -RepoRoot $repoRoot -Packages @('kanari-node', 'kanari-rpc-loadgen') -LogDirectory $runRoot
}

if (-not (Test-Path $nodeExe)) {
    throw "Release kanari-node not found at $nodeExe"
}

Write-Host "Preparing temporary four-node devnet under $runRoot"
$devnet = New-KanariFourNodeGenesis `
    -NodeExe $nodeExe `
    -RepoRoot $repoRoot `
    -RunRoot $runRoot `
    -PrepareLog (Join-Path $runRoot 'prepare.out.log')
$keysDir = $devnet.KeysDir
$genesis = $devnet.Genesis

$peerAuthorityArg = New-KanariPeerAuthorityMap -NodeExe $nodeExe -RunRoot $runRoot
Stop-KanariStaleFourNodeProcesses -BaseP2pPort $BaseP2pPort -BaseRpcPort $BaseRpcPort

$processes = @()
$urls = Get-KanariFourNodeUrls -BaseRpcPort $BaseRpcPort

try {
    for ($i = 1; $i -le 4; $i++) {
        $rpcPort = $BaseRpcPort + (($i - 1) * 10)
        $processes += Start-KanariValidator `
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

    Wait-KanariRpcReady -Urls $urls -Processes $processes -RunRoot $runRoot

    $loadArgs = @{
        RpcUrl      = $urls
        Requests    = $Requests
        Concurrency = $Concurrency
        TimeoutSec  = $TimeoutSec
        SkipBuild   = $true
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
}
finally {
    Stop-KanariProcesses -Processes $processes
    Write-Host "Run artifacts: $runRoot"
}
