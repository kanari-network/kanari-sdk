# Escrow on Kanari — On-Chain Escrow Deals

Ported from `sdk/kanari_pay/backend/kanari_escrow` to the Kanari framework
(`kanari_system::*`). Compiles with `edition = "2024"`; unit tests:
`kanari move test` (3/3 green).

## Architecture

One module `escrow::escrow`:

```
EscrowDeal<CoinType> (shared protocol object)
  - deal_id, buyer, seller, amount, description
  - state: 1 LOCKED | 2 DELIVERED | 3 COMPLETED | 4 DISPUTED
  - locked_at, delivered_at, completed_at
  - funds: Option<Coin<CoinType>>

EscrowProof (shared protocol object)
  - deal_id
  - entries: vector<ProofEntry>   (state, actor, timestamp, note)
```

## Lifecycle (CLI)

```powershell
$PKG = <published escrow address>

# 1) Buyer creates deal and locks funds (coin object is auto-resolved)
kanari move call --package $PKG --module escrow --function create_deal \
  --type-args 0x2::usd::USD \
  --args <deal_id> <seller_address> <amount> <description> <buyer_coin_id>

# 2) Seller confirms delivery
kanari move call --package $PKG --module escrow --function confirm_delivery \
  --type-args 0x2::usd::USD \
  --args <deal_object_id> <proof_object_id>

# 3) Buyer releases funds to seller
kanari move call --package $PKG --module escrow --function release_funds \
  --type-args 0x2::usd::USD \
  --args <deal_object_id> <proof_object_id>

# 4) Either party raises a dispute (refunds buyer)
kanari move call --package $PKG --module escrow --function raise_dispute \
  --type-args 0x2::usd::USD \
  --args <deal_object_id> <proof_object_id> <reason>
```

Read-only views: `get_state`, `get_state_ref`, `get_deal_details`,
`get_deal_details_ref`, `get_proof_count`, `check_deal_state`.

## App integration (sdk/kanariapp)

The Escrow screen (`EscrowScreen.kt`) talks to the package above through
`EscrowClient.kt` / `EscrowWalletViewModel`, mirroring the DeepBook flow:
build via `kanari_buildCallFunction`, then sign + submit.

## Tests

`tests/escrow_tests.move` covers deal creation with various amounts, sellers,
empty/unicode descriptions.
