# Framework Upgrade, Hash Pinning, and Rollback Policy

## 1. Scope

This policy applies to production validators running `kanari-move-runtime-v2`
with the persistent store. Tests and examples may use embedded framework
bytecode for determinism, but a production node must load framework modules from
disk and verify them before publishing state.

The retired `kanari-move-runtime-v1` is disconnected from the workspace (its
source is retained for reference only) and is out of scope.

## 2. What is pinned

`load_system_modules::framework::verify_framework_hash` computes a BLAKE3 digest
over the sorted module manifest:

```text
for each module: "<address>::<name>" NUL blake3(module_bytes) NUL
framework_hash = blake3(concatenated manifest entries)
```

Each framework has its own expected-hash environment variable:

| Framework         | Address | Expected-hash variable               |
| ----------------- | ------- | ------------------------------------ |
| Move stdlib       | `0x1`   | `KANARI_FRAMEWORK_EXPECTED_HASH_0X1` |
| Kanari system     | `0x2`   | `KANARI_FRAMEWORK_EXPECTED_HASH_0X2` |
| Kanari governance | `0x3`   | `KANARI_FRAMEWORK_EXPECTED_HASH_0X3` |

Startup fails closed when a pinned variable does not match the hash computed
from the disk artifacts. Without a pin, a hash change is only logged as a
warning; that is acceptable for local development but **not** for mainnet.

The loader also:

- discovers modules from the on-disk `bytecode_modules` directory (or the
  explicit `KANARI_FRAMEWORK_MODULES_DIR*` override);
- topologically sorts them and rejects an empty module set;
- verifies module dependencies with `move_bytecode_verifier::dependencies`;
- checks `Compatibility::full_check()` before replacing existing module bytes;
- persists the manifest and hash in the persistent store.

## 3. Upgrade procedure

1. **Build from a signed tag.** Use a release artifact produced by the signed
   release workflow; do not copy loose `.mv` files between machines.
2. **Measure the hash in staging.** Start one non-voting staging node from the
   exact artifact bundle and record the `framework hash (disk): <hex>` log line
   for `0x1`, `0x2`, and `0x3`. A second operator must independently reproduce
   the same three hashes from the same bundle.
3. **Publish the manifest.** The release manager adds the three hashes and the
   artifact digest to the release notes/`framework-hashes.json`, and the signed
   release workflow signs that file together with the binaries.
4. **Set all three pins before starting the node.** Example:

   ```powershell
   $env:KANARI_FRAMEWORK_EXPECTED_HASH_0X1="<signed 0x1 hash>"
   $env:KANARI_FRAMEWORK_EXPECTED_HASH_0X2="<signed 0x2 hash>"
   $env:KANARI_FRAMEWORK_EXPECTED_HASH_0X3="<signed 0x3 hash>"
   ```

5. **Coordinate the rollout.** All validators in a committee must run the same
   framework hash. Stop validators, update artifacts and pins, then restart them
   together before resuming normal transaction load. Do not mix framework
   versions inside one committee: divergent module sets can produce divergent
   state roots and checkpoint failures.
6. **Verify convergence.** Confirm every node reports the same height,
   transaction count, and state root before re-enabling client traffic.

## 4. Rollback and downgrade rules

- A downgrade is treated exactly like an upgrade: it changes the hash, so pinned
  nodes reject it. This is intentional rollback protection.
- Rolling back is permitted only to a previously signed hash and only as a
  coordinated committee action with a verified state snapshot from before the
  failed upgrade. Update all three pins to that signed hash at the same time.
- `KANARI_FRAMEWORK_ALLOW_INCOMPATIBLE=1` is a break-glass override for
  compatibility failures. It requires an incident record, two-operator approval,
  and a mandatory follow-up upgrade. Never set it in mainnet service
  configuration.
- If a node reports a hash mismatch, leave it stopped. Do not "fix" it by
  unsetting the pin on a production validator.

## 5. Evidence required for audit

- Signed release tag and commit hash.
- Signed `framework-hashes.json` with the three expected hashes.
- Two independent hash measurements from the same artifact bundle.
- Rollout record with validator identities, start times, and post-restart root
  convergence.
- Any break-glass override ticket and remediation commit.

This document defines the required operating policy; it is not a claim that an
external auditor has certified the rollout.
