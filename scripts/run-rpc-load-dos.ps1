param(
    [string]$RpcUrl = "http://127.0.0.1:6767",
    [ValidateRange(1, 100000)]
    [int]$Requests = 500,
    [ValidateRange(1, 512)]
    [int]$Concurrency = 32,
    [ValidateRange(1, 60)]
    [int]$TimeoutSec = 5,
    [switch]$IncludeMalformed,
    [switch]$IncludeOversized
)

$ErrorActionPreference = 'Stop'

function New-JsonRpcBody {
    param(
        [string]$Method,
        [object]$Params = @{},
        [int]$Id = 1
    )

    return @{
        jsonrpc = "2.0"
        method = $Method
        params = $Params
        id = $Id
    } | ConvertTo-Json -Depth 16 -Compress
}

function Invoke-OneRpcRequest {
    param(
        [string]$RpcUrl,
        [string]$Body,
        [int]$TimeoutSec
    )

    $started = [System.Diagnostics.Stopwatch]::StartNew()
    try {
        $response = Invoke-WebRequest `
            -Uri $RpcUrl `
            -Method Post `
            -Body $Body `
            -ContentType "application/json" `
            -TimeoutSec $TimeoutSec `
            -UseBasicParsing

        $started.Stop()
        return [pscustomobject]@{
            Ok = $true
            StatusCode = [int]$response.StatusCode
            Ms = [int]$started.ElapsedMilliseconds
            Error = $null
        }
    } catch {
        $started.Stop()
        $status = 0
        if ($_.Exception.Response -and $_.Exception.Response.StatusCode) {
            $status = [int]$_.Exception.Response.StatusCode
        }

        return [pscustomobject]@{
            Ok = $false
            StatusCode = $status
            Ms = [int]$started.ElapsedMilliseconds
            Error = $_.Exception.Message
        }
    }
}

Write-Host "Kanari RPC load/DoS probe"
Write-Host "  url=$RpcUrl"
Write-Host "  requests=$Requests concurrency=$Concurrency timeout=${TimeoutSec}s"
Write-Host "  malformed=$IncludeMalformed oversized=$IncludeOversized"

$probeBody = New-JsonRpcBody -Method "kanari_health" -Id 0
$probe = Invoke-OneRpcRequest -RpcUrl $RpcUrl -Body $probeBody -TimeoutSec $TimeoutSec
if (-not $probe.Ok) {
    Write-Host "RPC endpoint is not reachable: $($probe.Error)" -ForegroundColor Red
    exit 2
}

$bodies = New-Object System.Collections.Generic.List[string]
for ($i = 0; $i -lt $Requests; $i++) {
    if ($IncludeMalformed -and ($i % 17 -eq 0)) {
        $bodies.Add("{""jsonrpc"":""2.0"",""method"":""kanari_getStats"",""params"":")
    } elseif ($IncludeOversized -and ($i % 23 -eq 0)) {
        $oversized = "x" * 1048576
        $bodies.Add((New-JsonRpcBody -Method "kanari_getStats" -Params @{ blob = $oversized } -Id $i))
    } elseif ($i % 2 -eq 0) {
        $bodies.Add((New-JsonRpcBody -Method "kanari_health" -Id $i))
    } else {
        $bodies.Add((New-JsonRpcBody -Method "kanari_getStats" -Id $i))
    }
}

$queue = [System.Collections.Queue]::new()
foreach ($body in $bodies) {
    $queue.Enqueue($body)
}

$jobs = @()
$results = New-Object System.Collections.Generic.List[object]
$run = [scriptblock]::Create(@'
param($RpcUrl, $Body, $TimeoutSec)
$started = [System.Diagnostics.Stopwatch]::StartNew()
try {
    $response = Invoke-WebRequest -Uri $RpcUrl -Method Post -Body $Body -ContentType "application/json" -TimeoutSec $TimeoutSec -UseBasicParsing
    $started.Stop()
    [pscustomobject]@{
        Ok = $true
        StatusCode = [int]$response.StatusCode
        Ms = [int]$started.ElapsedMilliseconds
        Error = $null
    }
} catch {
    $started.Stop()
    $status = 0
    if ($_.Exception.Response -and $_.Exception.Response.StatusCode) {
        $status = [int]$_.Exception.Response.StatusCode
    }
    [pscustomobject]@{
        Ok = $false
        StatusCode = $status
        Ms = [int]$started.ElapsedMilliseconds
        Error = $_.Exception.Message
    }
}
'@)

$totalStarted = 0
$sw = [System.Diagnostics.Stopwatch]::StartNew()

while ($queue.Count -gt 0 -or $jobs.Count -gt 0) {
    while ($queue.Count -gt 0 -and $jobs.Count -lt $Concurrency) {
        $body = [string]$queue.Dequeue()
        $jobs += Start-Job -ScriptBlock $run -ArgumentList $RpcUrl, $body, $TimeoutSec
        $totalStarted += 1
    }

    $done = $jobs | Where-Object { $_.State -ne "Running" }
    foreach ($job in $done) {
        $output = Receive-Job $job
        if ($output) {
            $results.Add($output)
        } else {
            $results.Add([pscustomobject]@{
                Ok = $false
                StatusCode = 0
                Ms = 0
                Error = "job produced no result; state=$($job.State)"
            })
        }
        Remove-Job $job -Force
    }
    $doneIds = @($done | ForEach-Object { $_.Id })
    if ($doneIds.Count -gt 0) {
        $jobs = @($jobs | Where-Object { $doneIds -notcontains $_.Id })
    }
    Start-Sleep -Milliseconds 25
}

$sw.Stop()

$success = @($results | Where-Object { $_.Ok -and $_.StatusCode -ge 200 -and $_.StatusCode -lt 300 }).Count
$rateLimited = @($results | Where-Object { $_.StatusCode -eq 429 }).Count
$serverErrors = @($results | Where-Object { $_.StatusCode -ge 500 }).Count
$clientRejected = @($results | Where-Object { $_.StatusCode -ge 400 -and $_.StatusCode -lt 500 -and $_.StatusCode -ne 429 }).Count
$networkErrors = @($results | Where-Object { $_.StatusCode -eq 0 }).Count
$latencies = @($results | ForEach-Object { [int]$_.Ms } | Sort-Object)

function Percentile {
    param([int[]]$Values, [double]$P)
    if ($Values.Count -eq 0) { return 0 }
    $index = [int][Math]::Ceiling(($P / 100.0) * $Values.Count) - 1
    if ($index -lt 0) { $index = 0 }
    if ($index -ge $Values.Count) { $index = $Values.Count - 1 }
    return $Values[$index]
}

$durationSec = [Math]::Max($sw.Elapsed.TotalSeconds, 0.001)
$rps = [Math]::Round($results.Count / $durationSec, 2)

Write-Host "RPC load/DoS probe complete"
Write-Host "  completed=$($results.Count)/$Requests duration=$([Math]::Round($durationSec, 2))s rps=$rps"
Write-Host "  success=$success rate_limited=$rateLimited client_rejected=$clientRejected server_errors=$serverErrors network_errors=$networkErrors"
Write-Host "  latency_ms p50=$(Percentile $latencies 50) p95=$(Percentile $latencies 95) p99=$(Percentile $latencies 99)"

if ($serverErrors -gt 0 -or $networkErrors -gt 0) {
    Write-Host "RPC load/DoS probe found unstable responses." -ForegroundColor Red
    exit 1
}

if ($results.Count -ne $Requests) {
    Write-Host "RPC load/DoS probe did not collect every request result." -ForegroundColor Red
    exit 1
}

Write-Host "RPC load/DoS probe passed." -ForegroundColor Green
