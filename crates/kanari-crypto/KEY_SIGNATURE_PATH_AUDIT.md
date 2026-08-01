# Key and Signature Path Audit

Last checked: 2026-08-02

This audit maps where `kanari-crypto` key and signature APIs are consumed outside the crate.

## High-value paths

| Area | Representative files | Current security expectation |
| --- | --- | --- |
| Transaction build/sign | `sdk/kanari_pay/lib/src/modules/transactions/operations.dart`, `crates/kanari-types/src/transaction.rs` | The client signs the exact prepared transaction bytes/hash and submits the signature as bytes. The node/core path must verify against the sender/tagged curve before execution. |
| Core execution tests | `crates/kanari-core/tests/unit/engine_tests.rs`, `crates/kanari-core/tests/unit/produce_dag_vertex_tests.rs` | Tests now consistently use `KeyPair::tagged_address()` and explicit `curve_type` when signing. Untagged verification is fail-closed in `kanari-crypto`. |
| Bench workload | `crates/kanari-benchmarks/src/workload.rs` | Deterministic senders use `keypair_from_private_key`; oversized/malformed import now fails before expensive PQC/hybrid parsing. |
| Wallet app curve mapping | `sdk/kanari_pay/lib/src/kanaricurve.dart` | UI/client curve names must remain aligned with Rust `CurveType` names. Any PQC rename must be versioned to avoid breaking old wallets. |
| Auth private-key storage | `crates/kanari-auth/src/private_key_crypto.rs`, `crates/kanari-auth/src/auth_manager.rs`, `crates/kanari-auth/src/session.rs` | Private-key material should remain encrypted at rest and zeroized on session invalidation. This path does not currently provide PQC-provider migration by itself. |
| Consensus keys | `crates/kanari-node/src/app.rs`, `crates/mysticeti/crates/dag/src/crypto` | Consensus block signing is a separate Ed25519 path and must not be confused with user transaction PQC curves. |

## Findings

- No call site should rely on guessing curves from an untagged address for verification; `verify_signature` rejects untagged addresses.
- `keypair_from_private_key` now rejects oversized formatted private keys before decoding/parsing, reducing adversarial import DoS risk.
- Wallet/keystore compatibility tests cover legacy on-disk formats through public APIs rather than test-only internals.
- PQC provider migration is still open: `pqcrypto-dilithium`, `pqcrypto-sphincsplus`, `pqcrypto-traits`, and `pqcrypto-internals` are unmaintained according to RustSec.

## Migration rule

Do not silently change existing `CurveType` wire names or wallet private-key prefixes.

The safe migration path is:

1. Add a new provider implementation behind the existing `CurveType` variants or introduce explicitly versioned variants.
2. Keep old key import/verification compatibility for existing wallets.
3. Generate new PQC keys with the maintained provider only after cross-provider vectors pass.
4. Add wallet/app/node submit tests for old-wallet signing, new-wallet signing, and mixed verification.
