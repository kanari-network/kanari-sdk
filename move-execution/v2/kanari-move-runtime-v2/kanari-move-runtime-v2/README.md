# kanari-move-runtime-v2 — Lean 4 supply-invariant proof

Machine-checked Lean 4 model and proof of the supply invariant enforced by
`validate_cached_supply_invariants`
(`crates/kanari-move-runtime-v2/src/state/supply/supply_validation.rs`,
called from `state/apply/mod.rs`).

> **Scope of the claim.** The machine-checked result covers the
> **supply-relevant apply path** and its Lean model, not the whole runtime.
> See [VERIFICATION.md](VERIFICATION.md) for the bounded attestation,
> preconditions, and the parts that are only modelled and tested.

```
lake build
```

The library is machine-checked Lean 4 with Mathlib (pinned toolchain
`leanprover/lean4:v4.37.0-rc1`). The library files use the Lean 4.37 module
system (`module`, `public import`, `@[expose] public section`). The
apply-pipeline module uses Mathlib's `List.sum` big operator and
`Multiset.foldl_induction`; the other modules use Lean core tactics
(`omega`, `simp`, `decide`).

## Modules

| Module                                     | Role                                                                                                                                                                                |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `KanariMoveRuntime.Model.Balance`          | `Address`, `TokenType`, `Coin`, `gasCoin`                                                                                                                                           |
| `KanariMoveRuntime.Model.Treasury`         | `TreasuryCap` and its `mint` / `burn` field updates                                                                                                                                 |
| `KanariMoveRuntime.Model.State`            | cached supply state, `State.mintCap` / `burnCap`                                                                                                                                    |
| `KanariMoveRuntime.Supply.Statement`       | `walletVisibleSupply`, `totalSupply`, `SupplyInvariant`, executable `cachedSupplyCheck`                                                                                             |
| `KanariMoveRuntime.Supply.Proof`           | coin-sum lemmas, `mint_preserves`, checker soundness/completeness                                                                                                                   |
| `KanariMoveRuntime.Supply.Trace`           | abstract ledger, trace induction, negative result for out-of-treasury funding                                                                                                       |
| `KanariMoveRuntime.Supply.Counterexample`  | the `usd_shared_cap_mint_pays_out_twice_to_any_recipient` instance                                                                                                                  |
| `KanariMoveRuntime.Supply.Store`           | faithful `issued_supply_for_token` / `u64::checked_add` model and its checker theorems                                                                                              |
| `KanariMoveRuntime.Runtime.Pipeline`       | component-level apply-pipeline model (`applyOwnerDelta` / `applyOwnerDeltas`, `supplyDelta`, `nextTotalSupply`) with Mathlib `List.sum` / `Multiset.foldl_induction` demonstrations |
| `KanariMoveRuntime.Runtime.ApplyProof`     | compositional native apply proof: checked `i128` net delta, one checked `u64` total update, covered owner debits, full runtime bound, and validate-and-reject wrapper               |
| `KanariMoveRuntime.Runtime.FullApplyProof` | supply-relevant full apply-pipeline model: treasury cap write-back, gas-object adjustment, owner recompute, object-locked reconciliation, non-supply writes, and the validate gate  |
| `scripts/GenerateVectors.lean`             | emits `testdata/token_bound_vectors.tsv` for the Rust differential test                                                                                                             |

## The statement, precisely

The runtime check is

```text
wallet_visible_supply(T) <= total_supply(T)
    && wallet_visible_supply(T) + object_locked_supply(T) <= total_supply(T)
```

for every tracked token type `T`. `Statement.lean` mirrors both conjuncts in
`tokenBoundHolds`, and `cachedSupplyCheck` mirrors the loop over tracked
tokens.

The check is **not** a theorem about arbitrary states. A state can hold wallet
coins that were never minted through the treasury cap; `Counterexample.lean`
proves such a state exists and that the check rejects it. What is proved is:

1. **Preservation.** Minting through the cap registered for `T` increases
   wallet-visible supply and the cap total by the same amount
   (`Supply.mint_preserves`); transfers between wallets do not change the
   sum (`Supply.coinSupplyOf_transfer_pair`); burning decreases both sides
   equally (`Supply.Ledger.burn_preserves`).
2. **Native apply preservation.** `Runtime.ApplyProof.applyChangesetNative_preserves`
   proves that the composed native apply transition preserves the full bound
   `wallet <= total && wallet + locked <= total`. The net delta is folded with
   checked `i128` arithmetic, the total is updated **once** with checked `u64`
   arithmetic, and owner debits must be covered; the check gate is modeled by
   `applyChangesetValidated_safe`.
3. **Full apply-pipeline preservation (supply-relevant phases only).**
   `Runtime.FullApplyProof` extends the native proof to the remaining
   supply-relevant phases of
   `StateManager::apply_changeset`: created treasury-cap write-back, explicit
   gas-object debit and owner recompute, object-locked coin reconciliation,
   the candidate-clone legacy wallet repair, and the final
   `validate_cached_supply_invariants` gate. The signed
   reconciliation delta is `(issued_after - issued_before) -
(visible_after - visible_before)`; `reconcileLocked_bound` proves that a
   prestate bound is preserved when that delta is applied with checked
   `u64` arithmetic. `applyFullCore_preserves` composes all phases, and
   `applyFull_safe` proves that any state returned by the validated pipeline
   passes the runtime check. Index / Move-write / access-version writes are
   modeled explicitly and proved supply-neutral.
4. **Trace induction.** Every state reachable from an invariant ledger by
   mint / burn / transfer steps is invariant
   (`Supply.reachable_invariant`).
5. **Checker soundness / completeness.** Accepting a state implies the
   invariant (`Supply.cachedSupplyCheck_sound`, surfaced as `Supply.holds`);
   satisfying the accounted bound implies acceptance
   (`Supply.cachedSupplyCheck_complete`).
6. **The bug is caught.** Funding a wallet outside the treasury cap breaks
   the invariant (`Supply.Ledger.fundOutside_breaks`) and the concrete USD
   instance is rejected (`Counterexample.check_rejects`).

## Deviations from the previous draft

The previous `Supply/Proof.lean` contained

```lean
theorem holds ... : Statement st coins caps capOf isGas nativeTotal := by sorry
```

for an unconstrained `st`. That statement is false (see above), so it could
not be proved without `sorry`. This version:

- drops the redundant `caps`, `isGas` and `nativeTotal` parameters — the gas
  coin's total is the persisted `TreasuryCap` in `capOf`, matching
  `validate_persisted_native_supply`;
- makes the invariant a _validated-state_ property and proves preservation,
  checker soundness, trace induction, and the rejection of the bug state;
- keeps mathlib as a pinned dependency (`[[require]] mathlib` in
  `lakefile.toml`, toolchain `v4.37.0-rc1`) and uses it in the apply-pipeline
  model, while keeping the trusted core model small.

The trusted audit surface is `Model/*` and `Supply/Statement.lean`; everything
in `Proof.lean`, `Trace.lean`, `Counterexample.lean` and `Store.lean` is
checked by Lean.

## Matching the real lookup path (`Store.lean`)

`Statement.lean` abstracts the total supply as an opaque `capOf`. The real Rust
lookup is `issued_supply_for_token`
(`src/state/supply/supply_store.rs`):

```text
GAS_COIN                                  -> state.total_supply
supply:<normalized> TreasuryCap           -> cap.total_supply
supply:<normalized> u64                   -> that u64
supply:<raw> TreasuryCap / u64            -> same, for the raw spelling
otherwise                                 -> global_token_supplies[T]
```

`Supply.Store` models that precedence, plus `u64::checked_add` (the overflow
guard in `validate_token_supply_bound`) and the `tracked_fungible_token_types`
loop. It proves:

- `rustCachedSupplyCheck_sound` — acceptance implies
  `walletCache T ≤ issuedSupply T` and `walletCache T + locked T ≤ issuedSupply T`;
- `rustCachedSupplyCheck_no_overflow` — acceptance rules out accounted-supply
  overflow for a `u64` total;
- `reconciled_bound` — under the cache/canonical reconciliation performed by
  `validate_supply_invariants`, the bound applies to the canonical coin scan;
- `fallback_check_accepts` — the last-resort fallback is **not** a cap: with no
  persisted record, `issuedSupply T = walletCache T` and any `u64` size is
  accepted, so uncapped tokens have no fixed maximum supply in this check;
- `ofState_check_sound` — acceptance by the faithful checker implies
  `SupplyInvariant` for states whose cap totals agree with the persisted caps.

The remaining modelling assumption is `hcover`: every token reachable through
`supply:...` must be tracked. In Rust, `tracked_fungible_token_types` unions the
`treasury_index`, the wallet cache, the gas coin and all object-locked records;
a treasury entry that is absent from all of those would escape the loop.

The full precondition list, the explicit non-claims, and the reproduction
commands are recorded in [VERIFICATION.md](VERIFICATION.md).

## Differential test against Rust

`testdata/token_bound_vectors.tsv` is generated by the Lean checker:

```powershell
# from lean/
lake env lean --run scripts/GenerateVectors.lean > testdata/token_bound_vectors.tsv
```

`src/state/supply/supply_validation.rs::tests::lean_vectors_match_rust` includes
that file and asserts that `StateManager::validate_token_supply_bound` accepts
exactly the rows Lean accepts, including `u64::MAX` overflow cases. Run it with

```powershell
cargo test -p kanari-move-runtime-v2 --lib lean_vectors_match_rust
```
