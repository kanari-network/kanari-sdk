# Start a specific Kanari node
param(
    [Parameter(Mandatory=$true)]
    [int]$NodeId = 1,
    
    [string]$BaseDataDir = "$env:USERPROFILE\.kanari\kanari-db",
    [int]$BasePeerPort = 19000,
    [int]$BaseRpcPort = 19001
)

$p2pPort = $BasePeerPort + (($NodeId - 1) * 10)
$rpcPort = $BaseRpcPort + (($NodeId - 1) * 10)
$dataDir = "$BaseDataDir\node$NodeId"

# Create data directory if it doesn't exist
if (-not (Test-Path $dataDir)) {
    New-Item -ItemType Directory -Path $dataDir -Force | Out-Null
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Starting Kanari Node $NodeId" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "P2P Port: $p2pPort" -ForegroundColor Yellow
Write-Host "RPC Port: $rpcPort" -ForegroundColor Yellow
Write-Host "Data Dir: $dataDir" -ForegroundColor Yellow
Write-Host "RPC URL:  http://127.0.0.1:$rpcPort" -ForegroundColor Yellow
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Find kanari-node executable
$exePath = $null

# Check if cargo target exists
$releaseExe = "..\..\target\release\kanari-node.exe"
$debugExe = "..\..\target\debug\kanari-node.exe"

if (Test-Path $releaseExe) {
    $exePath = $releaseExe
    Write-Host "Using release build: $releaseExe" -ForegroundColor Green
} elseif (Test-Path $debugExe) {
    $exePath = $debugExe
    Write-Host "Using debug build: $debugExe" -ForegroundColor Yellow
} elseif (Get-Command kanari-node -ErrorAction SilentlyContinue) {
    $exePath = "kanari-node"
    Write-Host "Using kanari-node from PATH" -ForegroundColor Green
} else {
    Write-Host "Error: kanari-node executable not found!" -ForegroundColor Red
    Write-Host "Please build the project first:" -ForegroundColor Yellow
    Write-Host "  cd ..\.." -ForegroundColor Gray
    Write-Host "  cargo build --release" -ForegroundColor Gray
    Write-Host ""
    Write-Host "Or use debug build:" -ForegroundColor Yellow
    Write-Host "  cargo build" -ForegroundColor Gray
    exit 1
}

Write-Host ""

# Start the node
& $exePath start --p2p-port $p2pPort --rpc-port $rpcPort --data-dir $dataDir
