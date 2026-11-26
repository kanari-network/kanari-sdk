[System.Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidUsingPlainTextForPassword', 'Password', Justification = 'Password is required by the kanari CLI --password argument; kept as a string only at the process boundary.')]
param(
    [string]$From = "0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146",

    [string]$To = "0x7e669cf309e9761beb2083d12564895ccea1389f2851a479edcc0d1ad19e4350",

    [string]$Password = $env:KANARI_LOAD_PASSWORD,

    [string]$KeystorePath = 'C:\Users\TaroPukpui\.kanari\kanari_config\kanari.keystore',

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

. (Join-Path $PSScriptRoot 'lib\four-node-common.ps1')

$envConfig = Set-KanariFourNodeEnv -KeystorePath $KeystorePath -Password $Password
$KeystorePath = $envConfig.KeystorePath
$Password = $envConfig.Password

$repoRoot = Resolve-KanariFourNodeRoot -ScriptDirectory $PSScriptRoot
$exePaths = Get-KanariFourNodeExePaths -RepoRoot $repoRoot
$nodeExe = $exePaths.NodeExe
$kanariExe = $exePaths.KanariExe
$run = New-KanariFourNodeRunRoot -RepoRoot $repoRoot -Prefix 'four-node-tx-load'
$runRoot = $run.RunRoot

if (-not $SkipBuild) {
    Invoke-KanariFourNodeBuild -RepoRoot $repoRoot -LogDirectory $runRoot
}

if (-not (Test-Path $nodeExe)) {
    throw "Release kanari-node not found at $nodeExe"
}
if (-not (Test-Path $kanariExe)) {
    throw "Release kanari CLI not found at $kanariExe"
}

Write-Host "Preparing temporary four-node transaction devnet under $runRoot"
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

    $before = Get-KanariStatsByUrl -Urls $urls -Id 2

    Write-Host "Submitting transaction load through $($urls[0])"
    $txArgs = @(
        'client', 'stress-test',
        '--from', $From,
        '--to', $To,
        '--amount', (Format-KanariAmount -Amount $Amount),
        '--count', $Count,
        '--password', $Password,
        '--rpc', $urls[0]
    )
    $txLog = Join-Path $runRoot 'tx-load.out.log'
    $txErrLog = Join-Path $runRoot 'tx-load.err.log'
    $txProcess = Start-KanariLoggedProcess -Launch @{
        FilePath               = $kanariExe
        ArgumentList           = $txArgs
        WorkingDirectory       = $repoRoot
        RedirectStandardOutput = $txLog
        RedirectStandardError  = $txErrLog
        Wait                   = $true
    }
    $txExit = [int]$txProcess.ExitCode
    if ($txExit -ne 0) {
        Get-Content -LiteralPath $txLog -Tail 80
        Get-Content -LiteralPath $txErrLog -Tail 80
        throw "kanari client stress-test failed with exit code $txExit; logs: $txLog $txErrLog"
    }
    Get-Content -LiteralPath $txLog -Tail 20
    Get-Content -LiteralPath $txErrLog -Tail 20

    try {
        $finalStats = Wait-KanariStatsConverged `
            -Urls $urls `
            -TimeoutSec $RootSyncTimeoutSec `
            -RunRoot $runRoot `
            -Quiet `
            -BeforeStats $before `
            -MinTxDelta $Count `
            -MinHeightDelta 1 `
            -PassThru
    }
    catch {
        Write-Host "Final stats did not converge before timeout:" -ForegroundColor Yellow
        Show-KanariStats -Urls $urls
        throw "four-node transaction state root did not converge within ${RootSyncTimeoutSec}s"
    }

    Write-Host "Four-node transaction load complete"
    foreach ($url in $urls) {
        $stats = $finalStats[$url]
        $beforeStats = $before[$url]
        $deltaTx = [int64]$stats.total_transactions - [int64]$beforeStats.total_transactions
        Write-Host "  $url height=$($stats.height) txs=$($stats.total_transactions) delta_txs=$deltaTx root=$($stats.state_root)"
    }
}
finally {
    Stop-KanariProcesses -Processes $processes
    Write-Host "Run artifacts: $runRoot"
}