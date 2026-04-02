# Kanari Multi-Node Launcher (Ultimate Edition v3)

Write-Host "Starting Kanari Multi-Node Setup..." -ForegroundColor Green
Write-Host ""

$BaseDataDir = "$env:USERPROFILE\.kanari\node-db"
$BasePeerPort = 19000
$BaseRpcPort = 19001
$NodeCount = 7

# 1. Clear old databases
Write-Host "Clearing old databases to prevent state conflicts..." -ForegroundColor Yellow
if (Test-Path $BaseDataDir) {
    Remove-Item -Recurse -Force "$BaseDataDir\*" -ErrorAction SilentlyContinue
}

# 2. Generate authorities list
$authorities = @()
for ($i = 1; $i -le $NodeCount; $i++) {
    $authorities += "0x$i"
}
$authoritiesStr = $authorities -join ","

# 3. Detect LAN IP (Crucial for reliable P2P)
$localIp = $null
$adapters = Get-NetAdapter -ErrorAction SilentlyContinue | Where-Object { $_.Status -eq 'Up' -and $_.InterfaceDescription -notmatch 'Virtual|vEthernet|Hyper-V|Docker|VMware|Loopback' }
foreach ($a in $adapters) {
    $ip = Get-NetIPAddress -InterfaceIndex $a.ifIndex -AddressFamily IPv4 -ErrorAction SilentlyContinue | Where-Object { $_.IPAddress -notmatch '^127\.' -and $_.IPAddress -notmatch '^169\.254\.' } | Select-Object -First 1
    if ($ip) { $localIp = $ip.IPAddress; break }
}
if (-not $localIp) {
    $localIp = (Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue | Where-Object { $_.IPAddress -notmatch '^127\.' -and $_.IPAddress -notmatch '^169\.254\.' } | Select-Object -First 1).IPAddress
}

Write-Host ""
$startNow = Read-Host "Start all 5 nodes now? (Y/N)"
if ($startNow -match '^[Yy]') {
    $scriptPath = Join-Path $PSScriptRoot 'start-node.ps1'
    $CurrentPS = (Get-Process -Id $PID).Path
    
    for ($i = 1; $i -le $NodeCount; $i++) {
        # Node 1 starts normally. Nodes 2-5 MUST bootstrap to Node 1's P2P address.
        if ($i -eq 1) {
            $argString = "-NoExit -ExecutionPolicy Bypass -File `"$scriptPath`" -NodeId $i -Authorities `"$authoritiesStr`""
        } else {
            # Use the detected LAN IP for the bootstrap address
            $argString = "-NoExit -ExecutionPolicy Bypass -File `"$scriptPath`" -NodeId $i -Authorities `"$authoritiesStr`" -Bootstrap `"/ip4/$localIp/tcp/19000`""
        }

        Start-Process -FilePath $CurrentPS -ArgumentList $argString -WindowStyle Normal
        
        # Give Node 1 ample time to start its P2P listener before launching the others
        if ($i -eq 1) {
            Write-Host "Waiting 5 seconds for Node 1 to initialize P2P listener on $localIp:19000..." -ForegroundColor Cyan
            Start-Sleep -Seconds 5
        } else {
            Start-Sleep -Milliseconds 600
        }
    }
    Write-Host "Launched $NodeCount terminals successfully!" -ForegroundColor Green
    Write-Host "Please wait for the P2P network to stabilize." -ForegroundColor Cyan
} else {
    Write-Host "Aborted." -ForegroundColor Yellow
}