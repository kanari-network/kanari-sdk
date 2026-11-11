# Builds kanari-move-analyzer (single binary, move-analyzer linked as lib),
# bundles it into the extension, packages a .vsix and installs it.
# kanari-move-analyzer repairs ~/.move caches before starting the LSP.
# Usage (repo root): powershell -File tools\vscode-move-kanari\scripts\package.ps1
$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..\..\..')
$extDir = Join-Path $repoRoot 'tools\vscode-move-kanari'
$analyzerDir = Join-Path $repoRoot 'third_party\move'

Write-Host '==> Building kanari-move-analyzer (release)...'
cargo build --release -p kanari-move-analyzer --manifest-path (Join-Path $repoRoot 'Cargo.toml')
if ($LASTEXITCODE -ne 0) { throw "cargo build failed with exit code $LASTEXITCODE" }

$serverDir = Join-Path $extDir 'language-server'
New-Item -ItemType Directory -Force -Path $serverDir | Out-Null
# Single binary only — drop the old stock move-analyzer.exe if present.
Remove-Item (Join-Path $serverDir 'move-analyzer.exe') -Force -ErrorAction SilentlyContinue
Remove-Item (Join-Path $serverDir 'move-analyzer') -Force -ErrorAction SilentlyContinue

$wrapExe = if ($IsWindows -or $env:OS -eq 'Windows_NT') { 'kanari-move-analyzer.exe' } else { 'kanari-move-analyzer' }
Copy-Item (Join-Path $repoRoot "target\release\$wrapExe") (Join-Path $serverDir $wrapExe) -Force
Write-Host "==> Bundled $moveExe + $wrapExe"

$pluginDir = Join-Path $analyzerDir 'crates\move-analyzer\prettier-plugin'
Write-Host '==> Building prettier-plugin-move...'
Push-Location $pluginDir
try {
    npm install --no-audit --no-fund
    if ($LASTEXITCODE -ne 0) { throw "prettier-plugin npm install failed with exit code $LASTEXITCODE" }
    npm run compile
    if ($LASTEXITCODE -ne 0) { throw "prettier-plugin compile failed with exit code $LASTEXITCODE" }
}
finally {
    Pop-Location
}

$vendorDir = Join-Path $extDir 'vendor\prettier-plugin-move'
if (Test-Path $vendorDir) { Remove-Item $vendorDir -Recurse -Force }
New-Item -ItemType Directory -Force -Path $vendorDir | Out-Null
Copy-Item (Join-Path $pluginDir 'package.json') $vendorDir
Copy-Item (Join-Path $pluginDir 'tree-sitter-move.wasm') $vendorDir
Copy-Item (Join-Path $pluginDir 'out') $vendorDir -Recurse
Write-Host '==> Vendored prettier-plugin-move'

Write-Host '==> npm install + compile + package...'
Push-Location $extDir
try {
    npm install
    if ($LASTEXITCODE -ne 0) { throw "npm install failed with exit code $LASTEXITCODE" }
    npm run package
    if ($LASTEXITCODE -ne 0) { throw "npm run package failed with exit code $LASTEXITCODE" }

    Write-Host '==> Installing move-kanari.vsix...'
    code --install-extension (Join-Path $extDir 'move-kanari.vsix')
    Write-Host '==> Done. Reload VS Code window to activate.'
}
finally {
    Pop-Location
}
