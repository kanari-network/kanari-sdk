# Validator Security Hardening

## Secret Handling

Consensus private key material is read from `--consensus-private-key-file`; it
is never accepted as a command-line value. Set this before key generation and
node startup:

```powershell
$env:KANARI_CONSENSUS_KEY_PASSWORD = Read-Host "Consensus key password"
$env:KANARI_NODE_IDENTITY_PASSWORD = Read-Host "P2P identity password"
```

`consensus-keygen` writes AES-256-GCM/Argon2 encrypted `.key` files when the
password is present. Mainnet refuses plaintext consensus keys and plaintext P2P
identity files. The P2P identity is persistent, so a restart retains its peer
identity.

Migrate an existing validator seed without changing its authority/public key:

```powershell
$env:KANARI_CONSENSUS_KEY_PASSWORD = Read-Host "Consensus key password"
kanari-node consensus-key-encrypt `
  --input C:\kanari\keys\node1-consensus-private-key.hex `
  --output C:\kanari\keys\node1-consensus-private-key.key
```

Verify startup with the `.key` file before securely retiring the legacy `.hex`.

Environment variables are an integration boundary for an operator secret
provider. Production deployments should inject them from an OS credential
store, HSM sidecar, or orchestrator secret rather than a checked-in script.

## Persistence

Devnet, testnet, and mainnet require persistent storage by default and fail
fast if RocksDB cannot be opened. Only explicit `local`/`test` modes permit an
in-memory engine unless `KANARI_REQUIRE_PERSISTENT_STORAGE` overrides policy.

## Encrypted Full-Validator Backup

Stop the validator first. A successful strict RocksDB open confirms that no
running node owns the database lock.

```powershell
$env:KANARI_VALIDATOR_BACKUP_PASSWORD = Read-Host "Backup password"
kanari-node validator-backup-export `
  --network devnet `
  --data-dir C:\kanari\node1 `
  --consensus-private-key-file C:\kanari\keys\node1-consensus-private-key.key `
  --consensus-public-keys C:\kanari\keys\consensus-public-keys.json `
  --genesis C:\kanari\genesis\devnet-genesis.json `
  --output C:\kanari\backups\node1.kbackup.json
```

The encrypted archive includes the verified logical state snapshot, Mysticeti
WAL, persistent P2P identity, consensus key files, authority public-key map,
and genesis. It has an authenticated encryption tag and an additional SHA3-256
payload checksum.

Restore only into empty directories:

```powershell
$env:KANARI_VALIDATOR_BACKUP_PASSWORD = Read-Host "Backup password"
kanari-node validator-backup-import `
  --network devnet `
  --backup C:\kanari\backups\node1.kbackup.json `
  --data-dir C:\kanari\restored\node1 `
  --recovery-dir C:\kanari\restored\node1-secrets
```

The restore rejects wrong passwords, tampering, network mismatch, unsafe archive
paths, unknown entries, state-root mismatch, and non-empty destinations.

## Network Denial-of-Service Guards

- Gossipsub requires strictly signed messages.
- Encoded gossip messages are capped at 1 MiB.
- Decompressed payloads are capped at 16 MiB to reject compression bombs.
- Checkpoint and DAG synchronization buffers are bounded.
- Incoming and outgoing P2P queues are bounded to 4,096 messages.
- Divergent peers are quarantined.

## Claims That Still Require External Work

These cannot honestly be completed by a source-code patch alone:

- independent third-party audit;
- formal proof of the Kanari integration;
- multi-month production soak test;
- protocol-versioned hybrid/PQ Mysticeti deployment.

Track each activity against the exact source commit and retain its report as a
release artifact. See `../kanari-core/MYSTICETI_SECURITY_PROFILE.md` for the PQ
migration gate and consensus invariants.
