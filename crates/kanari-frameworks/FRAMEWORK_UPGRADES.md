# Framework Upgrades (kanari-system / move-stdlib)

How to ship a Move framework change (new module, new native, changed
signature) without wiping node data.

## Why nodes refuse to boot after a framework change

On startup every node compares the freshly built framework bytecode on disk
against the modules already stored in its database
(`save_framework_modules` in
`move-execution/v1/kanari-move-runtime-v1/src/move_runtime/load_system_modules.rs`).
It runs Move's `Compatibility::full_check`:

- adding a `public fun` / new module / new native: **compatible**, boots fine
- changing a signature the way we did with `transfer::share_object`
  (pure-Move `fun` → `native fun`, `T: store` → `T: key + store`):
  **incompatible**, node exits `FATAL` with
  `BACKWARD_INCOMPATIBLE_MODULE_UPDATE` and refuses to start

This is intentional: silent native swaps would fork the network. The
override below is the supported path for a deliberate upgrade.

## Upgrade without reset (keep data)

1. Rebuild the framework bundles so bytecode on disk is new:

   ```powershell
   cargo test -p kanari-framework-builder
   cargo build -p kanari-node
   ```

2. Stop all running nodes (file lock / partial writes).

3. Start **once** with the one-shot override:

   ```powershell
   $env:KANARI_FRAMEWORK_ALLOW_INCOMPATIBLE=1
   .\setup-multi-node.ps1 -NodeCount 4 -Network devnet -AllowReuseData
   ```

   Expect a warning, not an error:

   ```text
   Warning: Allowing incompatible system upgrade for 0x2::transfer: ...
   ```

4. Unset the variable. The next boot compares against the already-upgraded
   on-disk modules, so no override is needed anymore:

   ```powershell
   Remove-Item Env:KANARI_FRAMEWORK_ALLOW_INCOMPATIBLE
   ```

## When a reset is actually required

- genesis / address layout changes (`0x1`, `0x2` reassigned)
- state-root divergence across nodes (mixed old/new binaries in one round)
- consensus key rotation without shared `consensus-public-keys.json`

Then use the explicit reset flags (never the default):

```powershell
.\setup-multi-node.ps1 -NodeCount 4 -Network devnet -ResetSourceData -ResetReplicaData -ResetConsensusKeys
```

Without any flag, the script **refuses** to boot on reused data — pass
`-AllowReuseData` deliberately (see `setup-multi-node.ps1`).

## Checklist for framework PRs

- [ ] `move test` green for the touched package
      (`cargo run -p kanari -- move test` in the package dir)
- [ ] `cargo test -p kanari-framework-builder` (regenerates bundles)
- [ ] Note in the PR whether the change is compatibility-breaking
      (native added/changed?) so operators know to expect the one-shot env
- [ ] If breaking: verify one node boots with the override before rolling
      the cluster
