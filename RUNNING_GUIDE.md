# Kanari Blockchain - Running Guide

## Prerequisites

- Rust 1.70+ with Cargo
- Windows/Linux/macOS

## Quick Start

### 1. Build the Project

```bash
cargo build --release
```

### 2. Start the Blockchain Node

```bash
cargo run --bin kanari-node
```

Or use the shortcut commands:

```bash
cargo run --bin kanari-node -- run
cargo run --bin kanari-node -- start
```

## Available Commands

### Node Operations

#### `run` or `start`

Start the blockchain node in continuous mode. The node will:

- Initialize the blockchain with genesis state
- Allocate 10 billion KANARI to the dev address
- Process pending transactions every 5 seconds
- Produce blocks automatically
- Display real-time statistics

```bash
cargo run --bin kanari-node -- run
```

Output example:

```
🚀 Kanari Blockchain Node Starting...
   Network: Testnet
   Move VM: Enabled

📊 Initial State:
   Block Height: 0
   Total Accounts: 5
   Total Supply: 10000000000 Kanari

[timestamp] 🔗 height=0 txs=0 pending=0 accounts=5 wallets=0
```

### Blockchain Information

#### `stats`

Display comprehensive blockchain statistics:

```bash
cargo run --bin kanari-node -- stats
```

Output:

```
📊 Blockchain Statistics:
  Height: 0
  Total Blocks: 1
  Total Transactions: 0
  Pending Transactions: 0
  Total Accounts: 5
  Total Supply: 10000000000 Kanari
```

#### `account <address>`

Get detailed information about a specific account:

```bash
cargo run --bin kanari-node -- account 0x840512ff2c03135d82d55098f7461579cfe87f5c10c62718f818c0beeca138ea
```

Output:

```
👤 Account: 0x840512ff2c03135d82d55098f7461579cfe87f5c10c62718f818c0beeca138ea
  Balance: 10000000000000000000
  Sequence: 0
  Modules: 0
```

#### `block <height>`

Get information about a specific block:

```bash
cargo run --bin kanari-node -- block 0
```

Output:

```
🔗 Block #0
  Timestamp: 1732713600
  Hash: abc123...
  Prev Hash: genesis
  Transactions: 0
```

### Move Module Operations

#### `modules`

List all available Move modules in the Kanari system:

```bash
cargo run --bin kanari-node -- modules
```

Output displays all modules with their functions:

```
📦 Available Modules:

Module: 0x2::kanari
  Functions:
    - init()
    - total_supply()
    - balance_of()

Module: 0x2::balance
  Functions:
    - create()
    - destroy()
    - value()
    ...
```

#### `publish-all`

Publish all framework modules (MoveStdlib + Kanari System):

```bash
cargo run --bin kanari-node -- publish-all
```

This command:

1. Verifies framework paths
2. Publishes MoveStdlib dependencies (address 0x1)
3. Publishes Kanari system modules (address 0x2)
4. Handles module dependencies automatically

#### `publish-file <path>`

Publish a specific Move bytecode file:

```bash
cargo run --bin kanari-node -- publish-file path/to/module.mv
```

#### `inspect <path>`

Inspect Move module bytecode without publishing:

```bash
cargo run --bin kanari-node -- inspect path/to/module.mv
```

Output:

```
ModuleId address: 0x2
ModuleId name: kanari
```

### Wallet Operations

#### `list-wallets`

List all available wallet files:

```bash
cargo run --bin kanari-node -- list-wallets
```

Output:

```
0x1234...5678
0xabcd...ef01 (selected)
```

## System Addresses

The Kanari blockchain uses several special system addresses:

| Address | Name | Purpose | Initial Balance |
|---------|------|---------|-----------------|
| `0x0` | Genesis | System genesis address | 0 |
| `0x1` | Stdlib | Move standard library | 0 |
| `0x2` | Kanari System | Core system modules | 0 |
| `0x840512ff...a138ea` | Dev Address | Development allocation | 10B KANARI |
| `0xbeea2908...2d9dde42` | DAO Address | Gas fee collector | 0 (grows with fees) |

## Gas System

All transactions require gas:

| Transaction Type | Base Gas Cost | Additional Cost |
|-----------------|---------------|-----------------|
| Transfer | 21,000 units | - |
| Publish Module | 50,000 units | +10 per byte |
| Execute Function | 30,000 units | +1,000 per complexity |

**Gas Price**: 1,000 Mist per gas unit (default)

**Gas Collection**: All gas fees are sent to the DAO address for decentralized governance funding.

## Data Storage

The blockchain stores data in RocksDB at:

```
~/.kari/kanari-db/move_vm_db/
```

To reset the blockchain, delete this directory.

## Development Workflow

### 1. Start Development Node

```bash
cargo run --bin kanari-node
```

### 2. Monitor Stats (in another terminal)

```bash
cargo run --bin kanari-node -- stats
```

### 3. Publish Modules

```bash
cargo run --bin kanari-node -- publish-all
```

### 4. Check Account Balances

```bash
# Dev address
cargo run --bin kanari-node -- account 0x840512ff2c03135d82d55098f7461579cfe87f5c10c62718f818c0beeca138ea

# DAO address
cargo run --bin kanari-node -- account 0xbeea29083fee79171d91c39cc257a6ba71c6f1adb7789ec2dbbd79622d9dde42
```

## Troubleshooting

### Build Errors

If you encounter build errors:

```bash
cargo clean
cargo build
```

### Module Not Found

Ensure modules are published:

```bash
cargo run --bin kanari-node -- publish-all
```

### Database Corruption

Reset the blockchain:

```bash
# Windows
Remove-Item -Recurse -Force $env:USERPROFILE\.kari\kanari-db

# Linux/macOS
rm -rf ~/.kari/kanari-db
```

## Examples

### Complete Transaction Flow

1. **Start node**:

   ```bash
   cargo run --bin kanari-node -- run
   ```

2. **Check initial dev balance** (in another terminal):

   ```bash
   cargo run --bin kanari-node -- account 0x840512ff2c03135d82d55098f7461579cfe87f5c10c62718f818c0beeca138ea
   ```

   Expected: 10,000,000,000 KANARI (10 billion)

3. **Monitor DAO address**:

   ```bash
   cargo run --bin kanari-node -- account 0xbeea29083fee79171d91c39cc257a6ba71c6f1adb7789ec2dbbd79622d9dde42
   ```

   Expected: Grows as transactions execute and gas is collected

4. **View blockchain stats**:

   ```bash
   cargo run --bin kanari-node -- stats
   ```

## Performance Tips

- Use `--release` flag for production builds
- Monitor pending transactions to ensure blocks are being produced
- Check disk space for RocksDB storage
- Use `stats` command regularly to monitor blockchain health

## Network Information

- **Network**: Testnet
- **Block Time**: ~5 seconds (configurable)
- **Move VM**: Fully enabled
- **Consensus**: Single-node (development mode)

## Next Steps

- Integrate with wallet CLI (`kanari`)
- Create and fund accounts
- Execute Move smart contracts
- Monitor DAO gas collection
- Build custom Move modules

For wallet operations, see the main `kanari` CLI documentation.
