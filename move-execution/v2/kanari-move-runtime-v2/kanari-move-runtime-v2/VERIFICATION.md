# Verification scope and attestation

**Status: scoped attestation, not a whole-runtime proof.** Last verified
2026-11-23.

This document records exactly what is machine-checked, what is modelled and
tested, and what is explicitly out of scope for the Lean work in this
directory. It exists so that the result is never over-claimed.

## 1. The bounded claim

> For the **supply-relevant path** of `StateManager::apply_changeset`, every
> state returned by the validated pipeline satisfies the runtime bound
>
> ```text
> ∀ T ∈ tracked,
>   wallet_visible_supply(T) ≤ total_supply(T)
>     ∧ wallet_visible_supply(T) + object_locked_supply(T) ≤ total_supply(T)
> ```
>
> provided the preconditions in §3 hold. This is a theorem about the Lean
> model of those phases, corroborated by a differential test against the real
> Rust per-token check. It is **not** a line-by-line formal verification of the
> Rust runtime, the Move VM, or storage.

The allowed one-sentence summary is:

> _The supply-relevant apply path is covered by machine-checked Lean proofs
> and a Lean/Rust differential test; everything else is modelled and covered
> by tests._

The following claims are **not** made:

- `kanari-move-runtime-v2` is fully formally verified.
- Every Rust line in the apply path is proof-carrying.
- The Move VM, bytecode verifier, RocksDB layer, or scheduler cannot violate
  the supply invariant.
- The `hcover` assumption is proved rather than assumed.

## 2. What is machine-checked

All items below are `sorry`/`admit`/`axiom`-free and built by Lean
`v4.37.0-rc1` with Mathlib pinned in `lake-manifest.json`.

| Area                  | Artifact                                 | Checked statement                                                                                                                                            |
| --------------------- | ---------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Per-token bound       | `Supply/Store.lean`                      | `rustTokenBound_sound`, `rustTokenBound_complete`: `rustTokenBound` mirrors `validate_token_supply_bound`, including `u64::checked_add` overflow rejection   |
| Cap-based mint/burn   | `Supply/Proof.lean`, `Supply/Trace.lean` | `mint_preserves`, `Ledger.burn_preserves`, `coinSupplyOf_transfer_pair`, `reachable_invariant`                                                               |
| Negative case         | `Supply/Counterexample.lean`             | `fundOutside_breaks`, `invariant_fails`, `check_rejects`                                                                                                     |
| Native apply          | `Runtime/ApplyProof.lean`                | `applyChangesetNative_preserves`, `applyChangesetValidated_safe`: checked `i128` net delta, one checked `u64` total update, covered owner debits, full bound |
| Gate                  | `Runtime/FullApplyProof.lean`            | `fullGate_sound`, `fullGate_complete`                                                                                                                        |
| Treasury caps         | `Runtime/FullApplyProof.lean`            | `writeCap_nativeCapAgrees`, `writeCaps_nativeCapAgrees`                                                                                                      |
| Gas object            | `Runtime/FullApplyProof.lean`            | `debitCoin_spec`, `debitCoin_none_of_insufficient`, `applyGas_walletCache_eq_sum`, `applyGas_neg_le`                                                         |
| Native bridge         | `Runtime/FullApplyProof.lean`            | `applyNativePhase_gas_bound`, `applyNativePhase_nativeCapAgrees`, `writeNativeTotal_nativeCapAgrees`                                                         |
| Locked reconciliation | `Runtime/FullApplyProof.lean`            | `reconcileLocked_bound`, `reconcileAll_preserves_bound` using delta `(issued_after - issued_before) - (visible_after - visible_before)`                      |
| Legacy repair         | `Runtime/FullApplyProof.lean`            | `applyLegacyRepair_preserves_bound`                                                                                                                          |
| Non-supply writes     | `Runtime/FullApplyProof.lean`            | `applyNonSupply_preserves_bound`                                                                                                                             |
| Composition           | `Runtime/FullApplyProof.lean`            | `applyFullCore_preserves`, `applyFull_eq_repaired_of_bound`, **`applyFull_safe`**                                                                            |

`applyFull_safe` is the terminal guarantee: if the validated full pipeline
returns a state, that state passes the modelled runtime supply check.

## 3. Preconditions and modelling assumptions

The bounded claim holds only together with these conditions:

1. **`hcover`** (`Supply/Store.lean`):
   ```lean
   hcover : ∀ (T : TokenType) (c : TreasuryCap), capOf T = some c → T ∈ tracked
   ```
   In Rust, `tracked_fungible_token_types` unions `treasury_index`, the wallet
   cache, `GAS_COIN`, and all object-locked records. A treasury entry absent
   from all of those would escape the validation loop. This is the main
   residual assumption.
2. **Tracked set contains the gas coin.** `applyLegacyRepair_preserves_bound`
   and the cached checker assume `gasCoin ∈ tracked`.
3. **Input plausibility.** The changeset model receives the same supply inputs
   Rust computes: owner deltas, created cap writes, gas-object change, the
   canonical candidate-clone index value (`nativeIndexVisible`), and the
   pre-apply snapshots used for reconciliation.
4. **Checked arithmetic.** Native net deltas fit `i128`; totals, caps, and
   locked amounts fit `u64`; rejected overflow is modeled as `none`.
5. **Gate correspondence.** `fullGate` models
   `validate_cached_supply_invariants`; persisted native supply agreement is
   checked by `validate_persisted_native_supply` in the full Rust path.
6. **Supply-field fidelity.** `FullState` encodes the fields that affect the
   bound. Other fields are present to prove write phases supply-neutral, not
   to verify their exact Rust serialization.

## 4. Modelled and tested, but not machine-checked against Rust code

These are covered by the Rust test suite and by the explicit supply-neutral
model, **not** by Lean theorems about the concrete implementation:

- BCS encoding/decoding and RocksDB read/write behaviour;
- object bytes, dynamic-field payloads, index-list maintenance, access
  versions, event emission;
- Move bytecode verification and VM execution, gas-meter internals;
- scheduler/concurrency, checkpoint/RPC paths beyond the cached fast path;
- exact line-by-line control flow of `src/state/apply/mod.rs`.

Rust test evidence on the verified date:

```text
cargo test -p kanari-move-runtime-v2 --no-fail-fast
  => 208 passed; 0 failed
     (144 unit + 64 integration + 0 doc)
```

A single earlier run of `deny_list_cross_transaction` failed once inside
`third_party/move/.../dependencies.rs` (`unwrap()` on `None`); three isolated
reruns and the full `--no-fail-fast` run passed. It is recorded here as an
observed environmental/parallel-test flake, not as part of the attestation.

## 5. Differential test

`testdata/token_bound_vectors.tsv` contains 216 rows generated by
`scripts/GenerateVectors.lean`. Each row is
`total_supply <TAB> wallet_visible <TAB> object_locked <TAB> accept`, where
`accept = 1` exactly when the Lean `rustTokenBound` accepts.

`src/state/supply/supply_validation.rs::tests::lean_vectors_match_rust`
(included under `#[cfg(test)]`) asserts that the real
`StateManager::validate_token_supply_bound` accepts exactly the same rows,
including `u64::MAX` overflow cases. The Rust runtime path is unchanged; only
a test module was added.

## 6. Reproduction

From this directory (`.../kanari-move-runtime-v2/kanari-move-runtime-v2`):

```powershell
lake build
# Build completed successfully (1227 jobs), 0 errors, 0 warnings

lake env lean --run scripts/GenerateVectors.lean
# 216 rows
```

From the workspace root (`D:\move-jamae\kanari-sdk`):

```powershell
cargo test -p kanari-move-runtime-v2 --lib lean_vectors_match_rust
# 1 passed; 0 failed

cargo test -p kanari-move-runtime-v2 --no-fail-fast
# 208 passed; 0 failed

rustfmt --edition 2024 --check `
  move-execution/v2/kanari-move-runtime-v2/src/state/supply/supply_validation.rs
# clean
```

Also checked:

```powershell
# no sorry/admit/axiom in Runtime/
# all .lake/packages/** working trees clean
```

## 7. How to cite this work

Use:

> Machine-checked Lean proofs cover the supply-relevant apply path
> (`Runtime.ApplyProof`, `Runtime.FullApplyProof`) with theorem `applyFull_safe`,
> plus a 216-vector Lean/Rust differential test. The remainder of
> `kanari-move-runtime-v2` is modelled and covered by the Rust test suite, not
> formally verified.

Do **not** shorten this to “the runtime is formally verified”.
