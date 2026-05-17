# Mainnet Operations

This runbook is the final operations layer for taking `kanari-node` from a healthy dev or test deployment to a real validator rollout.

## Files

- `validator-committee.example.json`
  Template for the validator set, RPC endpoints, P2P bootstrap addresses, and data directories.
- `start-node.ps1`
  Starts one validator with explicit `--network`.
- `setup-multi-node.ps1`
  Starts a rehearsal cluster with one source validator and follower validators.
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
5. Confirm `KANARI_NETWORK=mainnet` is being used through `--network mainnet`.
6. Confirm `kanari_health` reports:
   - `status = ok`
   - `supply_invariants_ok = true`
   - `strict_persistence_required = true`
   - `strict_checkpoint_roots = true`
   - `persistent_storage_available = true`

## Rollout Plan

Use a staged rollout, not a simultaneous launch.

1. Prepare the committee list from `validator-committee.example.json`.
2. Start validator 1 first and confirm:
   - RPC is reachable
   - `kanari_getStats.total_supply > 0`
   - `kanari_health.status = ok`
3. Start the remaining validators one by one.
4. After each validator joins, run cluster monitoring:

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

## Multi-Node Sync Rehearsal

Run this before mainnet launch and after any consensus or state sync changes.

1. Start a rehearsal cluster:

```powershell
.\setup-multi-node.ps1 -Network mainnet
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
- never share one `data_dir` across validators
- keep committee membership under change control
- rehearse any committee change before applying it to mainnet

## Go/No-Go Checklist

- All validators start with `--network mainnet`
- All validators use persistent local storage
- Backups were taken and restore was rehearsed
- Cluster health script passes
- Heights match
- Supplies match
- Bootstrap addresses are reachable
- Committee IDs are correct
- On-call owner is assigned for rollout window
- Rollback path is documented and tested
