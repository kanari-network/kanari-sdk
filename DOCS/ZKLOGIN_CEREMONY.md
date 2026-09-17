# zkLogin Production Checklist: Ceremony + Chain Integration

This document tracks the two items that cannot be closed by code alone,
what exactly they require, and what is already done.

## 1. Groth16 ceremony ( toxic waste → canonical VK )

Status: **NOT DONE — demo VKs only.**

The binding circuit (`kanari-crypto/src/signatures/zklogin_circuit.rs`)
is sound and tested, but every VK in existence today comes from a local
random setup (`setup_binding_circuit` / `kanari zklogin setup-circuit`).
Whoever ran the setup knows the trapdoor and can forge proofs for any
statement. No code in this repo can fix that; only independent humans can.

### What a sound ceremony requires

1. **N ≥ 3 independent operators** (different orgs/jurisdictions/machines),
   each running the pinned circuit code at an audited commit.
2. **Sequential transcript**: operator 1 runs setup, publishes
   `(VK_1, transcript_1)`; operator *k* verifies all previous transcripts,
   then contributes. Every step is hash-chained:
   `round_k = SHA256(round_{k-1} || contributor_id || vk_hash_k || timestamp)`.
3. **Verification**: anyone replays the transcript — same code, same seeds
   policy (each operator uses private entropy) — and confirms the final VK.
4. **Pinning**: the final `vk_hash` is hardcoded as the ONLY accepted value
   in the consuming contracts / engine config (`verify_pinned_proof` takes
   the expected hash for exactly this).
5. **Destruction**: each operator attests (signed statement) to deleting
   their toxic waste. This step is social, not technical — which is why
   operator independence is the actual security.

### What the repo provides toward it

- Deterministic, reviewable circuit code (fixed structure; no
  value-dependent branching between setup and prove).
- `vk_fingerprint()` — the canonical 32-byte pin format.
- `verify_pinned_proof()` (Move) — rejects any VK except the pinned hash.
- `setup-circuit` / `prove` / `verify-proof` CLI flow that already speaks
  the final wire format, so swapping a demo VK for a ceremony VK changes
  no code paths — only the pinned hash.
- CI-time negative invariant test (`negative_invariants_canary_transform`)
  proving the statement *really* is "public == hash(witnesses)": honest
  instances satisfy, any public/witness drift does not. Independent
  verifiers can replay it (`cargo test -p kanari-crypto negative_invariants`)
  as part of transcript auditing.

### Verifiable-transcript commitments (what a sound ceremony publishes)

When the ceremony runs, the transcript must be independently auditable,
not just hash-chained. Each round `k` publishes a **signed record**:

```text
round_k = {
  code_commit_sha,        # pinned circuit at an audited git commit
  build_hash,             # reproducible build artifact hash (OS/arch noted)
  prev_round_hash,        # SHA256 of the full round_{k-1} record
  vk_hash_k,              # SHA256 of compressed VK after this contribution
  contribution_hash,      # SHA256 of the contributor's private entropy
  contributor_key_id,     # ed25519/falcon public key label
  timestamp
}
round_k_hash = SHA256(canonical JSON of round_k)
round_k_sig  = Ed25519(sign(round_k_hash, contributor_sk))
```

Why `contribution_hash`: each operator commits to their private entropy
**before** the reveal phase. After the ceremony ends, releasing the entropy
lets anyone confirm `SHA256(entropy_i) == contribution_hash_i` and re-run
the entire contribution — so a contributor cannot swap entropy to
influence the final VK post-hoc, and the transcript is tamper-evident.

The final record also recomputes `vk_fingerprint(final_vk)` and every
operator's `round_k_sig` is collected so no single party can rewrite the
log. The pinned value shipped to contracts is a **weak binding between the
final VK hash and the transcript root**: contracts pin only
`vk_fingerprint(final_vk)` (32 bytes); the full transcript lives
off-chain for adversarial audit, and the `round_k` chain gives it meaning.

### Explicit non-goal

A "re-run setup N times and chain the hashes" tool is NOT provided on
purpose: with full re-setups, only the LAST contributor's honesty matters,
so such a transcript would look like a ceremony while adding zero
soundness. When operators are ready, implement transcript chaining per
§What above (or, better, a Groth16 update protocol), not before.

## 2. Chain acceptance of zkLogin transactions

Status: **dispatch implemented and tested; ingestion NOT wired.**

What exists today (`kanari-crypto`):

- `CurveType::ZkLogin` + `ZkLogin:0x...` tagged senders route through the
  SAME `SignedTransaction::verify_signature()` the mempool already calls
  (`into_verified`), via `verify_zklogin_tx_signature`:
  bundle JSON parse (version-checked) → ephemeral Ed25519 over the tx hash
  → JWT (sig/kid/iss/aud/exp + nonce binding) or Groth16 proof (+ address
  extraction and expected-address match) → recovered address compared with
  the sender. Fail-closed at every step (see `tagged_dispatch_*` tests).
- `sign_message` / keygen / mnemonic paths REJECT ZkLogin (no private key
  exists by design).

What remains for a node to accept such transactions:

1. **Client**: build `ZkLogin:` senders with bundle signatures
   (`encode_zklogin_tx_signature` exists; no CLI transfer path yet).
2. **Ingestion**: route the bundle through verification (already the
   default path once submitted — no engine change needed for the check
   itself, since mempool calls `into_verified`).
3. **Replay**: identical bundles are already dropped by tx-hash dedup
   (`mempool.rs`); per-sender nonce discipline beyond that is unchanged
   from legacy txs (documented gap, same as today).
4. **Epoch binding**: `max_epoch` is enforced against wall-clock JWT `exp`
   for JWT mode; proof mode additionally needs the engine to compare
   `max_epoch` with chain epoch at admission (new check, consensus review
   required).
5. **DoS pricing**: pairing/RSA verification at admission is heavier than
   ed25519; mempool caps apply, but production gas pricing for the zk
   path needs calibration (see gas notes in `zklogin_proof.rs`).

## 3. Done elsewhere (for the record)

- Distinct `Expired` error (bad signature no longer reports as expired).
- Nonce enforcement inside `verify_jwt_with_jwks` (`Some` param).
- JWT timeliness hardening in `verify_claims_timing`: `exp` required and
  enforced with 60s leeway; `iat` and `nbf` (when present) must not lie in
  the future beyond the leeway — a token that is not yet valid or claims a
  future issue time fails closed.
- `sub` enforced non-empty inside `verify_jwt_with_jwks` (an empty subject
  would alias every login of an issuer; non-empty is checked there and at
  address derivation).
- v1 address scheme removed; v2 fixed-width canonical.
- `max_epoch` visibility: recorded in `VerifiedZkLogin`; chain-epoch
  comparison is item 2.4 above.
- Session encryption at rest: 0600 files + logout (password-sealed
  sessions tracked as future work, not a vulnerability today).
