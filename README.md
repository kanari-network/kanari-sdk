# Kanari

Kanari is a real-time transaction network designed for in-game payments and asset systems.

It allows developers to build instant game economy features — such as UID top-ups, item purchases, and asset transfers — with sub-second finality and no gas fees.

---

## What can you build with Kanari?

- 🎮 In-game payment system (UID top-up)
- 💰 Instant item purchases
- 🔁 Real-time asset transfers
- 📊 Game economy backend

---

## Why Kanari?

Traditional systems:

- ❌ Slow settlement (seconds or batch processing)
- ❌ Complex backend logic
- ❌ No verifiable state

Kanari:

- ⚡ Instant execution (~10 ms)
- 🔒 Secure finality (~300 ms)
- 💸 No gas fees (user-friendly UX)
- 🧩 Simple integration

---

## How it works

1. Transaction is submitted
2. Executed instantly by a small node set (~10 ms)
3. Propagated across the network (DAG)
4. Finalized by Byzantine quorum (~300 ms)

Result:

- Instant user experience
- Strong consistency and correctness

---

## Example: In-game payment

1. Player enters UID
2. Payment is submitted
3. Balance updates instantly
4. Transaction is finalized within ~300 ms

No waiting. No gas fees.

---

## Developer Quick Start

### Prerequisites

- Rust and Cargo (stable channel recommended)
- Clang, LLVM, CMake (for RocksDB)
- Libssl-dev, pkg-config

### Build CLI

```powershell
cargo build -p kanari
```

### Run CLI

```powershell
# List wallets (first run will bootstrap genesis)
cargo run -p kanari -- keytool list
```

### Move commands

```powershell
# Create new Move package
cargo run -p kanari -- move new my_token

# Test Move package
cargo run -p kanari -- move test ./my_token
```

**Note:** On first run, the CLI performs a Rust-side genesis that mints initial supply. To reset state, remove `~/.kanari/kanari-db/` and rerun.

---

## Testing

```powershell
# Run all tests
cargo test

# Run specific crate tests
cargo test -p kanari-types
```

---

## Project Structure

- `crates/kanari` — CLI binary and bootstrap logic
- `crates/kanari-types` — domain types (accounts, balances, TransferRecord)
- `crates/kanari-move-runtime` — Move VM integration (execution, validation, persistence)
- `crates/kanari-crypto` — key management, signing, crypto utilities
- `crates/kanari-frameworks/packages/kanari-system` — Move packages (on-chain modules)
- `crates/kanari-core` — blockchain engine and DAG consensus
- `crates/centauri` — consensus implementation
- `third_party/move` — bundled Move toolchain (path dependencies)

---

## Local State

- **RocksDB path**: `~/.kari/kanari-db/move_vm_db`
- **State storage**: Serialized `MoveVMState` under key `"state"`
- **Reset state**: Delete the DB directory and restart

---

## Architecture (Advanced)

Kanari uses a distributed execution model with:

- Event-driven execution (no block dependency)
- DAG-based propagation (Narwhal & Bullshark)
- Byzantine quorum consensus (2f+1 finality)
- Parallel transaction processing
- Sparse Merkle Tree state verification

Transactions are executed instantly and finalized asynchronously through DAG consensus.

For detailed architecture documentation:

- [Kanari Core README](crates/kanari-core/README.md)
- [DAG Architecture](crates/centauri/DAG_ARCHITECTURE.md)
- [System ER Diagram](DOCS/SYSTEM_ER.md)

---

## Key Files for Development

- `crates/kanari/src/main.rs` — CLI entry point and bootstrap
- `crates/kanari-move-runtime/src/move_runtime.rs` — Move VM integration
- `crates/kanari-types/src/transfer.rs` — TransferRecord validation
- `crates/kanari-frameworks/packages/kanari-system` — Move modules

---

## Documentation

- [Move CLI Guide](crates/kanari/MOVE_CLI_GUIDE.md) — Complete Move development guide
- [Kanari Core](crates/kanari-core/README.md) — Engine and consensus details
- [Whitepaper](documentation/whitepaper/) — Technical whitepaper
- [Developer Book](documentation/book/) — Comprehensive docs

---

## Vision

Kanari is built for real-time interactive systems like games, where speed, usability, and reliability are critical.

It bridges the gap between traditional game backend systems and verifiable distributed infrastructure.

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

Copyright (c) KanariNetwork, Inc.  
SPDX-License-Identifier: Apache-2.0

---

## Need help?

- **Issues**: [GitHub Issues](https://github.com/kanari-network/kanari-sdk/issues)

- **Docs**: [docs.kanari.network](https://docs.kanarinetwork.site)
