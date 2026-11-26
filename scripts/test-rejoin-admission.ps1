[System.Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidUsingPlainTextForPassword', 'Password', Justification = 'Password is required by the kanari CLI --password argument; kept as a string only at the process boundary.')]
param(
    [string]$From = "0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146",

    [string]$To = "0x7e669cf309e9761beb2083d12564895ccea1389f2851a479edcc0d1ad19e4350",

    [string]$KeystorePath = "C:\Users\TaroPukpui\.kanari\kanari_config\kanari.keystore",

    [string]$Password = "@Password12345678",

    [ValidateRange(0.000000001, 1000000000)]
    [double]$Amount = 0.000000001,

    [ValidateRange(1, 100)]
    [int]$Count = 5,

    # Env consumed by kanari-core: observed_peer_height > local_height + Lag
    # means the node rejects local tx admission with a retryable error.
    [ValidateRange(1, 1000)]
    [int]$SyncAdmissionLag = 5,

    [ValidateRange(1024, 65505)]
    [int]$BaseP2pPort = 19600,

    [ValidateRange(1024, 65505)]
    [int]$BaseRpcPort = 19601,

    [ValidateRange(5, 300)]
    [int]$RpcTimeoutSec = 3,

    [ValidateRange(1, 10)]
    [int]$RawRpcTimeoutSec = 2,

    [ValidateRange(1024, 65505)]
    [int]$CaptureProxyPort = 19690,

    [ValidateRange(1, 600)]
    [int]$SyncTimeoutSec = 180
)

$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'lib\four-node-common.ps1')

# Scenario-only environment. The CLI loads the keystore/password from the
# shared helper, and every spawned node inherits the admission lag from here.
$envConfig = Set-KanariFourNodeEnv -KeystorePath $KeystorePath -Password $Password
$KeystorePath = $envConfig.KeystorePath
$Password = $envConfig.Password
$env:KANARI_SYNC_ADMISSION_LAG = "$SyncAdmissionLag"

$repoRoot = Resolve-KanariFourNodeRoot -ScriptDirectory $PSScriptRoot
$exePaths = Get-KanariFourNodeExePaths -RepoRoot $repoRoot
$nodeExe = $exePaths.NodeExe
$kanariExe = $exePaths.KanariExe
$runRoot = (New-KanariFourNodeRunRoot -RepoRoot $repoRoot -Prefix 'rejoin-admission').RunRoot
$scenarioLog = Join-Path $runRoot 'scenario.log'
$summaryPath = Join-Path $runRoot 'scenario-result.txt'

function Write-Log {
    param([string]$Message)
    $line = "[{0}] {1}" -f (Get-Date -Format 'HH:mm:ss'), $Message
    Write-Host $line
    Add-Content -LiteralPath $scenarioLog -Value $line
}

function Invoke-ScenarioCli {
    param(
        [string[]]$CliArgs,
        [string]$LogPrefix,
        [int]$TimeoutSec
    )

    return Invoke-KanariCliBounded -Launch @{
        FilePath         = $kanariExe
        ArgumentList     = $CliArgs
        WorkingDirectory = $repoRoot
        OutLog           = "$LogPrefix.out.log"
        ErrLog           = "$LogPrefix.err.log"
    } -TimeoutSec $TimeoutSec
}

function Submit-OneTx {
    param(
        [string]$Url,
        [string]$LogPrefix,
        [int]$TimeoutSec
    )

    $amountText = Format-KanariAmount -Amount $Amount
    $cliArgs = @(
        'client', 'stress-test',
        '--from', $From,
        '--to', $To,
        '--amount', $amountText,
        '--count', '1',
        '--password', $Password,
        '--rpc', $Url,
        '--commit-timeout-sec', '15'
    )
    return Invoke-ScenarioCli -CliArgs $cliArgs -LogPrefix $LogPrefix -TimeoutSec $TimeoutSec
}

function Start-SubmitCaptureProxy {
    param(
        [int]$Port,
        [string]$TargetUrl,
        [string]$CaptureFile,
        [bool]$DropSubmit = $false
    )

    return Start-Job -ScriptBlock {
        param($ListenPort, $ForwardUrl, $OutFile, $DropCapturedSubmit)

        $listener = New-Object System.Net.HttpListener
        $listener.Prefixes.Add("http://127.0.0.1:$ListenPort/")
        $listener.Start()
        try {
            while (-not (Test-Path -LiteralPath $OutFile)) {
                $context = $listener.GetContext()
                $reader = New-Object System.IO.StreamReader($context.Request.InputStream, $context.Request.ContentEncoding)
                $body = $reader.ReadToEnd()
                try {
                    $parsed = $body | ConvertFrom-Json
                    $reply = $null
                    if ($parsed.method -eq 'kanari_submitObjectTransfer') {
                        [System.IO.File]::WriteAllText($OutFile, $body)
                        if ($DropCapturedSubmit) {
                            # Keep the signed payload fresh: acknowledge it locally
                            # but never let the healthy peers execute it.
                            $reply = [pscustomobject]@{
                                jsonrpc = '2.0'
                                id      = $parsed.id
                                result  = [pscustomobject]@{
                                    hash      = [string]$parsed.params.transaction_hash
                                    status    = 'pending'
                                    action    = 'submit'
                                    success   = $true
                                    previewed = $false
                                    submitted = $true
                                    committed = $false
                                }
                            } | ConvertTo-Json -Depth 8 -Compress
                        }
                    }
                    if ($null -eq $reply) {
                        $upstream = Invoke-RestMethod -Uri $ForwardUrl -Method Post -ContentType 'application/json' -Body $body -TimeoutSec 10
                        $reply = $upstream | ConvertTo-Json -Depth 64 -Compress
                    }
                }
                catch {
                    $reply = [pscustomobject]@{
                        jsonrpc = '2.0'
                        id      = 1
                        result  = $null
                        error   = [pscustomobject]@{ code = -32603; message = "capture proxy: $_" }
                    } | ConvertTo-Json -Depth 8 -Compress
                }
                $bytes = [System.Text.Encoding]::UTF8.GetBytes($reply)
                $context.Response.StatusCode = 200
                $context.Response.ContentType = 'application/json'
                $context.Response.ContentLength64 = $bytes.Length
                $context.Response.OutputStream.Write($bytes, 0, $bytes.Length)
                $context.Response.Close()
            }
        }
        finally {
            $listener.Stop()
            $listener.Close()
        }
    } -ArgumentList $Port, $TargetUrl, $CaptureFile, $DropSubmit
}

function Read-SharedText {
    param([string]$Path)

    $stream = [System.IO.File]::Open(
        $Path,
        [System.IO.FileMode]::Open,
        [System.IO.FileAccess]::Read,
        [System.IO.FileShare]::ReadWrite
    )
    $reader = New-Object System.IO.StreamReader($stream)
    try {
        return $reader.ReadToEnd()
    }
    finally {
        $reader.Dispose()
    }
}

function Invoke-RawSubmit {
    param(
        [string]$Url,
        [string]$Payload,
        [int]$TimeoutSec = 2
    )

    try {
        $response = Invoke-RestMethod -Uri $Url -Method Post -ContentType 'application/json' -Body $Payload -TimeoutSec $TimeoutSec
        return [pscustomobject]@{
            HttpStatus = 200
            Code       = if ($response.error) { $response.error.code } else { 0 }
            Message    = if ($response.error) { $response.error.message } else { ($response | ConvertTo-Json -Depth 12 -Compress) }
        }
    }
    catch {
        $details = if ($_.ErrorDetails.Message) { $_.ErrorDetails.Message } else { $_.Exception.Message }
        $code = '(http error)'
        $message = $details
        try {
            $parsed = $details | ConvertFrom-Json
            if ($parsed.error) {
                $code = $parsed.error.code
                $message = $parsed.error.message
            }
        }
        catch {
        }
        return [pscustomobject]@{
            HttpStatus = if ($_.Exception.Response) { [int]$_.Exception.Response.StatusCode } else { -1 }
            Code       = $code
            Message    = $message
        }
    }
}

if (-not (Test-Path -LiteralPath $nodeExe)) {
    throw "Release kanari-node not found at $nodeExe (no rebuild is performed by this script)."
}
if (-not (Test-Path -LiteralPath $kanariExe)) {
    throw "Release kanari CLI not found at $kanariExe (no rebuild is performed by this script)."
}
if (-not (Test-Path -LiteralPath $KeystorePath)) {
    throw "Keystore not found: $KeystorePath"
}

$nodes = @()
$resultState = 'FAIL'
$failureDetail = ''
$statsBeforeStop = @{}
$statsBeforeRejoin = @{}
$statsAtReject = @{}
$statsFinal = @{}
$rejectError = '(not observed)'
$rejectExitCode = '(not observed)'
$proxyJob = $null
$captureFile = Join-Path $runRoot 'captured-submit.json'

Write-Log "Rejoin admission scenario started; artifacts=$runRoot"
Write-Log "env KANARI_KEYSTORE_PATH=$env:KANARI_KEYSTORE_PATH"
Write-Log "env KANARI_LOAD_PASSWORD=<set>"
Write-Log "env KANARI_SYNC_ADMISSION_LAG=$env:KANARI_SYNC_ADMISSION_LAG"

try {
    $devnet = New-KanariFourNodeGenesis `
        -NodeExe $nodeExe `
        -RepoRoot $repoRoot `
        -RunRoot $runRoot `
        -PrepareLog (Join-Path $runRoot 'prepare.out.log') `
        -PrepareErrLog (Join-Path $runRoot 'prepare.err.log')
    $keysDir = $devnet.KeysDir
    $genesis = $devnet.Genesis

    # Validators require an explicit peer-to-authority map. The shared helper
    # generates all four identities; node 4 rejoins fresh, so only 1-3 stay
    # trusted as height sources after the gap is created.
    $peerAuthorityArg = New-KanariPeerAuthorityMap -NodeExe $nodeExe -RunRoot $runRoot
    $peerAuthorityActiveArg = (($peerAuthorityArg -split ',') | Where-Object { $_ -notmatch '=0x4$' }) -join ','
    Write-Log "Active peer-authority mappings for nodes 1-3: $peerAuthorityActiveArg"

    Stop-KanariStaleFourNodeProcesses -BaseP2pPort $BaseP2pPort -BaseRpcPort $BaseRpcPort

    for ($i = 1; $i -le 4; $i++) {
        $rpcPort = $BaseRpcPort + (($i - 1) * 10)
        $nodes += Start-KanariValidator `
            -NodeExe $nodeExe `
            -RepoRoot $repoRoot `
            -RunRoot $runRoot `
            -Index $i `
            -BaseP2pPort $BaseP2pPort `
            -RpcPort $rpcPort `
            -Genesis $genesis `
            -KeysDir $keysDir `
            -PeerAuthorityArg $peerAuthorityArg `
            -PassThruDetails
        Start-Sleep -Milliseconds 900
    }

    Wait-KanariRpcReady -Urls @($nodes | ForEach-Object { $_.Url }) -Processes @($nodes | ForEach-Object { $_.Process }) -Attempts 90 -RunRoot $runRoot

    # Capture baseline, then take node 4 down before generating the height gap.
    $statsBeforeStop = Get-KanariStatsByUrl -Urls @($nodes | ForEach-Object { $_.Url }) -Id 4 -TimeoutSec $RpcTimeoutSec
    Write-Log "Before stopping node4: $((Format-KanariStats -StatsByUrl $statsBeforeStop) -join ' | ')"

    $node4 = $nodes | Where-Object { $_.Name -eq 'node4' }

    # The admission gate only trusts heights from libp2p peers explicitly
    # mapped to validator authorities. Keep the consensus committee untouched:
    # this mapping is injected into SyncManager only and never changes quorum.
    # Node 4 rejoins with a fresh identity, so only nodes 1-3 are trusted sources.
    Write-Log "Active peer-authority mappings: $peerAuthorityActiveArg"

    Stop-Process -Id $node4.Process.Id -Force
    [void]$node4.Process.WaitForExit(10000)
    Write-Log "Stopped node4 (pid $($node4.Process.Id)); active nodes: node1,node2,node3"

    $active = @($nodes | Where-Object { $_.Name -ne 'node4' })
    $activeUrls = @($active | ForEach-Object { $_.Url })
    $baselineTx = @{}
    foreach ($url in $activeUrls) {
        $baselineTx[$url] = [int64]$statsBeforeStop[$url].total_transactions
    }

    $amountText = Format-KanariAmount -Amount $Amount
    $submitArgs = @(
        'client', 'stress-test',
        '--from', $From,
        '--to', $To,
        '--amount', $amountText,
        '--count', $Count,
        '--password', $Password,
        '--rpc', $activeUrls[0]
    )
    Write-Log "Submitting $Count initial tx(s) via $($activeUrls[0])"
    $initial = Invoke-ScenarioCli -CliArgs $submitArgs -LogPrefix (Join-Path $runRoot 'initial-tx') -TimeoutSec 180
    if ($initial.ExitCode -ne 0) {
        Write-Host $initial.Text
        throw "initial transaction load via node1 failed with exit code $($initial.ExitCode)"
    }

    $firstActiveUrl = $activeUrls[0]
    $initialMinTx = @{
        $firstActiveUrl = [int64]$baselineTx[$firstActiveUrl] + $Count
    }
    $statsBeforeRejoin = Wait-KanariStatsConverged -Urls $activeUrls -MinHeight ([int64]$statsBeforeStop[$firstActiveUrl].height + 1) -MinTxByUrl $initialMinTx -TimeoutSec $SyncTimeoutSec -PassThru -Label 'nodes 1-3 convergence before rejoin'

    # A fresh node starts at height 0. Keep the gap well above the lag so the
    # gate stays armed while many checkpoints are replayed.
    $targetGapHeight = [int64]$SyncAdmissionLag + 30
    $extraRound = 0
    while ([int64]($statsBeforeRejoin[$activeUrls[0]].height) -lt $targetGapHeight -and $extraRound -lt 3) {
        $extraCount = [int]($targetGapHeight - [int64]($statsBeforeRejoin[$activeUrls[0]].height))
        foreach ($url in $activeUrls) {
            $baselineTx[$url] = [int64]($statsBeforeRejoin[$url].total_transactions)
        }
        $extraArgs = @(
            'client', 'stress-test',
            '--from', $From,
            '--to', $To,
            '--amount', $amountText,
            '--count', $extraCount,
            '--password', $Password,
            '--rpc', $activeUrls[0]
        )
        Write-Log "Increasing observed height gap by $extraCount tx(s) via $($activeUrls[0])"
        $extra = Invoke-ScenarioCli -CliArgs $extraArgs -LogPrefix (Join-Path $runRoot "gap-tx-$extraRound") -TimeoutSec 180
        if ($extra.ExitCode -ne 0) {
            Write-Host $extra.Text
            throw "gap transaction batch failed with exit code $($extra.ExitCode)"
        }
        $gapMinTx = @{}
        foreach ($url in $activeUrls) {
            $gapMinTx[$url] = [int64]($baselineTx[$url] + $extraCount)
        }
        $statsBeforeRejoin = Wait-KanariStatsConverged -Urls $activeUrls -MinHeight ([int64]($statsBeforeRejoin[$activeUrls[0]].height) + 1) -MinTxByUrl $gapMinTx -TimeoutSec $SyncTimeoutSec -PassThru -Label 'nodes 1-3 convergence after gap batch'
        $extraRound++
    }
    if ([int64]($statsBeforeRejoin[$activeUrls[0]].height) -le [int64]$SyncAdmissionLag) {
        throw "could not build a height gap above KANARI_SYNC_ADMISSION_LAG=$SyncAdmissionLag"
    }
    Write-Log "Before rejoin: $((Format-KanariStats -StatsByUrl $statsBeforeRejoin) -join ' | ')"

    # Capture a fully signed but NOT executed submit payload through a local
    # proxy: the proxy acknowledges the submit itself, so the payload is fresh
    # everywhere and cannot be rejected as "already executed" on replay.
    $proxyJob = Start-SubmitCaptureProxy -Port $CaptureProxyPort -TargetUrl $activeUrls[0] -CaptureFile $captureFile -DropSubmit $true
    Start-Sleep -Milliseconds 800
    if ($proxyJob.State -eq 'Failed') {
        $proxyErrors = ($proxyJob | Receive-Job | Out-String)
        throw "capture proxy failed to start: $proxyErrors"
    }
    $captureArgs = @(
        'client', 'stress-test',
        '--from', $From,
        '--to', $To,
        '--amount', $amountText,
        '--count', '1',
        '--password', $Password,
        '--rpc', "http://127.0.0.1:$CaptureProxyPort",
        '--commit-timeout-sec', '5'
    )
    $captured = Invoke-ScenarioCli -CliArgs $captureArgs -LogPrefix (Join-Path $runRoot 'capture-tx') -TimeoutSec 60
    if (-not (Test-Path -LiteralPath $captureFile)) {
        Write-Host $captured.Text
        throw "capture proxy did not record a kanari_submitObjectTransfer payload; CLI exit=$($captured.ExitCode)"
    }
    Stop-Job -Job $proxyJob -ErrorAction SilentlyContinue
    Remove-Job -Job $proxyJob -Force -ErrorAction SilentlyContinue
    $proxyJob = $null
    $capturedPayload = [System.IO.File]::ReadAllText($captureFile)
    Write-Log "Captured fresh signed payload (CLI exit $($captured.ExitCode), commit is intentionally unobservable)"


    # Rejoin node 4 from an EMPTY data dir: no raw DB copy, only genesis + sync.
    if (Test-Path -LiteralPath $node4.DataDir) {
        Remove-Item -LiteralPath $node4.DataDir -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $node4.DataDir | Out-Null
    $rejoinOut = Join-Path $runRoot 'node4.rejoin.out.log'
    $rejoinErr = Join-Path $runRoot 'node4.rejoin.err.log'
    # One sync message at a time plus a short publish delay keeps the catch-up
    # gate observable. This only changes scenario timing, never consensus or the
    # trusted peer map. The helper restores both variables after process spawn.
    $node4.Process = (Start-KanariValidator `
            -NodeExe $nodeExe `
            -RepoRoot $repoRoot `
            -RunRoot $runRoot `
            -Index 4 `
            -BaseP2pPort $BaseP2pPort `
            -RpcPort ($BaseRpcPort + 30) `
            -Genesis $genesis `
            -KeysDir $keysDir `
            -PeerAuthorityArg $peerAuthorityActiveArg `
            -LogSuffix 'rejoin' `
            -Environment @{
            KANARI_MAX_CONCURRENT_SYNC_MESSAGES = '1'
            KANARI_CHAOS_P2P_PUBLISH_DELAY_MS   = '100'
        } `
            -PassThruDetails).Process
    Write-Log "Rejoined node4 with empty data dir (pid $($node4.Process.Id)); lag=$SyncAdmissionLag; peer-authorities registered"

    # The sync loop logs the trusted peer height before applying buffered
    # checkpoints. Probe as soon as that line appears so the gate is still armed.
    Wait-KanariRpcReady -Urls @($node4.Url) -Processes @($node4.Process) -Attempts 60 -RunRoot $runRoot
    Write-Log "READY node4 rejoin $($node4.Url)"
    $gateArmed = $false
    $syncDeadline = (Get-Date).AddSeconds(20)
    while ((Get-Date) -lt $syncDeadline) {
        if (Test-Path -LiteralPath $rejoinOut) {
            $seen = Select-String -LiteralPath $rejoinOut -Pattern 'Applied all buffered checkpoints but still behind network \(current: \d+, max seen: (\d+)\)' | Select-Object -Last 1
            if ($seen -and [int64]$seen.Matches[0].Groups[1].Value -gt [int64]$SyncAdmissionLag) {
                $gateArmed = $true
                break
            }
        }
        Start-Sleep -Milliseconds 5
    }
    if (-not $gateArmed) {
        throw "node4 did not observe a trusted peer height gap within 20s; logs: $rejoinOut / $rejoinErr"
    }
    # That log line comes from the checkpoint handler; the periodic sync loop
    # publishes the observed height on its own 500ms tick.
    Start-Sleep -Milliseconds 700
    $raw = $null
    $probes = 0
    $probeDeadline = (Get-Date).AddSeconds(20)
    while ((Get-Date) -lt $probeDeadline) {
        $probes++
        $raw = Invoke-RawSubmit -Url $node4.Url -Payload $capturedPayload -TimeoutSec $RawRpcTimeoutSec
        if ($raw.Message -match 'catching up with network' -and $raw.Message -match 'retry after sync') {
            break
        }
        if ($raw.Code -eq 0 -or $raw.Message -match 'already executed') {
            break
        }
        Start-Sleep -Milliseconds 5
    }
    if ($null -eq $raw) {
        throw "rejoined node4 RPC did not accept admission probes within 20s; logs: $rejoinOut / $rejoinErr"
    }
    $rejectExitCode = "http=$($raw.HttpStatus) rpc_code=$($raw.Code) probes=$probes"
    $rejectError = [string]$raw.Message
    Set-Content -LiteralPath (Join-Path $runRoot 'reject-tx.response.json') -Value ($raw | ConvertTo-Json -Depth 12)
    Write-Log "Reject submit response: $rejectExitCode"
    Write-Log "Reject error: $rejectError"
    if ($rejectError -notmatch 'catching up with network' -or $rejectError -notmatch 'retry after sync') {
        throw "expected 'catching up with network ... retry after sync' error after $probes probes, got: $rejectError (response: $(Join-Path $runRoot 'reject-tx.response.json'))"
    }
    $statsAtReject[$node4.Url] = (Invoke-KanariRpc -Url $node4.Url -Method 'kanari_getStats' -Id 7 -TimeoutSec $RawRpcTimeoutSec).result

    # The peer-authority mapping is policy only, so checkpoint/DAG sync keeps
    # using the original validator committee in this same process.

    $catchupHeight = [int64]($statsBeforeRejoin[$activeUrls[0]].height)
    $allUrls = @($nodes | ForEach-Object { $_.Url })
    $minTxAfterReject = @{}
    foreach ($url in $allUrls) {
        $minTxAfterReject[$url] = [int64]($statsBeforeRejoin[$url].total_transactions)
    }
    $statsSynced = Wait-KanariStatsConverged -Urls $allUrls -MinHeight $catchupHeight -MinTxByUrl $minTxAfterReject -TimeoutSec $SyncTimeoutSec -PassThru -Label 'node4 catch-up convergence'
    Write-Log "After node4 catch-up: $((Format-KanariStats -StatsByUrl $statsSynced) -join ' | ')"

    $syncedTx = @{}
    foreach ($url in $allUrls) {
        $syncedTx[$url] = [int64]($statsSynced[$url].total_transactions)
    }
    $accept = Submit-OneTx -Url $node4.Url -LogPrefix (Join-Path $runRoot 'accept-tx') -TimeoutSec 120
    if ($accept.ExitCode -ne 0) {
        Write-Host $accept.Text
        throw "post-sync submit via node4 failed with exit code $($accept.ExitCode)"
    }

    $finalMinTx = @{}
    foreach ($url in $allUrls) {
        $finalMinTx[$url] = [int64]($syncedTx[$url] + 1)
    }
    $statsFinal = Wait-KanariStatsConverged -Urls $allUrls -MinHeight ($catchupHeight + 1) -MinTxByUrl $finalMinTx -TimeoutSec $SyncTimeoutSec -PassThru -Label 'four-node convergence after accepted tx'
    Write-Log "Final: $((Format-KanariStats -StatsByUrl $statsFinal) -join ' | ')"

    $resultState = 'PASS'
    Write-Log 'SCENARIO PASS: rejoin admission gate rejected the early tx and accepted it after sync'
}
catch {
    $failureDetail = $_ | Out-String
    Write-Log "SCENARIO FAIL: $($_.Exception.Message)"
}
finally {
    if ($proxyJob) {
        try { Stop-Job -Job $proxyJob -ErrorAction SilentlyContinue } catch { }
        try { Remove-Job -Job $proxyJob -Force -ErrorAction SilentlyContinue } catch { }
    }
    Stop-KanariProcesses -Processes @($nodes | ForEach-Object { $_.Process })

    $summary = @(
        "RESULT: $resultState",
        "runlog: $runRoot",
        "env KANARI_SYNC_ADMISSION_LAG=$SyncAdmissionLag",
        "before stop: $((Format-KanariStats -StatsByUrl $statsBeforeStop) -join ' | ')",
        "before rejoin: $((Format-KanariStats -StatsByUrl $statsBeforeRejoin) -join ' | ')",
        "node4 at reject: $((Format-KanariStats -StatsByUrl $statsAtReject) -join ' | ')",
        "reject exit code: $rejectExitCode",
        "reject error: $rejectError",
        "final: $((Format-KanariStats -StatsByUrl $statsFinal) -join ' | ')",
        "scenario log: $scenarioLog"
    )
    if ($failureDetail) {
        $summary += "failure: $($failureDetail.Trim())"
    }
    Set-Content -LiteralPath $summaryPath -Value $summary
    Write-Log "Summary: $summaryPath"
    Write-Log "Run artifacts: $runRoot"
}

if ($resultState -ne 'PASS') {
    exit 1
}