## 7. Security, Deployment, and Operational Considerations

This chapter covers security best practices, deployment notes, and operational guidance tied to the repository's implementation.

7.1 Cryptography & key management

- Use `crates/kanari-crypto` for canonical keypair formats, signing, and keystore storage. Ensure client wallets use the same address derivation and signature prefixes expected by the runtime and RPC.
- Rotate keys and support hardware-backed keystores for high-value owners and indexers.

7.2 Capability hygiene and least privilege

- Prefer issuing narrow-scoped `UpdateCapability` objects instead of full `OwnerCapability` when delegating.
- Implement short expiration semantics for delegated capabilities where applicable; revocation is provided by Move modules.

7.3 Deterministic natives & validator safety

- Any native function added under `crates/kanari-move-runtime/src/move_runtime/` must be deterministic and thoroughly tested under the test harness in `crates/kanari-move-runtime/tests/`.

7.4 Backups, snapshots, and proof export

- Persistent stores under `crates/kanari-move-runtime/src/storage/` should be regularly snapshotted. Export compact proofs and event logs for external auditing.

7.5 Testing and CI

- Unit tests: run `cargo test -p <crate>` for targeted crates.
- Integration/e2e: use examples under `crates/kanari-move-runtime/examples/` and Move tests under `crates/kanari-frameworks/packages/kanari-system/tests/`.

7.6 Deployment checklist

- Verify native extension determinism across validator builds
- Ensure indexers are configured to consume and persist events reliably
- Configure monitoring for event lag, proof-generation latency, and storage health

References

- Crypto utilities: `crates/kanari-crypto/src/`
- Runtime storage: `crates/kanari-move-runtime/src/storage/`
- Tests & examples: `crates/kanari-move-runtime/examples/`, `crates/kanari-frameworks/packages/kanari-system/tests/`
