import KanariMoveRuntime.Runtime.BatchBurnProof
import KanariMoveRuntime.Runtime.ResidualGuards
import KanariMoveRuntime.Supply.Cover

/-!
Executable consistency checks for the residual-risk guards.

This is **not** a Rust differential test: it evaluates the Lean models only.
It checks that the machine-checked guards and the batch-burn supply mirror
agree with the ordinary native apply transition over a small exhaustive grid,
that the Move-write guard rejects supply-critical keys, and that the startup
cover check rejects a legacy raw `supply:<T>` cap that is not in the derived
tracked set.

Run with:

    lake env lean --run scripts/CheckResidualGuards.lean
-/

open KanariMoveRuntime
open KanariMoveRuntime.Runtime
open KanariMoveRuntime.Runtime.BatchBurn
open KanariMoveRuntime.Runtime.ResidualGuards
open KanariMoveRuntime.Supply
open KanariMoveRuntime.Supply.Cover

namespace ResidualCheck

def totals : List Nat := [0, 1, 5, 10]
def wallets : List Nat := [0, 1, 5, 10]
def locked : List Nat := [0, 1, 3]
def deltaSets : List (List Int) := [[], [-1], [-2, 1], [1], [-10], [-20], [5, -3]]

def eligible (ds : List Int) : BatchBurnChangeSet :=
  { success := true, forbiddenExtras := false, oneGasObject := true,
    oneCreatedObject := true, canonicalGasObject := true, nativeCoinObject := true,
    paymentOwnerMatches := true, noModulesAdded := true, senderIsDebit := true,
    gasCreditLeDebit := true, objectBalanceCoversDebit := true,
    objectWriteSucceeds := true, deltas := ds }

def nativeStates : List NativeState :=
  totals.flatMap fun total =>
    wallets.flatMap fun wallet =>
      locked.map fun lock => { total := total, locked := lock, balances := [wallet] }

def validNative (s : NativeState) : Bool :=
  decide (s.total ≤ u64Max ∧ s.balances.sum ≤ s.total ∧
    s.balances.sum + s.locked ≤ s.total)

def nativeOk (s : NativeState) (ds : List Int) : Bool :=
  match applyChangesetNative s ds with
  | none => true
  | some s' => nativeTokenBound s' == true

def batchOk (s : NativeState) (ds : List Int) : Bool :=
  match batchBurnNative s (eligible ds) with
  | none => true
  | some s' =>
      nativeTokenBound s' == true &&
      decide (s'.total ≤ s.total) &&
      (match applyChangesetNative s ds with
       | some t => t.total == s'.total && t.locked == s'.locked && t.balances == s'.balances
       | none => false)

def keyClass : Nat → SupplyWriteClass
  | 0 => .totalSupply
  | 1 => .globalSupplies
  | 2 => .supplyKey
  | 3 => .treasuryKey
  | _ => .other

def baseFull : FullState :=
  { total := 10, ownerBalances := [5, 5], walletCache := fun _ => 10,
    caps := fun T => if T = gasCoin then some 10 else none,
    coins := [], locked := fun _ => 0, indexes := [], moveWrites := [],
    accessVersions := 0 }

def guardOk (writes : List Nat) : Bool :=
  let anyCritical := writes.any (fun k => supplyCritical (keyClass k))
  match guardedApplyMoveWrites baseFull keyClass writes with
  | none => anyCritical == true
  | some s' =>
      anyCritical == false && s'.total == baseFull.total &&
        s'.walletCache gasCoin == baseFull.walletCache gasCoin &&
        s'.moveWrites == writes

def guardWrites : List (List Nat) := [[], [9, 10], [0], [2], [9, 3, 10], [1, 2, 3]]

def usd : TokenType := ⟨"0x2::usd::USD", false⟩

def cap : TreasuryCap :=
  { total_supply := 7, owner := ⟨"0x1"⟩, tokenType := usd }

def rawSupply : SupplyStore :=
  { nativeTotal := 10, supplyKey := fun _ => none,
    supplyKeyRaw := fun T => if T = usd then some (PersistedSupply.treasury cap) else none,
    walletCache := fun _ => 0 }

def rawStore : TrackedStore := TrackedStore.mk rawSupply [] [] []

def rawStoreTracked : TrackedStore := { rawStore with walletKeys := [usd] }

def coverOk : Bool :=
  coverCheckStartup rawStore [usd] == false &&
  coverCheckStartup rawStoreTracked [usd] == true &&
  hasPersistedTreasury rawStore usd == true

def failures : List String :=
  (nativeStates.filter (fun s => validNative s == true)).flatMap fun s =>
    deltaSets.filter (fun ds => nativeOk s ds == false || batchOk s ds == false)
      |>.map fun ds => s!"native/batch mismatch: total={s.total}, locked={s.locked}, balances={s.balances}, deltas={ds}"

def guardFailures : List String :=
  guardWrites.filter (fun ws => guardOk ws == false)
    |>.map fun ws => s!"guard mismatch: {ws}"

def main : IO Unit := do
  let nativeCount := (nativeStates.filter (fun s => validNative s == true)).length * deltaSets.length
  let errs := failures ++ guardFailures ++
    (if coverOk then [] else ["startup cover mismatch"])
  if errs.isEmpty then
    IO.println s!"Residual guard check passed: {nativeCount} native/batch vectors, \
      {guardWrites.length} guard vectors, 2 startup cover vectors"
  else
    for e in errs do IO.eprintln e
    IO.Process.exit 1

end ResidualCheck

def main : IO Unit := ResidualCheck.main
