# Mainnet Operations

This runbook is the final operations layer for taking `kanari-node` from a healthy dev or test deployment to a real validator rollout.

## Files

- `validator-committee.example.json`
  Template for the validator set, RPC endpoints, P2P bootstrap addresses, and data directories.
- `start-node.ps1`
  Starts one validator with explicit `--network` and consensus signing keys.
- `setup-multi-node.ps1`
  Starts a rehearsal cluster with one source validator, follower validators, and generated consensus keys.
- `monitor-cluster-health.ps1`
  Checks `kanari_health` and `kanari_getStats` across multiple nodes.
- `backup-node-data.ps1`
  Creates a filesystem backup of a validator data directory.
- `restore-node-data.ps1`
  Restores a validator data directory from a previous backup.

## Preflight

Before any mainnet rollout:

1. Build the binaries you will deploy.
2. Verify Move framework bytecode is present.
3. Confirm each validator has a unique `authority_id`.
4. Confirm each validator has its own dedicated `data_dir`.
5. Confirm each validator has a unique consensus private key.
6. Confirm every validator uses the same reviewed `consensus-public-keys.json` map.
7. Confirm private consensus key files are not committed, shared in chat, or copied between validators.
8. Confirm `KANARI_NETWORK=mainnet` is being used through `--network mainnet`.
9. Confirm `kanari_health` reports:
   - `status = ok`
   - `supply_invariants_ok = true`
   - `strict_persistence_required = true`
   - `strict_checkpoint_roots = true`
   - `persistent_storage_available = true`

## Rollout Plan

Use a staged rollout, not a simultaneous launch.

1. Prepare the committee list from `validator-committee.example.json`.
2. Prepare the consensus public-key map for the same committee IDs.
3. Distribute each validator's private consensus key only to that validator host.
4. Start validator 1 first and confirm:
   - RPC is reachable
   - `kanari_getStats.total_supply > 0`
   - `kanari_health.status = ok`
5. Start the remaining validators one by one.
6. After each validator joins, run cluster monitoring:

```powershell
.\monitor-cluster-health.ps1 `
  -RpcUrls @(
    "http://10.0.0.11:19001",
    "http://10.0.0.12:19001",
    "http://10.0.0.13:19001",
    "http://10.0.0.14:19001"
  ) `
  -RequireEqualHeight `
  -RequireEqualSupply
```

1. Only expose public traffic after the full validator set is healthy.

## Consensus Keys

Generate rehearsal keys with:

```powershell
cargo run --bin kanari-node -- consensus-keygen --node-count 4 --output-dir C:\kanari\mainnet\consensus-keys --force
```

For production, use the generated layout as the required shape, then store and distribute keys through your secure operator process:

```text
consensus-keys/
  consensus-public-keys.json
  node1-consensus-private-key.hex
  node2-consensus-private-key.hex
  node3-consensus-private-key.hex
  node4-consensus-private-key.hex
```

Manual validator start shape:

```powershell
$privateKey = (Get-Content C:\kanari\mainnet\consensus-keys\node1-consensus-private-key.hex -Raw).Trim()

kanari-node start `
  --network mainnet `
  --authority-id 0x1 `
  --authorities 0x1,0x2,0x3,0x4 `
  --data-dir C:\kanari\mainnet\validator1 `
  --p2p-port 19000 `
  --rpc-port 19001 `
  --consensus-private-key-hex $privateKey `
  --consensus-public-keys C:\kanari\mainnet\consensus-keys\consensus-public-keys.json
```

The node should fail fast if the private key is missing or does not match the public key assigned to its `authority_id`.

## Backup Drill

Create a backup before every rollout, restart window, or validator maintenance action.

Example:

```powershell
.\backup-node-data.ps1 `
  -SourceDataDir C:\kanari\mainnet\validator1 `
  -BackupRoot C:\kanari\backups `
  -Label validator1-predeploy
```

Expected result:

- A timestamped backup directory is created.
- `backup-metadata.json` is written next to the copied data.

## Restore Drill

Test restore on a non-production validator before mainnet launch day.

Example:

```powershell
.\restore-node-data.ps1 `
  -BackupDir C:\kanari\backups\validator1-predeploy-20260517-120000 `
  -TargetDataDir C:\kanari\restore-test\validator1 `
  -Force
```

After restore:

1. Start the restored validator with `--network mainnet`.
2. Verify `kanari_health` is `ok`.
3. Verify `kanari_getStats` matches the source validator height and supply.

## Monitoring And Alerting

Minimum required checks:

- RPC reachable
- `kanari_health.status = ok`
- `supply_invariants_ok = true`
- `persistent_storage_available = true`
- all validators report equal `total_supply`
- all validators converge on equal `height`

Suggested alerts:

- Critical: health endpoint unavailable
- Critical: `status != ok`
- Critical: `supply_invariants_ok = false`
- Critical: `persistent_storage_available = false`
- Warning: height drift between validators
- Warning: validator not catching up after restart

## Long-Running Database Policy

RocksDB compaction is automatic. Tune it per validator capacity with:

- `KANARI_DB_BLOCK_CACHE_MB` (default `512`)
- `KANARI_DB_WRITE_BUFFER_MB` (default `64`)
- `KANARI_DB_MAX_OPEN_FILES` (default `4096`)
- `KANARI_DB_MAX_WAL_MB` (default `1024`)
- `KANARI_DB_KEEP_LOG_FILES` (default `10`)
- `KANARI_DB_PERIODIC_COMPACTION_SECS` (default `604800`, seven days)

Archive nodes should leave transaction payload pruning disabled. A non-archive
validator may set `KANARI_HISTORY_RETENTION_CHECKPOINTS` to a positive checkpoint
count. This deletes only old transaction payloads; checkpoint metadata and the
permanent transaction-hash replay index are retained. Take and verify a state
snapshot before enabling or reducing this value. Explorer/indexer infrastructure
must read historical payloads from an archive node.

Never copy a live RocksDB directory. Stop the node or use the snapshot/backup
workflow above so the state root and exported entries come from one committed view.

## Multi-Node Sync Rehearsal

Run this before mainnet launch and after any consensus or state sync changes.

1. Start a rehearsal cluster:

```powershell
.\setup-multi-node.ps1 -Network mainnet -ResetConsensusKeys
```

1. Submit a few real transactions on validator 1.
2. Confirm follower validators catch up.
3. Restart one follower validator.
4. Re-run cluster monitoring with `-RequireEqualHeight -RequireEqualSupply`.
5. Restart validator 1.
6. Re-run monitoring again.

Success criteria:

- no supply drift
- no validator stuck behind
- no node reports degraded health

## Validator And Committee Configuration

Use `validator-committee.example.json` as the single source of truth for:

- validator names
- authority IDs
- P2P bootstrap addresses
- RPC URLs
- data directories

Rules:

- never reuse the same `authority_id`
- never reuse the same consensus private key
- never share one `data_dir` across validators
- never start a validator with a different public-key map than the rest of the committee
- keep committee membership under change control
- rehearse any committee change before applying it to mainnet

## Go/No-Go Checklist

- All validators start with `--network mainnet`
- All validators use persistent local storage
- All validators have explicit consensus signing keys configured
- All validators use the same reviewed consensus public-key map
- Backups were taken and restore was rehearsed
- Cluster health script passes
- Heights match
- Supplies match
- Bootstrap addresses are reachable
- Committee IDs are correct
- On-call owner is assigned for rollout window
- Rollback path is documented and tested
