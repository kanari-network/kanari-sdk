# DeepBook on Kanari — Central Limit Order Book (CLOB)

Ported and adapted from the Iota reference design
(`jamesatomc/token_iota` → `sources/DeepBook.move`) to the Kanari framework
(`kanari_system::*`). Compiles with `edition = "2024"`; unit tests:
`kanari move test` (10/10 green).

## Architecture

```
GlobalOrderBookRegistry (shared, one per deployment)
 ├─ books:  blake2b256(sorted type names) -> book address   ← dedupe per pair
 └─ admins: book address -> fee admin address
        │
        ▼
OrderBook<Base, Quote> (shared, one per token pair)
 ├─ bids: vector<LimitOrder>  sorted price DESC, FIFO on ties
 ├─ asks: vector<LimitOrder>  sorted price ASC,  FIFO on ties
 ├─ base_balance / quote_balance        ← maker funds escrowed while resting
 ├─ fee_balance_base / fee_balance_quote← admin-withdrawable
 ├─ fee_bps (≤ 1000 = 10%) · max_depth (≤ 1000 per side)
 └─ base_decimals / quote_decimals      ← raw-unit conversion
```

- **Price encoding** — `PRICE_SCALE = 1e9` quote-per-base; per-book decimals
  convert to raw token units: `quote = qty * price * 10^qd / (1e9 * 10^bd)`.
- **Order types** (`order_type` arg): `0` limit · `1` IOC · `2` FOK ·
  `3` post-only (aborts `E_POST_ONLY_VIOLATION` if it would cross).
- **Matching** — price-time priority against the opposite side; taker pays
  `fee_bps` on the quote leg; fully filled orders are removed, partials rest.
- **Depth cap** — when a side is full the worst resting order is evicted and
  refunded proportionally (`locked_amount * unmatched / quantity`, u128 math).
- **Funds safety** — every price/fee/refund computation routes through
  `kanari_system::math` (`mul_div_u128` with U256 intermediates,
  `bps_mul_floor_u128` for fees, `try_as_u64_from_u128` for narrowing casts),
  so no hand-rolled overflow logic exists in this module; dust orders that
  would round to zero are rejected (`E_INSUFFICIENT_LIQUIDITY`); cancels
  refund the unmatched locked portion to the maker only (`E_UNAUTHORIZED`
  otherwise).
- **Events** — `OrderBookCreated`, `OrderPlaced`, `OrderMatched`,
  `OrderCancelled`, `FeesWithdrawn`, `BookAdminChanged`,
  `OrderBookRegistryCreated` (UIs discover book addresses from these).

## Kanari adaptations vs the Iota original

| Iota                                                  | Kanari                                                                  |
| ----------------------------------------------------- | ----------------------------------------------------------------------- |
| `iota::balance::join`                                 | `kanari_system::balance::merge`                                         |
| `iota::object::uid_to_address`                        | `kanari_system::object::uid_address`                                    |
| `iota::hash::blake2b256`                              | `std::hash::blake2b256`                                                 |
| `iota::{coin,table,event,transfer,tx_context}`        | `kanari_system::{coin,table,event,transfer,tx_context}`                 |
| `module kanari_network::DeepBook;` (legacy)           | `module dex_v1::deepbook { }` (edition 2024)                            |
| `public entry fun get_or_create_order_book…: address` | plain `public fun` — Move entry functions cannot return values          |
| shared objects need only `key`                        | `share_object` requires `key + store` → both singletons declare `store` |

## Lifecycle (CLI)

```powershell
$PKG = <published deepbook address>

# 1) once per deployment — create the shared registry
kanari move call --package $PKG --module deepbook --function create_global_registry

# 2) create (or fetch) the KANARI/USD book: fee 30 bps, depth 100, decimals 9/6
kanari move call --package 0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146 --module deepbook --function get_or_create_order_book_with_decimals --type-args "0x2::kanari::KANARI" --type-args "0x2::usd::USD" --args 0x4e77a197b049225932265caab347a504d59cb9b2dba084a3be5b6ff8274ad9d7 30 100 9 6
#    -> discover the book address from the OrderBookCreated event

# 3) trade (object params are auto-resolved from ids)
kanari move call --package $PKG --module deepbook --function place_bid \
  --type-args 0x2::kanari::KANARI 0x2::usd_coin::USD \
  --args <book_id> <price_1e9> <qty_base> <quote_coin_id> 0
kanari move call --package $PKG --module deepbook --function place_ask \
  --type-args 0x2::kanari::KANARI 0x2::usd_coin::USD \
  --args <book_id> <price_1e9> <qty_base> <base_coin_id> 0
kanari move call --package $PKG --module deepbook --function cancel_order \
  --type-args 0x2::kanari::KANARI 0x2::usd_coin::USD \
  --args <book_id> <order_id>

# 4) admin: collect fees
kanari move call --package $PKG --module deepbook --function withdraw_fees \
  --type-args 0x2::kanari::KANARI 0x2::usd_coin::USD \
  --args <registry_id> <book_id> <to_address> <base_amount> <quote_amount>
```

Read-only quotes (`level/spread/depth`) are available as view functions:
`get_best_bid`, `get_best_ask`, `get_spread`, `get_book_depth`,
`get_bid_count`, `get_ask_count`, `get_locked_balances`, `get_fee_balances`.

## Tests

`tests/deepbook_tests.move` covers the pure-math core without minting coins:
lexicographic pair ordering (registry dedupe), `pow10`, par/scaled price
round-trips, decimal normalization (9/6), dust-order rejection
(`E_INSUFFICIENT_LIQUIDITY`) and u64-blowout refund rejection (`E_OVERFLOW`).
