param(
    [ValidateRange(10000, 1000000)]
    [int]$Transactions = 10000,

    [ValidateRange(1, 1000000)]
    [int]$Senders = 256,

    [ValidateRange(1, 10)]
    [int]$Runs = 1,

    [ValidateRange(1, 64)]
    [int]$StateCfShards = 8,

    [ValidateRange(1, 256)]
    [int]$Threads = 24,

    [ValidateRange(0, 64)]
    [int]$PinCpuCount = 0,

    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')

if (-not $SkipBuild) {
    Push-Location $repoRoot
    try {
        cargo build --release -p kanari-benchmarks
    }
    finally {
        Pop-Location
    }
}

# Host-level only: these settings affect the benchmark child process. They do
# not change consensus, root semantics, key/value bytes, durability, or code.
$previousBackend = $env:KANARI_BENCH_STATE_BACKEND
$previousShards = $env:KANARI_STATE_CF_SHARDS
$previousThreads = $env:RAYON_NUM_THREADS

try {
    $env:KANARI_BENCH_STATE_BACKEND = 'rocksdb'
    $env:KANARI_STATE_CF_SHARDS = [string]$StateCfShards
    $env:RAYON_NUM_THREADS = [string]$Threads

    $exe = Join-Path $repoRoot 'target\release\kanari-benchmarks.exe'
    if (-not (Test-Path $exe)) {
        throw "Benchmark binary not found: $exe"
    }

    $cpuCount = [Environment]::ProcessorCount
    $pinCount = $PinCpuCount
    if ($pinCount -gt $cpuCount) { $pinCount = $cpuCount }
    $affinityCount = $pinCount
    if ($affinityCount -eq 0) { $affinityCount = $cpuCount }
    $affinityLabel = "all"
    if ($pinCount -gt 0) { $affinityLabel = "first-$pinCount-cpus" }

    Write-Host "Stable persistent benchmark: txs=$Transactions senders=$Senders runs=$Runs cf_shards=$StateCfShards threads=$Threads priority=High affinity=$affinityLabel"

    Push-Location $repoRoot
    try {
        $arguments = @(
            '--mode', 'production',
            '--txs', [string]$Transactions,
            '--senders', [string]$Senders,
            '--runs', [string]$Runs,
            '--json'
        )
        $process = Start-Process -FilePath $exe -ArgumentList $arguments -NoNewWindow -PassThru
        try {
            try {
                $process.PriorityClass = [System.Diagnostics.ProcessPriorityClass]::High
            }
            catch {
                Write-Warning "Could not raise benchmark priority: $($_.Exception.Message)"
            }
            try {
                if ($affinityCount -lt 63) {
                    $mask = [long]0
                    for ($i = 0; $i -lt $affinityCount; $i++) {
                        $mask = $mask -bor ([long]1 -shl $i)
                    }
                    $process.ProcessorAffinity = [IntPtr]$mask
                }
            }
            catch {
                Write-Warning "Could not set benchmark affinity: $($_.Exception.Message)"
            }
        }
        finally {
            $process.WaitForExit()
            $exitCode = $process.ExitCode
            if ($null -ne $exitCode -and $exitCode -ne 0) {
                throw "Benchmark exited with code $exitCode"
            }
        }
    }
    finally {
        Pop-Location
    }
}
finally {
    if ($null -eq $previousBackend) { Remove-Item Env:\KANARI_BENCH_STATE_BACKEND -ErrorAction SilentlyContinue } else { $env:KANARI_BENCH_STATE_BACKEND = $previousBackend }
    if ($null -eq $previousShards) { Remove-Item Env:\KANARI_STATE_CF_SHARDS -ErrorAction SilentlyContinue } else { $env:KANARI_STATE_CF_SHARDS = $previousShards }
    if ($null -eq $previousThreads) { Remove-Item Env:\RAYON_NUM_THREADS -ErrorAction SilentlyContinue } else { $env:RAYON_NUM_THREADS = $previousThreads }
}
