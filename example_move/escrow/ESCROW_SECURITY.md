# Escrow Security Review

This is an internal pre-production security review. It is **not** a third-party
security audit or a production deployment approval.

## Object model

- `EscrowDeal<CoinType>` and `EscrowProof` are shared objects.
- Funds are held in `EscrowDeal.funds` as a `Coin<CoinType>`.
- Address-owned mutable objects remain sender-only in the runtime.
- Cross-party protocol state uses shared-object inputs, not foreign mutable
  address-owned inputs.

## Authorization invariants

- Only `deal.seller` may confirm delivery.
- Only `deal.buyer` may release funds after `STATE_DELIVERED`.
- Only buyer or seller may raise a dispute.
- The proof `deal_id` must match the deal `deal_id`.
- A deal can transition only once from each valid state.
- Funds are extracted exactly once with `option::extract`.
- Release transfers funds to the seller; dispute transfers funds to the buyer.

## Runtime invariants tested

- Foreign address-owned mutable objects are rejected.
- Shared protocol state is the supported cross-party model.
- Foreign coins remain rejected as mutable inputs.
- Escrow coin split conserves the original amount.
- Core supply accounting passes the supply regression suite.

## Audit events

`EscrowFundsEvent` is emitted for every fund lifecycle transition:

- `action = 1`: locked during deal creation
- `action = 2`: released to seller
- `action = 3`: refunded to buyer

Each event includes `deal_id`, amount, buyer, seller, actor, recipient, and
timestamp. Indexers can reconcile these events against object changes and the
object-locked supply ledger.

## Production blockers

Before production use, complete all of the following:

1. Publish the shared-object package on the target network.
2. Verify the published package/module bytecode and package address.
3. Run end-to-end tests on a clean node: create, confirm, release, dispute,
   replay, wrong buyer, wrong seller, wrong proof, and wrong coin type.
4. Review package upgrade authority and key custody.
5. Obtain an independent Move/security audit for real-value deployment.
