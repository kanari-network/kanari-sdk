param(
    [ValidateRange(1, 168)]
    [int]$Hours = 8,
    [ValidateRange(1, 256)]
    [int]$Workers = 1
)

$ErrorActionPreference = 'Stop'
$deadline = (Get-Date).AddHours($Hours)
$iteration = 0

Write-Host "Kanari adversarial soak: runs until $deadline with $Workers worker(s)."
Write-Host "Covers bounded P2P decompression/DoS and Byzantine Mysticeti native blocks."

while ((Get-Date) -lt $deadline) {
    $iteration += 1
    Write-Host "[$(Get-Date -Format o)] iteration $iteration"

    cargo test -p kanari-node long_run_malformed_compressed_payloads_are_bounded -- --ignored --test-threads=$Workers
    cargo test -p kanari-core long_run_byzantine_native_blocks_cannot_advance_checkpoint -- --ignored --test-threads=$Workers
}

Write-Host "Adversarial soak completed: $iteration iteration(s) without a test failure."
