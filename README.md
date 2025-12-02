# Kanari (kanari-cp)

Kanari is a Rust workspace that demonstrates integrating the Move VM with a Rust runtime and CLI. The repository includes Rust crates for types, crypto utilities, a Move runtime bridge, and Move packages that define the on-chain logic.

This README gives a concise developer quick-start, project layout, and common commands for working with the codebase locally (Windows PowerShell examples).

## Project layout (high level)

- `crates/kanari` — CLI binary (`kanari`) and bootstrap logic.
- `crates/kanari-types` — shared domain types (accounts, balances, `TransferRecord`).
- `crates/kanari-move-runtime` — Move VM integration (calling Move functions, validating transfers, persisting state).
- `crates/kanari-crypto` — key management, signing, and crypto utilities.
- `crates/kanari-frameworks/packages/kanari-system` — Move packages (Move modules used by this system).
- `third_party/move` — bundled Move toolchain and crates used as path dependencies in some places.

## Local state

- By default the runtime persists state into RocksDB at `~/.kari/kanari-db/move_vm_db`.
- The runtime stores the serialized `MoveVMState` under the key `"state"`.

## Prerequisites

- Rust and Cargo (stable channel recommended).
- (Optional) If you plan to use the Move toolchain independently, install it; the repository contains a local copy used as a path dependency in some crates.

## Build and run (PowerShell)

```powershell
# Build the kanari CLI
cargo build -p kanari

# Run the CLI and list wallets (this will perform a Rust-side genesis on first run)
cargo run -p kanari -- list-wallets
```

Notes:

- On first run (or if the DB directory is removed), the CLI performs a bootstrap/genesis that mints the initial supply to the developer address embedded in the code.
- To reset state, remove `~/.kari/kanari-db/move_vm_db` and rerun the command above.

## Examples — using the CLI

```powershell
# Show wallets
cargo run -p kanari -- list-wallets

# Forward Move subcommands to the in-repo Move CLI (examples)
cargo run -p kanari -- move new <package-name>
cargo run -p kanari -- move test <path-to-move-package>
```

If you see errors when forwarding Move commands, ensure the workspace builds and that the `third_party/move` path dependencies are present.

## Testing

```powershell
# Run the tests for the shared types crate
cargo test -p kanari-types

# Run all workspace tests (may take longer)
cargo test
```

## Key files to inspect when developing

- `crates/kanari/src/main.rs` — CLI wiring and bootstrap logic.
- `crates/kanari-move-runtime/src/move_runtime.rs` — Move VM integration helpers.
- `crates/kanari-types/src/transfer.rs` — `TransferRecord` shape and validation.
- `crates/kanari-frameworks/packages/kanari-system` — Move modules and package layout.
- `DOCS/SYSTEM_ER.md` — ER diagram and mapping between runtime entities and code (additional documentation).

## Development notes and suggested next tasks

- Decide whether genesis should be executed inside the Move VM (preferred for Move-governed logic) or remain a Rust-side bootstrap. Currently the project seeds genesis from Rust when the DB is empty.
- Add a `kanari status` command that prints total supply and non-zero balances in JSON for easier verification.
- Clean up markdown lint warnings in `DOCS/SYSTEM_ER.md` for clearer docs.

## Need help or want changes?

Tell me if you want the README expanded (CI steps, Docker, contributor guide), or if you want me to implement one of the suggested next tasks. I can also convert this README into Thai or another language if preferred.
