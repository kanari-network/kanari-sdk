# Shared helpers for the four-node local runners.
#
# Dot-source this file from scripts/run-local-four-node-*.ps1.
# Helpers take explicit parameters and must have no top-level side effects other
# than defining functions; caller scripts own all state and error policy.

function Set-KanariFourNodeEnv {
    [System.Diagnostics.CodeAnalysis.SuppressMessageAttribute('PSAvoidUsingPlainTextForPassword', 'Password', Justification = 'Password is required by the kanari CLI --password argument; kept as a string only at the process boundary.')]
    param(
        [AllowEmptyString()][string]$KeystorePath = '',
        [AllowEmptyString()][string]$Password = ''
    )

    if ([string]::IsNullOrWhiteSpace($KeystorePath)) {
        $KeystorePath = 'C:\Users\TaroPukpui\.kanari\kanari_config\kanari.keystore'
    }
    if ([string]::IsNullOrWhiteSpace($Password)) {
        $Password = '@Password12345678'
    }

    $env:KANARI_KEYSTORE_PATH = $KeystorePath
    $env:KANARI_LOAD_PASSWORD = $Password

    [pscustomobject]@{
        KeystorePath = $KeystorePath
        Password     = $Password
    }
}

function Resolve-KanariFourNodeRoot {
    param([Parameter(Mandatory = $true)][string]$ScriptDirectory)

    return [string](Resolve-Path -LiteralPath (Join-Path $ScriptDirectory '..'))
}

function Get-KanariFourNodeExePaths {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [ValidateSet('release', 'debug')][string]$BuildProfile = 'release'
    )

    $profileDir = if ($BuildProfile -eq 'debug') { 'debug' } else { 'release' }
    return [pscustomobject]@{
        NodeExe   = Join-Path $RepoRoot "target\$profileDir\kanari-node.exe"
        KanariExe = Join-Path $RepoRoot "target\$profileDir\kanari.exe"
    }
}

function New-KanariFourNodeRunRoot {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$Prefix
    )

    $runId = Get-Date -Format 'yyyyMMdd-HHmmss'
    $runRoot = Join-Path $RepoRoot ".kanari-runlogs\$Prefix-$runId"
    New-Item -ItemType Directory -Force -Path $runRoot | Out-Null

    return [pscustomobject]@{
        RunId   = $runId
        RunRoot = $runRoot
    }
}

function Get-KanariFourNodeUrls {
    param([Parameter(Mandatory = $true)][int]$BaseRpcPort)

    return @(0..3 | ForEach-Object { "http://127.0.0.1:$($BaseRpcPort + ($_ * 10))" })
}

function Format-KanariAmount {
    param([Parameter(Mandatory = $true)][double]$Amount)

    return $Amount.ToString('0.############################', [System.Globalization.CultureInfo]::InvariantCulture)
}

function Format-KanariProcessArgument {
    param([AllowNull()][string]$Value)

    if ([string]::IsNullOrEmpty($Value)) { return '' }
    if ($Value -match '[\s"]') {
        return '"' + ($Value -replace '"', '\"') + '"'
    }
    return $Value
}

# One hashtable parameter avoids PowerShell 5.1 parameter-set ambiguity when a
# dot-sourced helper is called with many mandatory parameters.
function Start-KanariLoggedProcess {
    param([hashtable]$Launch)

    $arguments = @{} + $Launch
    $arguments.PassThru = $true
    if (-not $arguments.ContainsKey('WindowStyle')) { $arguments.WindowStyle = 'Hidden' }
    return Start-Process @arguments
}

function Invoke-KanariCliBounded {
    param(
        [hashtable]$Launch,
        [int]$TimeoutSec = 60
    )

    $outPath = [string]$Launch.OutLog
    $errPath = [string]$Launch.ErrLog
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = [string]$Launch.FilePath
    $psi.Arguments = (@($Launch.ArgumentList) | ForEach-Object { Format-KanariProcessArgument $_ }) -join ' '
    $psi.WorkingDirectory = [string]$Launch.WorkingDirectory
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.CreateNoWindow = $true

    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $psi
    [void]$process.Start()
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    if (-not $process.WaitForExit($TimeoutSec * 1000)) {
        try { $process.Kill() } catch { }
        try { [void]$process.WaitForExit(5000) } catch { }
    }
    $stdout = $stdoutTask.GetAwaiter().GetResult()
    $stderr = $stderrTask.GetAwaiter().GetResult()
    [System.IO.File]::WriteAllText($outPath, $stdout)
    [System.IO.File]::WriteAllText($errPath, $stderr)
    $timedOut = -not $process.HasExited
    if ($timedOut) {
        throw "CLI did not exit within ${TimeoutSec}s (possible hang). Logs: $outPath / $errPath`n$stdout`n$stderr"
    }
    return [pscustomobject]@{
        ExitCode = $process.ExitCode
        Text     = "$stdout`n$stderr"
        Out      = $outPath
        Err      = $errPath
    }
}

function Invoke-KanariFourNodeBuild {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [ValidateSet('release', 'debug')][string]$BuildProfile = 'release',
        [string[]]$Packages = @('kanari-node', 'kanari'),
        [string]$LogDirectory = ''
    )

    if ([string]::IsNullOrWhiteSpace($LogDirectory)) {
        $LogDirectory = Join-Path ([System.IO.Path]::GetTempPath()) "kanari-build-$([guid]::NewGuid().ToString('N'))"
    }
    New-Item -ItemType Directory -Force -Path $LogDirectory | Out-Null

    foreach ($package in $Packages) {
        $buildArgs = if ($BuildProfile -eq 'debug') {
            @('build', '-p', $package)
        }
        else {
            @('build', '-p', $package, '--release')
        }
        $buildOut = Join-Path $LogDirectory "$package.build.out.log"
        $buildErr = Join-Path $LogDirectory "$package.build.err.log"
        $buildProcess = Start-KanariLoggedProcess -Launch @{
            FilePath               = 'cargo'
            ArgumentList           = $buildArgs
            WorkingDirectory       = $RepoRoot
            RedirectStandardOutput = $buildOut
            RedirectStandardError  = $buildErr
            Wait                   = $true
        }
        $buildExit = [int]$buildProcess.ExitCode
        if ($buildExit -ne 0) {
            Get-Content -LiteralPath $buildOut -Tail 60
            Get-Content -LiteralPath $buildErr -Tail 60
            throw "cargo build -p $package failed with exit code $buildExit; logs: $LogDirectory"
        }
    }
}

function New-KanariJsonRpcBody {
    param(
        [Parameter(Mandatory = $true)][string]$Method,
        [object]$Params = @{},
        [int]$Id = 1
    )

    return @{
        jsonrpc = '2.0'
        id      = $Id
        method  = $Method
        params  = $Params
    } | ConvertTo-Json -Depth 32 -Compress
}

function Get-KanariNearestRankPercentile {
    param([int[]]$Values, [double]$Percentile)

    if ($Values.Count -eq 0) { return 0 }
    $index = [int][Math]::Ceiling(($Percentile / 100.0) * $Values.Count) - 1
    if ($index -lt 0) { $index = 0 }
    if ($index -ge $Values.Count) { $index = $Values.Count - 1 }
    return $Values[$index]
}

function Invoke-KanariRpc {
    param(
        [Parameter(Mandatory = $true)][string]$Url,
        [Parameter(Mandatory = $true)][string]$Method,
        [object]$Params = @{},
        [int]$Id = 1,
        [int]$TimeoutSec = 5
    )

    $body = New-KanariJsonRpcBody -Method $Method -Params $Params -Id $Id
    return Invoke-RestMethod -Uri $Url -Method Post -ContentType 'application/json' -Body $body -TimeoutSec $TimeoutSec
}

function New-KanariFourNodeGenesis {
    param(
        [Parameter(Mandatory = $true)][string]$NodeExe,
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$RunRoot,
        [Parameter(Mandatory = $true)][string]$PrepareLog,
        [string]$PrepareErrLog = ''
    )

    $keysDir = Join-Path $RunRoot 'consensus-keys'
    $genesis = Join-Path $RunRoot 'devnet-genesis.json'
    # Genesis export initializes a store. Keep it out of node1's data dir so
    # node1 does not start with state but no blockchain metadata.
    $sourceData = Join-Path $RunRoot 'genesis-db'
    New-Item -ItemType Directory -Force -Path $sourceData | Out-Null

    # Split-Path -LiteralPath cannot be combined with -Parent in Windows
    # PowerShell 5.1; use the literal path API directly.
    $prepareDir = [System.IO.Path]::GetDirectoryName($PrepareLog)
    if ([string]::IsNullOrWhiteSpace($prepareDir)) {
        $prepareDir = $RunRoot
    }
    if ([string]::IsNullOrWhiteSpace($PrepareErrLog)) {
        $PrepareErrLog = Join-Path $prepareDir 'prepare.err.log'
    }
    $genesisOut = Join-Path $prepareDir 'prepare-genesis.out.log'
    $genesisErr = Join-Path $prepareDir 'prepare-genesis.err.log'

    $keygenArgs = @('consensus-keygen', '--node-count', '4', '--output-dir', $keysDir, '--force')
    $genesisArgs = @('genesis-export', '--network', 'devnet', '--data-dir', $sourceData, '--output', $genesis)

    $keygenProcess = Start-KanariLoggedProcess -Launch @{
        FilePath               = $NodeExe
        ArgumentList           = $keygenArgs
        WorkingDirectory       = $RepoRoot
        RedirectStandardOutput = $PrepareLog
        RedirectStandardError  = $PrepareErrLog
        Wait                   = $true
    }
    $keygenExit = [int]$keygenProcess.ExitCode
    if ($keygenExit -ne 0) {
        Get-Content -LiteralPath $PrepareLog -Tail 80
        Get-Content -LiteralPath $PrepareErrLog -Tail 80
        throw "consensus-keygen failed with exit code $keygenExit; logs: $PrepareLog $PrepareErrLog"
    }

    $exportProcess = Start-KanariLoggedProcess -Launch @{
        FilePath               = $NodeExe
        ArgumentList           = $genesisArgs
        WorkingDirectory       = $RepoRoot
        RedirectStandardOutput = $genesisOut
        RedirectStandardError  = $genesisErr
        Wait                   = $true
    }
    $exportExit = [int]$exportProcess.ExitCode
    # Preserve the historical combined prepare log without re-emitting native
    # stderr through PowerShell's error stream.
    foreach ($pair in @(@($genesisOut, $PrepareLog), @($genesisErr, $PrepareErrLog))) {
        if (Test-Path -LiteralPath $pair[0]) {
            Add-Content -LiteralPath $pair[1] -Value (Get-Content -LiteralPath $pair[0])
        }
    }
    if ($exportExit -ne 0) {
        Get-Content -LiteralPath $genesisOut -Tail 80
        Get-Content -LiteralPath $genesisErr -Tail 80
        throw "genesis-export failed with exit code $exportExit; logs: $genesisOut $genesisErr"
    }

    return [pscustomobject]@{
        KeysDir    = $keysDir
        Genesis    = $genesis
        SourceData = $sourceData
    }
}

function New-KanariPeerAuthorityMap {
    param(
        [Parameter(Mandatory = $true)][string]$NodeExe,
        [Parameter(Mandatory = $true)][string]$RunRoot
    )

    $entries = @()
    for ($i = 1; $i -le 4; $i++) {
        $peerDataDir = Join-Path $RunRoot "node$i-db"
        New-Item -ItemType Directory -Force -Path $peerDataDir | Out-Null
        $peerOutPath = Join-Path $RunRoot "node$i-peer-id.out"
        $peerErrPath = Join-Path $RunRoot "node$i-peer-id.err.log"
        $peerProcess = Start-KanariLoggedProcess -Launch @{
            FilePath               = $NodeExe
            ArgumentList           = @('p2p-peer-id', '--network', 'devnet', '--data-dir', $peerDataDir)
            WorkingDirectory       = $RunRoot
            RedirectStandardOutput = $peerOutPath
            RedirectStandardError  = $peerErrPath
            Wait                   = $true
        }
        $peerExit = [int]$peerProcess.ExitCode
        if ($peerExit -ne 0) {
            Get-Content -LiteralPath $peerOutPath -Tail 40
            Get-Content -LiteralPath $peerErrPath -Tail 40
            throw "p2p-peer-id failed for node$i with exit code $peerExit; logs: $peerOutPath $peerErrPath"
        }

        # The peer ID may be printed on either stream depending on build flags.
        $peerText = @()
        foreach ($peerLog in @($peerOutPath, $peerErrPath)) {
            if (Test-Path -LiteralPath $peerLog) {
                $peerText += Get-Content -LiteralPath $peerLog
            }
        }
        $peerText = $peerText -join [Environment]::NewLine
        $peerMatch = [regex]::Match($peerText, '12D3Koo[A-Za-z0-9]+')
        if (-not $peerMatch.Success) {
            throw "could not parse libp2p peer ID for node$i from: $peerText"
        }
        $entries += "$($peerMatch.Value)=0x$i"
    }

    $mapping = $entries -join ','
    Write-Host "Trusted peer-authority mappings: $mapping"
    return $mapping
}

function Start-KanariValidator {
    param(
        [Parameter(Mandatory = $true)][string]$NodeExe,
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)][string]$RunRoot,
        [Parameter(Mandatory = $true)][int]$Index,
        [Parameter(Mandatory = $true)][int]$BaseP2pPort,
        [Parameter(Mandatory = $true)][string]$Genesis,
        [Parameter(Mandatory = $true)][string]$KeysDir,
        [Parameter(Mandatory = $true)][string]$PeerAuthorityArg,
        [int]$RpcPort,
        [string]$LogSuffix = '',
        [hashtable]$Environment = @{},
        [switch]$PassThruDetails
    )

    $p2pPort = $BaseP2pPort + (($Index - 1) * 10)
    if ($RpcPort -le 0) {
        $RpcPort = 0
    }
    $dataDir = Join-Path $RunRoot "node$Index-db"
    New-Item -ItemType Directory -Force -Path $dataDir | Out-Null

    $nodeArgs = @(
        'start',
        '--network', 'devnet',
        '--p2p-port', $p2pPort,
        '--rpc-port', $RpcPort,
        '--rpc-host', '127.0.0.1',
        '--data-dir', $dataDir,
        '--authority-id', "0x$Index",
        '--authorities', '0x1,0x2,0x3,0x4',
        '--genesis', $Genesis,
        '--consensus-private-key-file', (Join-Path $KeysDir "node$Index-consensus-private-key.key"),
        '--consensus-public-keys', (Join-Path $KeysDir 'consensus-public-keys.json'),
        '--peer-authority', $PeerAuthorityArg
    )
    if ($Index -ne 1) {
        $nodeArgs += @('--bootstrap', "/ip4/127.0.0.1/tcp/$BaseP2pPort")
    }

    $suffix = if ([string]::IsNullOrWhiteSpace($LogSuffix)) { '' } else { ".$LogSuffix" }
    $outPath = Join-Path $RunRoot "node$Index$suffix.out.log"
    $errPath = Join-Path $RunRoot "node$Index$suffix.err.log"

    $previousValues = @{}
    foreach ($key in $Environment.Keys) {
        $previousValues[$key] = Get-Item -Path "Env:$key" -ErrorAction SilentlyContinue
        $value = $Environment[$key]
        if ($null -eq $value) {
            Remove-Item -Path "Env:$key" -ErrorAction SilentlyContinue
        }
        else {
            Set-Item -Path "Env:$key" -Value ([string]$value)
        }
    }

    try {
        $nodeProcess = Start-KanariLoggedProcess -Launch @{
            FilePath               = $NodeExe
            ArgumentList           = $nodeArgs
            WorkingDirectory       = $RepoRoot
            RedirectStandardOutput = $outPath
            RedirectStandardError  = $errPath
        }
    }
    finally {
        foreach ($key in $previousValues.Keys) {
            $previous = $previousValues[$key]
            if ($null -eq $previous) {
                Remove-Item -Path "Env:$key" -ErrorAction SilentlyContinue
            }
            else {
                Set-Item -Path "Env:$key" -Value $previous.Value
            }
        }
    }

    if ($PassThruDetails) {
        return [pscustomobject]@{
            Name    = "node$Index"
            Process = $nodeProcess
            Url     = "http://127.0.0.1:$RpcPort"
            DataDir = $dataDir
            Args    = $nodeArgs
            Out     = $outPath
            Err     = $errPath
        }
    }
    return $nodeProcess
}

function Wait-KanariRpcReady {
    param(
        [Parameter(Mandatory = $true)][string[]]$Urls,
        [int]$Attempts = 120,
        [array]$Processes = @(),
        [string]$RunRoot = ''
    )

    $ready = @{}
    for ($attempt = 1; $attempt -le $Attempts; $attempt++) {
        foreach ($url in $Urls) {
            if ($ready[$url]) { continue }
            try {
                $health = Invoke-KanariRpc -Url $url -Method 'kanari_health' -Id 1 -TimeoutSec 2
                if ($health.result) {
                    $ready[$url] = $true
                    Write-Host "READY $url"
                }
            }
            catch {
            }
        }

        if ($ready.Count -eq $Urls.Count) { return }

        foreach ($process in $Processes) {
            if ($process -and $process.HasExited) {
                throw "node process exited early pid=$($process.Id) code=$($process.ExitCode); logs: $RunRoot"
            }
        }

        Start-Sleep -Seconds 1
    }

    throw "only $($ready.Count)/$($Urls.Count) RPC endpoints became ready; logs: $RunRoot"
}

function Wait-KanariStatsConverged {
    param(
        [Parameter(Mandatory = $true)][string[]]$Urls,
        [int]$TimeoutSec = 120,
        [string]$RunRoot = '',
        [switch]$Quiet,
        [hashtable]$BeforeStats = @{},
        [int]$MinTxDelta = 0,
        [int]$MinHeightDelta = 0,
        [int64]$MinHeight = 0,
        [hashtable]$MinTxByUrl = @{},
        [string]$Label = 'convergence',
        [switch]$PassThru
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        $roots = New-Object System.Collections.Generic.HashSet[string]
        $heights = New-Object System.Collections.Generic.HashSet[string]
        $txs = New-Object System.Collections.Generic.HashSet[string]
        $latest = @{}
        $readable = 0

        foreach ($url in $Urls) {
            try {
                $stats = (Invoke-KanariRpc -Url $url -Method 'kanari_getStats' -Id 2 -TimeoutSec 3).result
            }
            catch {
                if (-not $Quiet) {
                    Write-Host "WAIT $url transient RPC read failed: $($_.Exception.Message)"
                }
                continue
            }
            if (-not $stats) {
                if (-not $Quiet) { Write-Host "WAIT $url returned empty stats" }
                continue
            }
            $readable += 1
            $latest[$url] = $stats
            [void]$roots.Add([string]$stats.state_root)
            [void]$heights.Add([string]$stats.height)
            [void]$txs.Add([string]$stats.total_transactions)
        }

        if ($readable -eq $Urls.Count -and $roots.Count -eq 1 -and $heights.Count -eq 1 -and $txs.Count -eq 1) {
            $accepted = $true
            if ($MinTxDelta -gt 0 -or $MinHeightDelta -gt 0) {
                $current = $latest[$Urls[0]]
                $before = $BeforeStats[$Urls[0]]
                if ($null -eq $current -or $null -eq $before) {
                    $accepted = $false
                }
                else {
                    $txDelta = [int64]$current.total_transactions - [int64]$before.total_transactions
                    $heightDelta = [int64]$current.height - [int64]$before.height
                    if ($txDelta -lt $MinTxDelta -or $heightDelta -lt $MinHeightDelta) {
                        $accepted = $false
                    }
                }
            }
            if ($accepted -and [int64]$latest[$Urls[0]].height -ge $MinHeight) {
                foreach ($url in $Urls) {
                    $floor = if ($MinTxByUrl.ContainsKey($url)) { [int64]$MinTxByUrl[$url] } else { 0 }
                    if ([int64]$latest[$url].total_transactions -lt $floor) {
                        $accepted = $false
                        break
                    }
                }
            }
            else {
                $accepted = $false
            }
            if ($accepted) {
                if (-not $Quiet) {
                    foreach ($url in $Urls) {
                        $stats = $latest[$url]
                        Write-Host "SYNCED $url height=$($stats.height) txs=$($stats.total_transactions) root=$($stats.state_root)"
                    }
                }
                if ($PassThru) { return $latest }
                return
            }
        }

        Start-Sleep -Seconds 1
    }

    throw "$Label timed out after ${TimeoutSec}s; logs: $RunRoot"
}

function Get-KanariStatsByUrl {
    param(
        [Parameter(Mandatory = $true)][string[]]$Urls,
        [int]$Id = 2,
        [int]$TimeoutSec = 3
    )

    $statsByUrl = @{}
    foreach ($url in $Urls) {
        $statsByUrl[$url] = (Invoke-KanariRpc -Url $url -Method 'kanari_getStats' -Id $Id -TimeoutSec $TimeoutSec).result
    }
    return $statsByUrl
}

function Format-KanariStats {
    param([hashtable]$StatsByUrl)

    return @(
        foreach ($url in $StatsByUrl.Keys | Sort-Object) {
            $stats = $StatsByUrl[$url]
            "{0} height={1} txs={2} root={3}" -f $url, $stats.height, $stats.total_transactions, $stats.state_root
        }
    )
}

function Show-KanariStats {
    param([Parameter(Mandatory = $true)][string[]]$Urls)

    foreach ($url in $Urls) {
        try {
            $stats = (Invoke-KanariRpc -Url $url -Method 'kanari_getStats' -Id 3 -TimeoutSec 3).result
            Write-Host "STATS $url height=$($stats.height) txs=$($stats.total_transactions) pending=$($stats.pending_transactions) root=$($stats.state_root)"
        }
        catch {
            Write-Host "STATS $url OFFLINE $($_.Exception.Message)"
        }
    }
}

function Stop-KanariStaleFourNodeProcesses {
    param(
        [Parameter(Mandatory = $true)][int]$BaseP2pPort,
        [Parameter(Mandatory = $true)][int]$BaseRpcPort
    )

    $ports = @()
    foreach ($offset in 0, 10, 20, 30) {
        $ports += ($BaseP2pPort + $offset)
        $ports += ($BaseRpcPort + $offset)
    }
    $portPattern = (($ports | ForEach-Object { [regex]::Escape([string]$_) }) -join '|')

    $stale = @()
    $processes = Get-CimInstance Win32_Process -Filter "Name='kanari-node.exe'" -ErrorAction SilentlyContinue
    foreach ($process in @($processes)) {
        $commandLine = [string]$process.CommandLine
        if ($commandLine -match "(?:--p2p-port|--rpc-port)\s+($portPattern)(?:\s|$)") {
            $stale += $process
        }
    }

    foreach ($process in $stale) {
        Write-Host "Stopping stale kanari-node process $($process.ProcessId) from a previous four-node run"
        try {
            Stop-Process -Id $process.ProcessId -Force -ErrorAction Stop
        }
        catch {
            # A process can exit between enumeration and Stop-Process; that is
            # already the desired state. Any other failure is caught by the
            # listener verification below.
        }
    }

    Start-Sleep -Milliseconds 200
    $listeners = @(
        [System.Net.NetworkInformation.IPGlobalProperties]::GetIPGlobalProperties().GetActiveTcpListeners() |
        Where-Object { $ports -contains $_.Port }
    )
    if ($listeners.Count -gt 0) {
        $occupied = ($listeners | ForEach-Object { $_.Port } | Sort-Object -Unique) -join ','
        throw "Four-node ports are still in use after stale-process cleanup: $occupied. Stop the owning processes before running this script."
    }
}

function Stop-KanariProcesses {
    # Keep this untyped: an [array] parameter converts an hashtable of
    # node-index -> Process into a single Object[] element, which would silently
    # skip cleanup for the chaos runners.
    param([object]$Processes = @())

    if ($Processes -is [System.Collections.IDictionary]) {
        $targets = @($Processes.Values)
    }
    else {
        $targets = @($Processes)
    }

    foreach ($process in $targets) {
        try {
            if ($process -and -not $process.HasExited) {
                Stop-Process -Id $process.Id -Force
            }
        }
        catch {
        }
    }
}
