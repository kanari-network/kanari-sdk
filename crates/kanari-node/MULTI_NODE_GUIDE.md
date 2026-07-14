# Kanari Multi-Node Setup Guide

Guide for running multiple `kanari-node` validators with libp2p networking and explicit consensus signing keys.

## Build

```powershell
cargo build -p kanari-node
```

For release binaries:

```powershell
cargo build -p kanari-node --release
```

## Consensus Keys

DAG consensus no longer falls back to deterministic demo keys. Every validator must start with:

- one unique private consensus signing key
- one shared `consensus-public-keys.json` file containing the public keys for the whole authority set

Generate local keys for a 4-node test cluster:

```powershell
cargo run --bin kanari-node -- consensus-keygen --node-count 4 --output-dir .\consensus-keys --force
```

This creates:

```text
consensus-keys/
  consensus-public-keys.json
  node1-consensus-private-key.key
  node2-consensus-private-key.key
  node3-consensus-private-key.key
  node4-consensus-private-key.key
```

Keep private key files out of git and do not reuse one private key across validators.

## Fast Local Setup

The PowerShell setup script generates consensus keys automatically when they are missing.

```powershell
.\setup-multi-node.ps1 -NodeCount 4 -Network devnet -ResetSourceData -ResetReplicaData -ResetConsensusKeys
```

By default it stores keys under:

```text
%USERPROFILE%\.kanari\consensus-keys
```

Start nodes in separate terminals:

```powershell
.\start-node.ps1 -NodeId 1 -Network devnet -Authorities "0x1,0x2,0x3,0x4"
```

```powershell
.\start-node.ps1 -NodeId 2 -Network devnet -Authorities "0x1,0x2,0x3,0x4" -Bootstrap "/ip4/127.0.0.1/tcp/19000"
```

```powershell
.\start-node.ps1 -NodeId 3 -Network devnet -Authorities "0x1,0x2,0x3,0x4" -Bootstrap "/ip4/127.0.0.1/tcp/19000"
```

```powershell
.\start-node.ps1 -NodeId 4 -Network devnet -Authorities "0x1,0x2,0x3,0x4" -Bootstrap "/ip4/127.0.0.1/tcp/19000"
```

`start-node.ps1` passes the matching `node<N>-consensus-private-key.key` file path and the shared `consensus-public-keys.json`; private key contents never appear in process arguments. Set `KANARI_CONSENSUS_KEY_PASSWORD` before key generation/start to encrypt these files. RPC binds to `0.0.0.0` by default, so use firewall rules or a narrower `-RpcHost` outside a trusted dev network.

## Manual Start

Manual start commands must pass the consensus private key and public-key map explicitly.

### Node 1

```powershell
cargo run --bin kanari-node -- start `
  --network devnet `
  --p2p-port 19000 `
  --rpc-port 19001 `
  --data-dir data/node1 `
  --authority-id 0x1 `
  --authorities 0x1,0x2,0x3,0x4 `
  --consensus-private-key-file .\consensus-keys\node1-consensus-private-key.key `
  --consensus-public-keys .\consensus-keys\consensus-public-keys.json
```

### Node 2

```powershell
cargo run --bin kanari-node -- start `
  --network devnet `
  --p2p-port 19010 `
  --rpc-port 19011 `
  --data-dir data/node2 `
  --authority-id 0x2 `
  --authorities 0x1,0x2,0x3,0x4 `
  --consensus-private-key-file .\consensus-keys\node2-consensus-private-key.key `
  --consensus-public-keys .\consensus-keys\consensus-public-keys.json `
  --bootstrap "/ip4/<node1-ip>/tcp/19000"
```

### Node 3

```powershell
cargo run --bin kanari-node -- start `
  --network devnet `
  --p2p-port 19020 `
  --rpc-port 19021 `
  --data-dir data/node3 `
  --authority-id 0x3 `
  --authorities 0x1,0x2,0x3,0x4 `
  --consensus-private-key-file .\consensus-keys\node3-consensus-private-key.key `
  --consensus-public-keys .\consensus-keys\consensus-public-keys.json `
  --bootstrap "/ip4/<node1-ip>/tcp/19000"
```

### Node 4

```powershell
cargo run --bin kanari-node -- start `
  --network devnet `
  --p2p-port 19030 `
  --rpc-port 19031 `
  --data-dir data/node4 `
  --authority-id 0x4 `
  --authorities 0x1,0x2,0x3,0x4 `
  --consensus-private-key-file .\consensus-keys\node4-consensus-private-key.key `
  --consensus-public-keys .\consensus-keys\consensus-public-keys.json `
  --bootstrap "/ip4/<node1-ip>/tcp/19000"
```

## Shared Genesis Manifest

Create one manifest from the exact binary/release used by the cluster and copy
it to every new node before startup:

```powershell
cargo run --bin kanari-node -- genesis-export `
  --network devnet `
  --data-dir .\data\node1 `
  --output .\genesis\devnet-genesis.json
```

Start every node with the same manifest:

```powershell
--genesis .\genesis\devnet-genesis.json
```

The node validates the network, protocol version, state schema version,
genesis checkpoint hash, and genesis state root before joining peers. If any
value differs, startup stops. Do not disable this validation or copy a live
database as a substitute for the manifest.

`setup-multi-node.ps1` now creates this manifest automatically from the source
node data directory when it is missing and passes it to every launched node.
To use a different location, pass `-GenesisPath <PATH>` to the setup script.

For a node that joins an already-running chain, create a snapshot while the
source node is stopped:

```powershell
cargo run -p kanari-node -- snapshot-export `
  --network devnet `
  --data-dir "$env:USERPROFILE\.kanari\kanari-db" `
  --output .\snapshots\devnet-height-5.json
```

If export reports a checkpoint/state-root mismatch because the source database
uses an older state-root schema, audit that database first, then opt into an
explicit migration export. The snapshot records both roots and does not
silently rewrite checkpoint history:

```powershell
cargo run -p kanari-node -- snapshot-export `
  --network devnet `
  --data-dir "$env:USERPROFILE\.kanari\kanari-db" `
  --output .\snapshots\devnet-migrated.json `
  --allow-state-root-migration
```

Then import it into fresh replica directories during setup:

```powershell
.\setup-multi-node.ps1 -NodeCount 5 -Network devnet `
  -SnapshotPath .\snapshots\devnet-height-5.json
```

Import verifies the snapshot hash, genesis identity, checkpoint height, and
state root before writing. It refuses to overwrite a non-empty data directory.

## Important Start Options

- `--network <NETWORK>`: selects `devnet`, `testnet`, or `mainnet`
- `--authority-id <ID>`: validator authority ID, for example `0x1`
- `--authorities <IDS>`: comma-separated committee, for example `0x1,0x2,0x3,0x4`
- `--consensus-private-key-file <PATH>`: encrypted consensus key file (plaintext is rejected on mainnet)
- `--consensus-public-keys <PATH>`: JSON map of authority ID to public key hex
- `--p2p-port <PORT>`: P2P networking port
- `--rpc-port <PORT>`: RPC server port
- `--rpc-host <HOST>`: RPC bind address
- `--data-dir <PATH>`: blockchain and state data directory
- `--bootstrap <MULTIADDR>`: bootstrap peer, can be specified multiple times
- `--genesis <PATH>`: shared genesis manifest required for safe node joining
- `--relay-server`: enable circuit relay server mode

## Relay Server Mode

Relay mode also needs consensus keys when the node participates as a validator.

```powershell
kanari-node start `
  --network devnet `
  --p2p-port 19000 `
  --rpc-port 19001 `
  --authority-id 0x1 `
  --authorities 0x1,0x2,0x3,0x4 `
  --consensus-private-key-file .\consensus-keys\node1-consensus-private-key.key `
  --consensus-public-keys .\consensus-keys\consensus-public-keys.json `
  --relay-server
```

## Data Directories

Each node must have a separate data directory.

Windows:

```powershell
--data-dir C:\Users\<Username>\.kanari\kanari-db\node1
--data-dir C:\Users\<Username>\.kanari\kanari-db\node2
--data-dir C:\Users\<Username>\.kanari\kanari-db\node3
--data-dir C:\Users\<Username>\.kanari\kanari-db\node4
```

Linux/macOS:

```bash
--data-dir ~/.kanari/kanari-db/node1
--data-dir ~/.kanari/kanari-db/node2
--data-dir ~/.kanari/kanari-db/node3
--data-dir ~/.kanari/kanari-db/node4
```

## RPC Endpoints

Local endpoints:

- Node 1: `http://127.0.0.1:19001`
- Node 2: `http://127.0.0.1:19011`
- Node 3: `http://127.0.0.1:19021`
- Node 4: `http://127.0.0.1:19031`

To expose RPC to the LAN, bind with `--rpc-host 0.0.0.0` or a specific machine IP. Only do this on a trusted network or behind firewall rules.

## Check State Root Alignment

Devnet and mainnet nodes verify checkpoint state roots strictly by default. If one node reports the same height but a different `state_root`, treat it as a stop-the-line divergence: stop that node and resync its data instead of continuing with a split root.

```powershell
$ports=@(19001,19011,19021,19031)
foreach ($p in $ports) {
  try {
    $body=@{jsonrpc='2.0';method='kanari_getStats';params=@();id=1} | ConvertTo-Json -Compress
    $r=Invoke-RestMethod -Uri "http://127.0.0.1:$p" -Method Post -ContentType 'application/json' -Body $body -TimeoutSec 2
    "${p}: height=$($r.result.height) txs=$($r.result.total_transactions) pending=$($r.result.pending_transactions) owners=$($r.result.total_owners) root=$($r.result.state_root)"
  } catch {
    "${p}: ERROR $($_.Exception.Message)"
  }
}
```

For temporary local-only experiments you can opt out with `KANARI_STRICT_CHECKPOINT_ROOTS=0`, but do not run shared devnet, testnet, or mainnet validators with relaxed checkpoint roots.

## Long-Running Multi-Node Soak Test

Use this flow before treating a branch as mainnet-ready. It exercises deterministic execution, object/gas invariants, consensus finality, and the RPC contract for several hours instead of only one happy-path transaction.

### 1. Start From a Clean 4-Node Devnet

```powershell
.\setup-multi-node.ps1 -NodeCount 4 -Network devnet -ResetSourceData -ResetReplicaData -ResetConsensusKeys
```

Start the four nodes with the commands from **Fast Local Setup**. Wait until all nodes report synced heights in the explorer or logs.

### 2. Confirm RPC Health on Every Node

```powershell
$nodes = @(
  "http://127.0.0.1:19001",
  "http://127.0.0.1:19011",
  "http://127.0.0.1:19021",
  "http://127.0.0.1:19031"
)

foreach ($node in $nodes) {
  Invoke-RestMethod -Uri $node -Method Post -ContentType "application/json" -Body '{"jsonrpc":"2.0","method":"kanari_health","params":[],"id":1}'
  Invoke-RestMethod -Uri $node -Method Post -ContentType "application/json" -Body '{"jsonrpc":"2.0","method":"kanari_getNetworkStatus","params":[],"id":2}'
}
```

Success criteria:

- `supply_invariants_ok` is `true`
- `strict_persistence_required` is `true` for persistent-node testing
- every node sees the same authority set
- no node reports state-root divergence

### 3. Run a Sustained Transaction Loop

Set the target and password for your local wallet, then run a loop. Keep `--count` modest so object/gas sequencing issues show up clearly instead of being hidden by a huge backlog.

```powershell
$to = "0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146"
$password = "@Password12345678"
$end = (Get-Date).AddHours(6)

while ((Get-Date) -lt $end) {
  cargo run -p kanari -- client transfer --to $to --amount 0.01 -p $password
  cargo run -p kanari -- client stress-test --to $to --amount 0.001 --count 10 -p $password
  Start-Sleep -Seconds 10
}
```

Success criteria:

- no `Gas payment object ... cannot overlap with a mutable object input`
- no invalid pending transaction repeatedly blocks checkpoint production
- submitted transactions become committed, not only pending
- checkpoint height increases on all nodes

### 4. Check Finality Alignment

Sample block height and latest full block from every node.

```powershell
foreach ($node in $nodes) {
  $height = Invoke-RestMethod -Uri $node -Method Post -ContentType "application/json" -Body '{"jsonrpc":"2.0","method":"kanari_getBlockHeight","params":[],"id":3}'
  $height.result
}
```

Use the explorer **State Divergence Audit** and **Latest Checkpoint** panels to compare:

- checkpoint height
- checkpoint hash
- state root
- transaction count
- object change count

All readable nodes should converge to the same canonical height, hash, and state root. A short delay is acceptable during sync; persistent disagreement is a stop-the-line failure.

### 5. Stop-The-Line Conditions

Stop the run and inspect logs immediately if any of these appear:

- checkpoint production repeatedly fails on the same pending transaction
- state root differs between nodes at the same checkpoint height
- a checkpoint has invalid `prev_checkpoint_hash`
- duplicate transaction hashes appear in the same checkpoint
- RPC returns transaction errors without structured `reason` data for known policy failures
- object refs accepted by RPC do not match current object `version` and `digest`

## Troubleshooting

### Node Fails With Missing Consensus Key

Run `consensus-keygen`, then pass `--consensus-private-key-file` and `--consensus-public-keys`, or use `start-node.ps1` with the correct `-ConsensusKeyDir`.

### Node Fails With Consensus Public Key Mismatch

The private key for this node does not match the public key listed for its `--authority-id`. Regenerate the key set or use the correct private key file for that node.

### Nodes Cannot Find Each Other

Check unique P2P ports, firewall rules, and `--bootstrap` multiaddrs. On the same LAN, mDNS can discover peers automatically.

### Block Sync Not Working

Check logs, verify every node uses the same `--authorities` list and `consensus-public-keys.json`, then restart one follower after the source node is healthy.
