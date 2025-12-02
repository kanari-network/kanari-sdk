Kanari-crypto Move integration and example

This document shows how to compile Move modules with the local `movec` CLI and run the Rust example that creates and saves a wallet using `kanari-crypto`.

Prerequisites
- Rust toolchain (cargo)
- `movec` CLI installed locally and available on PATH (the user mentioned movec is installed)

1) Compile Move modules with movec (example)

Open PowerShell in the repo root and run (adjust paths as needed):

```powershell
# Build a Move package located at move/coin
movec build --package-dir .\move\coin
```

This will produce compiled Move bytecode (usually under the package's build dir). The Rust host can then load and publish those compiled modules into the MoveVM.

2) Run the Rust example that creates and saves a wallet

Change to the `crates/kanari-crypto` directory and run the example with cargo:

```powershell
cd .\crates\kanari-crypto
# Run the example (it uses the crate's APIs to generate and save a wallet)
cargo run --example move_coin_wallet
```

Notes
- The example demonstrates wallet creation and saving to the keystore using `kanari-crypto` APIs. It does not automatically publish Move modules — that's shown as a manual movec step because compiling Move bytecode requires the local Move toolchain.
- If you want me to add a Move module source and wiring code to have the Rust host load the compiled .mv files automatically, I can add a small `kanari-move-host` crate that looks up the compiled artifacts and publishes them into the MoveVM.
