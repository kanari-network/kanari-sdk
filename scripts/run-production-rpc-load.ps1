param(
    [Parameter(Mandatory = $true)]
    [string[]]$RpcUrl,

    [ValidateRange(1, 10000000)]
    [int]$Requests = 100000,

    [ValidateRange(1, 10000)]
    [int]$Concurrency = 1024,

    [ValidateRange(1, 120)]
    [int]$TimeoutSec = 5,

    [switch]$IncludeMalformed,

    [switch]$IncludeOversized,

    [ValidateRange(1024, 16777216)]
    [int]$OversizedBytes = 1048576,

    [switch]$FailOnRateLimit,

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

. (Join-Path $PSScriptRoot 'lib\four-node-common.ps1')

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$loadgen = Join-Path $repoRoot 'target\release\kanari-rpc-loadgen.exe'

Write-Host "Kanari production RPC load run"
Write-Host "  endpoints=$($RpcUrl -join ', ')"
Write-Host "  requests=$Requests concurrency=$Concurrency timeout=${TimeoutSec}s"
Write-Host "  malformed=$IncludeMalformed oversized=$IncludeOversized fail_on_rate_limit=$FailOnRateLimit"
Write-Host "  gates min_rps=$MinRps max_p99_ms=$MaxP99Ms max_client_rejected_percent=$MaxClientRejectedPercent max_endpoint_imbalance_percent=$MaxEndpointImbalancePercent"

if (-not $SkipBuild) {
    Invoke-KanariFourNodeBuild `
        -RepoRoot $repoRoot `
        -Packages @('kanari-rpc-loadgen') `
        -LogDirectory (Join-Path $repoRoot '.kanari-runlogs\rpc-loadgen-build')
}

if (-not (Test-Path $loadgen)) {
    throw "Release load generator not found at $loadgen. Run cargo build -p kanari-rpc-loadgen --release first."
}

$loadArgs = @()
foreach ($url in $RpcUrl) {
    $loadArgs += @('--rpc-url', $url)
}
$loadArgs += @('--requests', $Requests)
$loadArgs += @('--concurrency', $Concurrency)
$loadArgs += @('--timeout-secs', $TimeoutSec)

if ($IncludeMalformed) {
    $loadArgs += @('--malformed-every', 17)
}
if ($IncludeOversized) {
    $loadArgs += @('--oversized-every', 23, '--oversized-bytes', $OversizedBytes)
}
if ($FailOnRateLimit) {
    $loadArgs += '--fail-on-rate-limit'
}
if ($MinRps -gt 0) {
    $loadArgs += @('--min-rps', $MinRps)
}
if ($MaxP99Ms -gt 0) {
    $loadArgs += @('--max-p99-ms', $MaxP99Ms)
}
if ($MaxClientRejectedPercent -gt 0) {
    $loadArgs += @('--max-client-rejected-percent', $MaxClientRejectedPercent)
}
if ($MaxEndpointImbalancePercent -gt 0) {
    $loadArgs += @('--max-endpoint-imbalance-percent', $MaxEndpointImbalancePercent)
}

$loadLogRoot = Join-Path $repoRoot '.kanari-runlogs\rpc-loadgen'
New-Item -ItemType Directory -Force -Path $loadLogRoot | Out-Null
$loadProcess = Start-KanariLoggedProcess -Launch @{
    FilePath               = $loadgen
    ArgumentList           = $loadArgs
    WorkingDirectory       = $repoRoot
    RedirectStandardOutput = (Join-Path $loadLogRoot 'loadgen.out.log')
    RedirectStandardError  = (Join-Path $loadLogRoot 'loadgen.err.log')
    Wait                   = $true
}
exit [int]$loadProcess.ExitCode
