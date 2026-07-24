param(
    [Parameter(Mandatory = $true)]
    [string[]]$RpcUrl,

    [ValidateRange(1, 10000000)]
    [int]$Requests = 200000,

    [ValidateRange(1, 10000)]
    [int]$Concurrency = 2048,

    [ValidateRange(1, 120)]
    [int]$TimeoutSec = 5,

    [ValidateRange(0, 1000000000)]
    [double]$MinRps = 10000,

    [ValidateRange(0, 3600000)]
    [int]$MaxP99Ms = 300,

    [ValidateRange(0, 100)]
    [double]$MaxClientRejectedPercent = 10,

    [ValidateRange(0, 100)]
    [double]$MaxEndpointImbalancePercent = 5,

    [switch]$IncludeMalformed,

    [switch]$IncludeOversized,

    [switch]$FailOnRateLimit,

    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$loadScript = Join-Path $repoRoot 'scripts\run-production-rpc-load.ps1'
if (-not (Test-Path -LiteralPath $loadScript)) {
    throw "Missing production RPC load runner: $loadScript"
}

Write-Host "Kanari production benchmark profile"
Write-Host "  machine=$env:COMPUTERNAME os=$([System.Environment]::OSVersion.VersionString)"
Write-Host "  cpu_count=$env:NUMBER_OF_PROCESSORS"
Write-Host "  endpoints=$($RpcUrl -join ', ')"

& $loadScript `
    -RpcUrl $RpcUrl `
    -Requests $Requests `
    -Concurrency $Concurrency `
    -TimeoutSec $TimeoutSec `
    -IncludeMalformed:$IncludeMalformed `
    -IncludeOversized:$IncludeOversized `
    -FailOnRateLimit:$FailOnRateLimit `
    -MinRps $MinRps `
    -MaxP99Ms $MaxP99Ms `
    -MaxClientRejectedPercent $MaxClientRejectedPercent `
    -MaxEndpointImbalancePercent $MaxEndpointImbalancePercent `
    -SkipBuild:$SkipBuild
