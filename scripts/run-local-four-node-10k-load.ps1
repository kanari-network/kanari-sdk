[System.Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidUsingPlainTextForPassword', 'Password', Justification = 'Password is required by the kanari CLI --password argument; kept as a string only at the process boundary.')]
param(
    [string[]]$Senders = @(
        '0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146',
        '0x7e669cf309e9761beb2083d12564895ccea1389f2851a479edcc0d1ad19e4350'
    ),

    [string]$Recipient = '0x7e669cf309e9761beb2083d12564895ccea1389f2851a479edcc0d1ad19e4350',

    [ValidateRange(1, 1000000)]
    [int]$CountPerSender = 5000,

    [ValidateRange(0.000000001, 1000000000)]
    [double]$Amount = 0.000000001,

    [ValidateRange(0, 1000)]
    [int]$TempWalletCount = 0,

    [ValidateRange(1, 1000000)]
    [int]$FaucetCoinsPerSender = 16,

    [ValidateRange(1, 64)]
    [int]$FanoutBatchSize = 64,

    [ValidateRange(30, 7200)]
    [int]$LaneTimeoutSec = 900,

    [switch]$AutoCoinFanout,

    [ValidateRange(16, 65536)]
    [int]$P2pChannelCapacity = 8192,

    [ValidateRange(1, 4096)]
    [int]$MaxConcurrentSyncMessages = 512,

    [ValidateRange(0, 20)]
    [int]$ChaosRounds = 0,

    [ValidateRange(0, 20)]
    [int]$CrashDuringLoadRounds = 0,

    [ValidateSet('follower', 'leader', 'two-node', 'round-robin', 'client-ingress')]
    [string]$CrashDuringLoadPattern = 'follower',

    [ValidateRange(1, 3600)]
    [int]$CrashDuringLoadFirstDelaySec = 10,

    [ValidateRange(1, 3600)]
    [int]$CrashDuringLoadRestartDelaySec = 1,

    [ValidateRange(0, 20)]
    [int]$RecoveryAuditRounds = 0,

    [ValidateRange(0, 3600)]
    [int]$ProfileIntervalSec = 0,

    [switch]$StartLinuxPerfRecorders,

    [ValidateRange(1, 10000)]
    [int]$PerfSampleHz = 99,

    [ValidateRange(1, 3600)]
    [int]$PerfDurationSec = 30,

    [switch]$SelfRecipient,

    [ValidateRange(1, 300)]
    [int]$RootSyncTimeoutSec = 300,

    [int]$BaseP2pPort = 21400,

    [int]$BaseRpcPort = 21401,

    [string]$KeystorePath = 'C:\Users\TaroPukpui\.kanari\kanari_config\kanari.keystore',

    [string]$Password = $env:KANARI_LOAD_PASSWORD,

    [ValidateSet('release', 'debug')]
    [string]$BuildProfile = 'release',

    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'lib\four-node-common.ps1')

$envConfig = Set-KanariFourNodeEnv -KeystorePath $KeystorePath -Password $Password
$KeystorePath = $envConfig.KeystorePath
$Password = $envConfig.Password

$repoRoot = Resolve-KanariFourNodeRoot -ScriptDirectory $PSScriptRoot
$loadScript = Join-Path $repoRoot 'scripts\run-local-four-node-parallel-tx-chaos.ps1'

& $loadScript `
    -Senders $Senders `
    -Recipient $Recipient `
    -SelfRecipient:$SelfRecipient `
    -Password $Password `
    -KeystorePath $KeystorePath `
    -TempWalletCount $TempWalletCount `
    -FundSenders `
    -FaucetAmount 1 `
    -FaucetCoinsPerSender $FaucetCoinsPerSender `
    -FanoutBatchSize $FanoutBatchSize `
    -LaneTimeoutSec $LaneTimeoutSec `
    -AutoCoinFanout:$AutoCoinFanout `
    -P2pChannelCapacity $P2pChannelCapacity `
    -MaxConcurrentSyncMessages $MaxConcurrentSyncMessages `
    -CountPerSender $CountPerSender `
    -Amount $Amount `
    -ChaosRounds $ChaosRounds `
    -CrashDuringLoadRounds $CrashDuringLoadRounds `
    -CrashDuringLoadPattern $CrashDuringLoadPattern `
    -CrashDuringLoadFirstDelaySec $CrashDuringLoadFirstDelaySec `
    -CrashDuringLoadRestartDelaySec $CrashDuringLoadRestartDelaySec `
    -RecoveryAuditRounds $RecoveryAuditRounds `
    -ProfileIntervalSec $ProfileIntervalSec `
    -StartLinuxPerfRecorders:$StartLinuxPerfRecorders `
    -PerfSampleHz $PerfSampleHz `
    -PerfDurationSec $PerfDurationSec `
    -RootSyncTimeoutSec $RootSyncTimeoutSec `
    -BaseP2pPort $BaseP2pPort `
    -BaseRpcPort $BaseRpcPort `
    -BuildProfile $BuildProfile `
    -SkipBuild:$SkipBuild
