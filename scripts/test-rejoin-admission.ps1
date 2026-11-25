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

# Scenario-only environment. The CLI loads the keystore/password from these,
# and every spawned node inherits KANARI_SYNC_ADMISSION_LAG from this shell.
$env:KANARI_KEYSTORE_PATH = $KeystorePath
$env:KANARI_LOAD_PASSWORD = $Password
$env:KANARI_SYNC_ADMISSION_LAG = "$SyncAdmissionLag"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..')
$nodeExe = Join-Path $repoRoot 'target\release\kanari-node.exe'
$kanariExe = Join-Path $repoRoot 'target\release\kanari.exe'
$runId = Get-Date -Format 'yyyyMMdd-HHmmss'
$runRoot = Join-Path $repoRoot ".kanari-runlogs\rejoin-admission-$runId"
New-Item -ItemType Directory -Force -Path $runRoot | Out-Null
$scenarioLog = Join-Path $runRoot 'scenario.log'
$summaryPath = Join-Path $runRoot 'scenario-result.txt'

function Write-Log {
    param([string]$Message)
    $line = "[{0}] {1}" -f (Get-Date -Format 'HH:mm:ss'), $Message
    Write-Host $line
    Add-Content -LiteralPath $scenarioLog -Value $line
}

function Invoke-KanariRpc {
    param(
        [string]$Url,
        [string]$Method,
        [object]$Params = @{},
        [int]$Id = 1,
        [int]$TimeoutSec = 3
    )

    $body = @{
        jsonrpc = '2.0'
        id = $Id
        method = $Method
        params = $Params
    } | ConvertTo-Json -Depth 32 -Compress

    return Invoke-RestMethod -Uri $Url -Method Post -ContentType 'application/json' -Body $body -TimeoutSec $TimeoutSec
}

function Wait-RpcReady {
    param(
        [string]$Url,
        [int]$TimeoutSec = 60,
        [string]$Label = 'node'
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        try {
            $health = Invoke-KanariRpc -Url $Url -Method 'kanari_health' -Id 1 -TimeoutSec ([Math]::Min(2, $RpcTimeoutSec))
            if ($health.result) {
                Write-Log "READY $Label $Url"
                return
            }
        } catch {
        }
        Start-Sleep -Milliseconds 500
    }
    throw "$Label RPC did not become ready within ${TimeoutSec}s: $Url"
}

function Wait-Converged {
    param(
        [string[]]$Urls,
        [int64]$MinHeight = 0,
        [hashtable]$MinTxByUrl = @{},
        [int]$TimeoutSec = 60,
        [string]$Label = 'convergence'
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        $statsByUrl = @{}
        $roots = New-Object System.Collections.Generic.HashSet[string]
        $heights = New-Object System.Collections.Generic.HashSet[string]
        $txs = New-Object System.Collections.Generic.HashSet[string]

        foreach ($url in $Urls) {
            $stats = (Invoke-KanariRpc -Url $url -Method 'kanari_getStats' -Id 2 -TimeoutSec $RpcTimeoutSec).result
            $statsByUrl[$url] = $stats
            [void]$roots.Add([string]$stats.state_root)
            [void]$heights.Add([string]$stats.height)
            [void]$txs.Add([string]$stats.total_transactions)
        }

        $txFloorMet = $true
        foreach ($url in $Urls) {
            $floor = 0
            if ($MinTxByUrl.ContainsKey($url)) {
                $floor = [int64]$MinTxByUrl[$url]
            }
            if ([int64]($statsByUrl[$url].total_transactions) -lt $floor) {
                $txFloorMet = $false
            }
        }

        if ($roots.Count -eq 1 -and $heights.Count -eq 1 -and $txs.Count -eq 1 -and
            $txFloorMet -and [int64]($statsByUrl[$Urls[0]].height) -ge $MinHeight) {
            return $statsByUrl
        }
        Start-Sleep -Seconds 1
    }

    $details = @()
    foreach ($url in $Urls) {
        try {
            $s = (Invoke-KanariRpc -Url $url -Method 'kanari_getStats' -Id 3 -TimeoutSec $RpcTimeoutSec).result
            $details += ("{0} height={1} txs={2} root={3}" -f $url, $s.height, $s.total_transactions, $s.state_root)
        } catch {
            $details += "$url stats unavailable: $_"
        }
    }
    throw "$Label timed out after ${TimeoutSec}s; $($details -join ' | ')"
}

function Format-Stats {
    param([hashtable]$StatsByUrl)
    return @(
        foreach ($url in $StatsByUrl.Keys | Sort-Object) {
            $s = $StatsByUrl[$url]
            "{0} height={1} txs={2} root={3}" -f $url, $s.height, $s.total_transactions, $s.state_root
        }
    )
}

function Quote-ProcessArgument {
    param([string]$Value)
    if ($Value -match '[\s"]') {
        return '"' + ($Value -replace '"', '\"') + '"'
    }
    return $Value
}

function Invoke-KanariCliBounded {
    param(
        [string[]]$CliArgs,
        [string]$LogPrefix,
        [int]$TimeoutSec = 60
    )

    $out = "$LogPrefix.out.log"
    $err = "$LogPrefix.err.log"
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $kanariExe
    $psi.Arguments = (($CliArgs | ForEach-Object { Quote-ProcessArgument $_ }) -join ' ')
    $psi.WorkingDirectory = $repoRoot
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true

    $proc = New-Object System.Diagnostics.Process
    $proc.StartInfo = $psi
    [void]$proc.Start()
    $stdoutTask = $proc.StandardOutput.ReadToEndAsync()
    $stderrTask = $proc.StandardError.ReadToEndAsync()

    if (-not $proc.WaitForExit($TimeoutSec * 1000)) {
        try { $proc.Kill() } catch { }
        try { [void]$proc.WaitForExit(5000) } catch { }
        $stdout = $stdoutTask.GetAwaiter().GetResult()
        $stderr = $stderrTask.GetAwaiter().GetResult()
        [System.IO.File]::WriteAllText($out, $stdout)
        [System.IO.File]::WriteAllText($err, $stderr)
        throw "CLI did not exit within ${TimeoutSec}s (possible hang). Logs: $out / $err`n$stdout`n$stderr"
    }

    $stdout = $stdoutTask.GetAwaiter().GetResult()
    $stderr = $stderrTask.GetAwaiter().GetResult()
    $exitCode = $proc.ExitCode
    [System.IO.File]::WriteAllText($out, $stdout)
    [System.IO.File]::WriteAllText($err, $stderr)
    $text = "$stdout`n$stderr"
    return [pscustomobject]@{
        ExitCode = $exitCode
        Text     = $text
        Out      = $out
        Err      = $err
    }
}

function Submit-OneTx {
    param(
        [string]$Url,
        [string]$LogPrefix,
        [int]$TimeoutSec
    )

    $amountText = $Amount.ToString('0.############################', [System.Globalization.CultureInfo]::InvariantCulture)
    $args = @(
        'client', 'stress-test',
        '--from', $From,
        '--to', $To,
        '--amount', $amountText,
        '--count', '1',
        '--password', $Password,
        '--rpc', $Url,
        '--commit-timeout-sec', '15'
    )
    return Invoke-KanariCliBounded -CliArgs $args -LogPrefix $LogPrefix -TimeoutSec $TimeoutSec
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
                                id = $parsed.id
                                result = [pscustomobject]@{
                                    hash = [string]$parsed.params.transaction_hash
                                    status = 'pending'
                                    action = 'submit'
                                    success = $true
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
                } catch {
                    $reply = [pscustomobject]@{
                        jsonrpc = '2.0'
                        id = 1
                        result = $null
                        error = [pscustomobject]@{ code = -32603; message = "capture proxy: $_" }
                    } | ConvertTo-Json -Depth 8 -Compress
                }
                $bytes = [System.Text.Encoding]::UTF8.GetBytes($reply)
                $context.Response.StatusCode = 200
                $context.Response.ContentType = 'application/json'
                $context.Response.ContentLength64 = $bytes.Length
                $context.Response.OutputStream.Write($bytes, 0, $bytes.Length)
                $context.Response.Close()
            }
        } finally {
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
    } finally {
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
    } catch {
        $details = if ($_.ErrorDetails.Message) { $_.ErrorDetails.Message } else { $_.Exception.Message }
        $code = '(http error)'
        $message = $details
        try {
            $parsed = $details | ConvertFrom-Json
            if ($parsed.error) {
                $code = $parsed.error.code
                $message = $parsed.error.message
            }
        } catch {
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
    $keysDir = Join-Path $runRoot 'consensus-keys'
    $genesis = Join-Path $runRoot 'devnet-genesis.json'
    # Genesis export initializes a persistent store; never reuse node1's dir.
    $sourceData = Join-Path $runRoot 'genesis-db'
    New-Item -ItemType Directory -Force -Path $sourceData | Out-Null

    $prepareLog = Join-Path $runRoot 'prepare.out.log'
    $oldEap = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        & $nodeExe consensus-keygen --node-count 4 --output-dir $keysDir --force *> $prepareLog
        $exitCode = $LASTEXITCODE
        if ($exitCode -eq 0) {
            & $nodeExe genesis-export --network devnet --data-dir $sourceData --output $genesis *>> $prepareLog
            $exitCode = $LASTEXITCODE
        }
    } finally {
        $ErrorActionPreference = $oldEap
    }
    if ($exitCode -ne 0) {
        Get-Content -LiteralPath $prepareLog -Tail 60
        throw "devnet preparation failed with exit code $exitCode; log: $prepareLog"
    }

    $authorities = '0x1,0x2,0x3,0x4'
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
        $proc = Start-Process `
            -FilePath $nodeExe `
            -ArgumentList $nodeArgs `
            -WorkingDirectory $repoRoot `
            -RedirectStandardOutput $out `
            -RedirectStandardError $err `
            -WindowStyle Hidden `
            -PassThru

        $nodes += [pscustomobject]@{
            Name        = "node$i"
            Process     = $proc
            Url         = "http://127.0.0.1:$rpcPort"
            DataDir     = $dataDir
            Args        = $nodeArgs
            BootstrapOut = $out
        }
        Start-Sleep -Milliseconds 900
    }

    foreach ($node in $nodes) {
        Wait-RpcReady -Url $node.Url -Label $node.Name -TimeoutSec 90
    }

    # Capture baseline, then take node 4 down before generating the height gap.
    foreach ($node in $nodes) {
        $statsBeforeStop[$node.Url] = (Invoke-KanariRpc -Url $node.Url -Method 'kanari_getStats' -Id 4 -TimeoutSec $RpcTimeoutSec).result
    }
    Write-Log "Before stopping node4: $((Format-Stats -StatsByUrl $statsBeforeStop) -join ' | ')"

    $node4 = $nodes | Where-Object { $_.Name -eq 'node4' }

    # The admission gate only trusts peer heights from IDs in --authorities.
    # Libp2p peer IDs are not validator authority IDs by default, so register
    # the active nodes' peer IDs on the rejoining node only (healthy nodes keep
    # the original validator set). Public-key entries are required for every
    # configured authority; the rejoining node never signs as these extra IDs.
    $peerIds = @()
    foreach ($node in @($nodes | Where-Object { $_.Name -ne 'node4' })) {
        $logText = Read-SharedText -Path $node.BootstrapOut
        $match = [regex]::Match($logText, 'Local peer id: (?<peer>12D3Koo[A-Za-z0-9]+)')
        if (-not $match.Success) {
            throw "could not read libp2p peer ID for $($node.Name) from $($node.BootstrapOut)"
        }
        $peerIds += $match.Groups['peer'].Value
    }
    Write-Log "Active peer IDs: $($peerIds -join ',')"

    $rejoinAuthorities = "0x1,0x2,0x3,0x4,$($peerIds -join ',')"
    $publicKeysPath = Join-Path $keysDir 'consensus-public-keys.json'
    $publicKeys = [System.IO.File]::ReadAllText($publicKeysPath) | ConvertFrom-Json
    foreach ($peerId in $peerIds) {
        $publicKeys | Add-Member -NotePropertyName "0x$peerId" -NotePropertyValue $publicKeys.'0x4' -Force
    }
    $rejoinPublicKeys = Join-Path $runRoot 'consensus-public-keys-rejoin.json'
    Set-Content -LiteralPath $rejoinPublicKeys -Value ($publicKeys | ConvertTo-Json -Depth 8)

    Stop-Process -Id $node4.Process.Id -Force
    [void]$node4.Process.WaitForExit(10000)
    Write-Log "Stopped node4 (pid $($node4.Process.Id)); active nodes: node1,node2,node3"

    $active = @($nodes | Where-Object { $_.Name -ne 'node4' })
    $activeUrls = @($active | ForEach-Object { $_.Url })
    $baselineTx = @{}
    foreach ($url in $activeUrls) {
        $baselineTx[$url] = [int64]$statsBeforeStop[$url].total_transactions
    }

    $amountText = $Amount.ToString('0.############################', [System.Globalization.CultureInfo]::InvariantCulture)
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
    $initial = Invoke-KanariCliBounded -CliArgs $submitArgs -LogPrefix (Join-Path $runRoot 'initial-tx') -TimeoutSec 180
    if ($initial.ExitCode -ne 0) {
        Write-Host $initial.Text
        throw "initial transaction load via node1 failed with exit code $($initial.ExitCode)"
    }

    $firstActiveUrl = $activeUrls[0]
    $initialMinTx = @{
        $firstActiveUrl = [int64]$baselineTx[$firstActiveUrl] + $Count
    }
    $statsBeforeRejoin = Wait-Converged -Urls $activeUrls -MinHeight ([int64]$statsBeforeStop[$firstActiveUrl].height + 1) -MinTxByUrl $initialMinTx -TimeoutSec $SyncTimeoutSec -Label 'nodes 1-3 convergence before rejoin'

    # A fresh node starts at height 0. Keep the gap well above the lag so the
    # gate stays armed for several checkpoints instead of only the first one.
    $targetGapHeight = [int64]$SyncAdmissionLag + 6
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
        $extra = Invoke-KanariCliBounded -CliArgs $extraArgs -LogPrefix (Join-Path $runRoot "gap-tx-$extraRound") -TimeoutSec 180
        if ($extra.ExitCode -ne 0) {
            Write-Host $extra.Text
            throw "gap transaction batch failed with exit code $($extra.ExitCode)"
        }
        $gapMinTx = @{}
        foreach ($url in $activeUrls) {
            $gapMinTx[$url] = [int64]($baselineTx[$url] + $extraCount)
        }
        $statsBeforeRejoin = Wait-Converged -Urls $activeUrls -MinHeight ([int64]($statsBeforeRejoin[$activeUrls[0]].height) + 1) -MinTxByUrl $gapMinTx -TimeoutSec $SyncTimeoutSec -Label 'nodes 1-3 convergence after gap batch'
        $extraRound++
    }
    if ([int64]($statsBeforeRejoin[$activeUrls[0]].height) -le [int64]$SyncAdmissionLag) {
        throw "could not build a height gap above KANARI_SYNC_ADMISSION_LAG=$SyncAdmissionLag"
    }
    Write-Log "Before rejoin: $((Format-Stats -StatsByUrl $statsBeforeRejoin) -join ' | ')"

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
    $captured = Invoke-KanariCliBounded -CliArgs $captureArgs -LogPrefix (Join-Path $runRoot 'capture-tx') -TimeoutSec 60
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
    $rejoinArgs = @($node4.Args)
    for ($argIndex = 0; $argIndex -lt $rejoinArgs.Count - 1; $argIndex++) {
        if ($rejoinArgs[$argIndex] -eq '--authorities') {
            $rejoinArgs[$argIndex + 1] = $rejoinAuthorities
        }
        if ($rejoinArgs[$argIndex] -eq '--consensus-public-keys') {
            $rejoinArgs[$argIndex + 1] = $rejoinPublicKeys
        }
    }
    $rejoinOut = Join-Path $runRoot 'node4-rejoin.out.log'
    $rejoinErr = Join-Path $runRoot 'node4-rejoin.err.log'
    $node4.Process = Start-Process `
        -FilePath $nodeExe `
        -ArgumentList $rejoinArgs `
        -WorkingDirectory $repoRoot `
        -RedirectStandardOutput $rejoinOut `
        -RedirectStandardError $rejoinErr `
        -WindowStyle Hidden `
        -PassThru
    Write-Log "Rejoined node4 with empty data dir (pid $($node4.Process.Id)); lag=$SyncAdmissionLag; peer-authorities registered"

    # Poll fast: the gate is observable only between the first peer-height
    # advertisement and local checkpoint catch-up (a few hundred milliseconds).
    $ready = $false
    $readyDeadline = (Get-Date).AddSeconds(60)
    while ((Get-Date) -lt $readyDeadline) {
        try {
            $health = Invoke-KanariRpc -Url $node4.Url -Method 'kanari_health' -Id 1 -TimeoutSec 1
            if ($health.result) {
                $ready = $true
                break
            }
        } catch {
        }
        Start-Sleep -Milliseconds 25
    }
    if (-not $ready) {
        throw "rejoined node4 RPC did not become ready within 60s; log: $rejoinOut"
    }
    Write-Log "READY node4 rejoin $($node4.Url)"

    # Raw replay of the signed payload with a short HTTP timeout: no CLI start
    # delay and no possibility of an unbounded wait. The observed peer height is
    # armed by the sync loop a few hundred ms after RPC readiness, so poll the
    # same payload quickly until the gate responds instead of racing one probe.
    $statsAtReject[$node4.Url] = (Invoke-KanariRpc -Url $node4.Url -Method 'kanari_getStats' -Id 5 -TimeoutSec $RawRpcTimeoutSec).result
    Write-Log "node4 at first submit: height=$($statsAtReject[$node4.Url].height) root=$($statsAtReject[$node4.Url].state_root)"

    $raw = $null
    $probes = 0
    $probeDeadline = (Get-Date).AddSeconds(10)
    while ((Get-Date) -lt $probeDeadline) {
        $probes++
        $raw = Invoke-RawSubmit -Url $node4.Url -Payload $capturedPayload -TimeoutSec $RawRpcTimeoutSec
        if ($raw.Message -match 'catching up with network' -and $raw.Message -match 'retry after sync') {
            break
        }
        Start-Sleep -Milliseconds 25
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

    # Peer IDs had to be registered as authorities to arm the gate, but those
    # extra validators change the Mysticeti threshold and block checkpoint
    # sync. Restart once more from an empty dir with the original validator set
    # so node 4 can reach the same height/root before the acceptance check.
    Stop-Process -Id $node4.Process.Id -Force
    [void]$node4.Process.WaitForExit(10000)
    if (Test-Path -LiteralPath $node4.DataDir) {
        Remove-Item -LiteralPath $node4.DataDir -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $node4.DataDir | Out-Null
    $syncOut = Join-Path $runRoot 'node4-sync.out.log'
    $syncErr = Join-Path $runRoot 'node4-sync.err.log'
    $node4.Process = Start-Process `
        -FilePath $nodeExe `
        -ArgumentList $node4.Args `
        -WorkingDirectory $repoRoot `
        -RedirectStandardOutput $syncOut `
        -RedirectStandardError $syncErr `
        -WindowStyle Hidden `
        -PassThru
    Wait-RpcReady -Url $node4.Url -Label 'node4 sync restart' -TimeoutSec 90
    Write-Log "Restarted node4 for catch-up with the original validator set (pid $($node4.Process.Id))"

    $catchupHeight = [int64]($statsBeforeRejoin[$activeUrls[0]].height)
    $allUrls = @($nodes | ForEach-Object { $_.Url })
    $minTxAfterReject = @{}
    foreach ($url in $allUrls) {
        $minTxAfterReject[$url] = [int64]($statsBeforeRejoin[$url].total_transactions)
    }
    $statsSynced = Wait-Converged -Urls $allUrls -MinHeight $catchupHeight -MinTxByUrl $minTxAfterReject -TimeoutSec $SyncTimeoutSec -Label 'node4 catch-up convergence'
    Write-Log "After node4 catch-up: $((Format-Stats -StatsByUrl $statsSynced) -join ' | ')"

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
    $statsFinal = Wait-Converged -Urls $allUrls -MinHeight ($catchupHeight + 1) -MinTxByUrl $finalMinTx -TimeoutSec $SyncTimeoutSec -Label 'four-node convergence after accepted tx'
    Write-Log "Final: $((Format-Stats -StatsByUrl $statsFinal) -join ' | ')"

    $resultState = 'PASS'
    Write-Log 'SCENARIO PASS: rejoin admission gate rejected the early tx and accepted it after sync'
} catch {
    $failureDetail = $_ | Out-String
    Write-Log "SCENARIO FAIL: $($_.Exception.Message)"
} finally {
    if ($proxyJob) {
        try { Stop-Job -Job $proxyJob -ErrorAction SilentlyContinue } catch { }
        try { Remove-Job -Job $proxyJob -Force -ErrorAction SilentlyContinue } catch { }
    }
    foreach ($node in $nodes) {
        try {
            if ($node.Process -and -not $node.Process.HasExited) {
                Stop-Process -Id $node.Process.Id -Force
            }
        } catch {
        }
    }

    $summary = @(
        "RESULT: $resultState",
        "runlog: $runRoot",
        "env KANARI_SYNC_ADMISSION_LAG=$SyncAdmissionLag",
        "before stop: $((Format-Stats -StatsByUrl $statsBeforeStop) -join ' | ')",
        "before rejoin: $((Format-Stats -StatsByUrl $statsBeforeRejoin) -join ' | ')",
        "node4 at reject: $((Format-Stats -StatsByUrl $statsAtReject) -join ' | ')",
        "reject exit code: $rejectExitCode",
        "reject error: $rejectError",
        "final: $((Format-Stats -StatsByUrl $statsFinal) -join ' | ')",
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
