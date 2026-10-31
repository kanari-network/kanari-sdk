import KanariMoveRuntime.Supply.Store

/-!
Generate the shared test-vector file consumed by the Rust differential test.

This script prints one row per vector:

    total_supply <TAB> wallet_visible <TAB> object_locked <TAB> accept

where `accept` is `1` exactly when `Supply.rustTokenBound` accepts. The rows
are checked against the real `StateManager::validate_token_supply_bound` by
`src/state/supply/supply_validation.rs` (`tests::lean_vectors_match_rust`).

Regenerate with (from `lean/`):

    lake env lean --run scripts/GenerateVectors.lean > testdata/token_bound_vectors.tsv
-/

open KanariMoveRuntime

namespace KanariMoveRuntime.Supply

def interesting : List Nat :=
  [0, 1, 2, 10, u64Max - 1, u64Max]

def vectors : List (Nat × Nat × Nat) :=
  interesting.flatMap fun total =>
    interesting.flatMap fun wallet =>
      interesting.map fun locked => (total, wallet, locked)

def render (v : Nat × Nat × Nat) : String :=
  let (total, wallet, locked) := v
  let accepted := if rustTokenBound total wallet locked then 1 else 0
  s!"{total}\t{wallet}\t{locked}\t{accepted}"

end KanariMoveRuntime.Supply

def main : IO Unit := do
  for v in KanariMoveRuntime.Supply.vectors do
    IO.println (KanariMoveRuntime.Supply.render v)
