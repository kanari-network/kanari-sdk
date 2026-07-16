# Adversarial soak testing

The repository includes opt-in, long-running adversarial tests. They complement normal unit/property tests; they are not a substitute for an independent audit or a production network exercise.

Run one soak iteration:

```powershell
cargo test -p kanari-node long_run_malformed_compressed_payloads_are_bounded -- --ignored
cargo test -p kanari-core long_run_byzantine_native_blocks_cannot_advance_checkpoint -- --ignored
```

Run continuously (default 8 hours):

```powershell
.\scripts\run-adversarial-soak.ps1
```

The runner checks that:

- arbitrary and compressed-bomb P2P input stays within the decompression budget and never panics;
- arbitrary malformed Byzantine Mysticeti native blocks cannot advance checkpoint height.

For a network-level exercise, run the four-node devnet separately and inject process restarts, delayed peers, duplicate messages, and malformed network traffic from an isolated test host. Do not conduct these tests against shared or production validators.
