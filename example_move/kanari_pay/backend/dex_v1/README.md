# DEX v1 - Automated Market Maker (AMM)

A decentralized exchange implementation using the constant product formula (x * y = k) on Kanari blockchain.

## 📋 Overview

This module implements a basic AMM DEX with the following features:

- **Liquidity Pools**: Create pools for any token pair
- **Add/Remove Liquidity**: Provide liquidity and receive LP tokens
- **Token Swaps**: Swap between tokens in the pool
- **View Functions**: Query pool state without executing transactions

## 🏗️ Architecture

### Core Components

1. **Pool Structure**
   - Stores reserves for two token types
   - Manages LP token supply
   - Configurable fee percentage (in basis points)

2. **LP Tokens**
   - Represent ownership share in the pool
   - Minted when adding liquidity
   - Burned when removing liquidity

3. **Swap Mechanism**
   - Uses constant product formula: `x * y = k`
   - Fee applied to each swap (configurable)
   - Ensures price impact based on pool depth

## 🚀 Usage

### 1. Create a Pool

```move
use dex_v1::dex_v1;

// Create a pool with 0.3% fee (30 basis points)
dex_v1::create_pool<COIN_A, COIN_B>(30, ctx);
```

### 2. Add Liquidity

```move
let lp_tokens = dex_v1::add_liquidity<CoinTypeA, CoinTypeB>(
    &mut pool,
    coin_a,
    coin_b,
    ctx
);
```

**Initial Liquidity:**

- LP tokens = sqrt(amount_a * amount_b) - 1000
- Minimum initial liquidity: 1000 LP tokens

**Subsequent Liquidity:**

- LP tokens = min((amount_a *total_lp / reserve_a), (amount_b* total_lp / reserve_b))

### 3. Remove Liquidity

```move
let (coin_a, coin_b) = dex_v1::remove_liquidity<CoinTypeA, CoinTypeB>(
    &mut pool,
    lp_coin,
    ctx
);
```

### 4. Swap Tokens

**Swap A for B:**

```move
let coin_b_out = dex_v1::swap_a_for_b<CoinTypeA, CoinTypeB>(
    &mut pool,
    coin_a_in,
    ctx
);
```

**Swap B for A:**

```move
let coin_a_out = dex_v1::swap_b_for_a<CoinTypeA, CoinTypeB>(
    &mut pool,
    coin_b_in,
    ctx
);
```

## 👁️ View Functions (Getters)

All view functions are read-only and can be called via RPC without gas fees.

### Get Pool ID

```move
let pool_id = dex_v1::get_pool_id<CoinTypeA, CoinTypeB>(&pool);
// Returns: address
```

### Get Reserves

```move
let reserve_a = dex_v1::get_reserve_a<CoinTypeA, CoinTypeB>(&pool);
let reserve_b = dex_v1::get_reserve_b<CoinTypeA, CoinTypeB>(&pool);
// Returns: u64
```

### Get LP Supply

```move
let lp_supply = dex_v1::get_lp_supply<CoinTypeA, CoinTypeB>(&pool);
// Returns: u64
```

### Get Fee Percentage

```move
let fee_percent = dex_v1::get_fee_percent<CoinTypeA, CoinTypeB>(&pool);
// Returns: u64 (in basis points, e.g., 30 = 0.3%)
```

### Get All Pool Info

```move
let (reserve_a, reserve_b, lp_supply, fee_percent) = 
    dex_v1::get_pool_info<CoinTypeA, CoinTypeB>(&pool);
// Returns: (u64, u64, u64, u64)
```

### Calculate Swap Output

```move
// Before swapping, calculate expected output
let expected_output = dex_v1::get_swap_a_for_b_output<CoinTypeA, CoinTypeB>(
    &pool,
    amount_in
);
// Returns: u64 (expected output amount)
```

## 🔌 RPC/CLI Examples

### Query Pool Information via CLI

After publishing the package and creating a pool, you can query pool data:

```bash
# First, get your owned objects to find the pool
cargo run -p kanari -- client objects

# Call view function to get pool info
# Format: cargo run -p kanari -- client view <PACKAGE_ID> <MODULE> <FUNCTION> [ARGS]
cargo run -p kanari -- client view 0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146 dex_v1 get_pool_info 0x<POOL_OBJECT_ID>
```

### Expected Response Format

View functions return data in this format:

```json
{
  "action": "view",
  "result": [1000000, 2000000, 1414213, 30],
  "status": "success"
}
```

Where the result array contains:

- `[0]`: Reserve A (u64)
- `[1]`: Reserve B (u64)
- `[2]`: LP Token Supply (u64)
- `[3]`: Fee Percent in basis points (u64)

### Using in Flutter/Dart SDK

```dart
// Query pool information
final response = await client.callViewFunction(
  packageId: '0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146',
  moduleName: 'dex_v1',
  functionName: 'get_pool_info',
  arguments: ['0x<POOL_OBJECT_ID>'],
);

// Extract result from response
if (response['status'] == 'success') {
  final result = response['result'];
  final reserveA = result[0];
  final reserveB = result[1];
  final lpSupply = result[2];
  final feePercent = result[3];
  
  print('Reserve A: $reserveA');
  print('Reserve B: $reserveB');
  print('LP Supply: $lpSupply');
  print('Fee: ${feePercent / 100}%');
}
```

## 🧪 Testing

Run tests:

```bash
cd example_move/kanari_kit/backend/dex_v1
cargo run -p kanari -- move test
```

Test helpers available:

- `create_pool_for_testing`: Create pool without transfer (for tests only)
- `destroy_pool_for_testing`: Cleanup pool (for tests only)

## 📐 Mathematical Formulas

### Swap Calculation

```
amount_out = (amount_in * (10000 - fee_percent) * reserve_out) / 
             (reserve_in * 10000 + amount_in * (10000 - fee_percent))
```

Where:

- `fee_percent` is in basis points (e.g., 30 = 0.3%)
- Formula ensures constant product invariant after fee deduction

### LP Token Minting (Initial)

```
liquidity = sqrt(amount_a * amount_b) - 1000
```

### LP Token Minting (Subsequent)

```
lp_a = (amount_a * total_lp_supply) / reserve_a
lp_b = (amount_b * total_lp_supply) / reserve_b
liquidity = min(lp_a, lp_b)
```

## ⚠️ Error Codes

| Code | Constant | Description |
|------|----------|-------------|
| 1 | `E_INSUFFICIENT_LIQUIDITY` | Output amount exceeds available liquidity |
| 2 | `E_INSUFFICIENT_AMOUNT` | Input amount is zero or negative |
| 3 | `E_INSUFFICIENT_LIQUIDITY_MINTED` | Initial LP tokens would be too low |
| 4 | `E_INSUFFICIENT_LIQUIDITY_BURNED` | LP token amount is invalid |
| 5 | `E_INSUFFICIENT_OUTPUT_AMOUNT` | Calculated output is zero |

## 🔒 Security Considerations

1. **Slippage Protection**: Always check expected output before swapping
2. **Price Impact**: Large swaps relative to pool size cause significant price changes
3. **Impermanent Loss**: LP providers may experience loss compared to holding tokens
4. **Front-running**: Consider using slippage tolerance in production

## 📝 Example: Complete Flow

```move
// 1. Create pool
dex_v1::create_pool<USDC, KANARI>(30, ctx);

// 2. Add initial liquidity
let lp_tokens = dex_v1::add_liquidity<USDC, KANARI>(
    &mut pool,
    usdc_coins,
    kanari_coins,
    ctx
);

// 3. Check pool state
let (reserve_usdc, reserve_kanari, lp_supply, fee) = 
    dex_v1::get_pool_info<USDC, KANARI>(&pool);

// 4. Calculate swap output
let expected_output = dex_v1::get_swap_a_for_b_output<USDC, KANARI>(
    &pool,
    1000  // amount to swap
);

// 5. Execute swap
let kanari_out = dex_v1::swap_a_for_b<USDC, KANARI>(
    &mut pool,
    usdc_in,
    ctx
);

// 6. Remove liquidity later
let (usdc_back, kanari_back) = dex_v1::remove_liquidity<USDC, KANARI>(
    &mut pool,
    lp_tokens,
    ctx
);
```

## 🛠️ Development

### Building

```bash
cargo run -p kanari -- move build
```

### Publishing

```bash
cargo run -p kanari -- move publish --package-path example_move/kanari_kit/backend/dex_v1
```

## 📚 References

- [Constant Product Market Makers](https://github.com/hydrogen-labs/awesome-amm)
- [Uniswap V2 Whitepaper](https://uniswap.org/whitepaper.pdf)
- [Kanari System Documentation](../../../../../documentation/book/src)

## 📄 License

Apache-2.0
