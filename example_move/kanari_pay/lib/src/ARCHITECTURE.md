# KanariClient Architecture

## 📁 Modular Structure

This project has been refactored to have a modular structure similar to the `escrow_client.dart` pattern for easier maintenance and extension.

```mmarkdown
lib/src/
├── kanari_client.dart              # Main facade (backward compatible)
│
├── core/                           # 🔥 Shared utilities
│   ├── bcs_serializers.dart        # BCS encoding/decoding helpers
│   ├── rpc_utils.dart              # HTTP/RPC helper functions
│   └── core.dart                   # Barrel file
│
└── modules/                        # 🔥 Feature modules
    ├── transactions/               # ✅ Transaction operations (implemented)
    │   ├── constants.dart          
    │   ├── operations.dart         
    │   └── transactions.dart       
    ├── queries.dart                # ✅ Read-only blockchain queries (implemented)
    │
    ├── tokens/                     # 📝 Token/Fungible assets (template)
    │   ├── constants.dart          
    │   ├── operations.dart         
    │   ├── queries.dart            
    │   └── tokens.dart             
    │
    ├── nft/                        # 📝 NFT & Collections (template)
    │   ├── constants.dart          
    │   ├── operations.dart         
    │   ├── queries.dart            
    │   └── nft.dart                
    │
    ├── defi/                       # 📝 DeFi (DEX, Lending) (template)
    │   ├── constants.dart          
    │   ├── operations.dart         
    │   ├── queries.dart            
    │   └── defi.dart               
    │
    ├── escrow/                     # ✅ Escrow system (separate module)
    │   ├── constants.dart          
    │   ├── models.dart             
    │   ├── operations.dart         
    │   ├── queries.dart            
    │   └── escrow.dart             
    │
    └── modules.dart                # Central barrel file
```

**Legend:**

- ✅ = Implemented and ready to use
- 📝 = Template created, ready for implementation
- 🔧 = In progress

## ✅ Design Principles

### 1. **Facade Pattern**

`KanariClient` acts as a facade that delegates to various modules:

- **QueriesModule**: Handles read operations (getAccount, getBalance, etc.)
- **TransactionOperations**: Handles write operations (transfer, executeFunction, etc.)

### 2. **Separation of Concerns**

- **Core Layer**: Shared utility functions (BCS serialization, RPC calls)
- **Modules Layer**: Business logic separated by responsibility
- **Facade Layer**: Unified API for external usage

### 3. **Backward Compatibility**

All original APIs work as before, no breaking changes.

### 4. **Modular Extensibility**

Easy to add new features by creating new modules following available templates.

## 🎯 Usage Examples

### Original Usage (Still Works)

```dart
final client = KanariClient('http://localhost:3000');

// Queries
final account = await client.getAccount(address);
final balance = await client.getBalance(address);

// Transactions
await client.transfer(
  wallet: wallet,
  recipient: recipientAddress,
  amount: 1000,
);

await client.executeFunction(
  wallet: wallet,
  package: packageAddress,
  module: 'module_name',
  function: 'function_name',
);
```

### Direct Module Access (For Advanced Usage)

```dart
import 'package:kanari_kit/src/modules/modules.dart';

// Access queries module directly
final queries = client.queries;
final account = await queries.getAccount(address);

// Access transactions module directly
final transactions = client.transactions;
await transactions.transfer(
  wallet: wallet,
  recipient: recipientAddress,
  amount: 1000,
);

// Use template modules (when implemented)
final tokenOps = TokenOperations(url, queries, client);
final nftQueries = NftQueries(url, queries, client);
```

## 🚀 Adding New Features

### Method 1: Using Template Modules (Recommended)

See detailed guide at: [`MODULE_DEVELOPMENT_GUIDE.md`](./modules/MODULE_DEVELOPMENT_GUIDE.md)

**Short Steps:**

1. Copy template from `tokens/`, `nft/`, or `defi/`
2. Edit constants, operations, queries as needed
3. Implement methods by removing `throw UnimplementedError()`
4. Export in `modules.dart`
5. (Optional) Add wrapper in `KanariClient`

### Method 2: Adding New Query Function

1. Open `modules/queries.dart`
2. Add new method:

```dart
Future<MyType> myNewQuery(String param) async {
  final resp = await RpcUtils.request(
    client,
    url,
    'kanari_myMethod',
    {'param': param},
    (j) => MyType.fromJson(j as Map<String, dynamic>),
  );
  if (resp.error != null) throw Exception(resp.error!.message);
  return resp.result!;
}
```

1. Add wrapper in `kanari_client.dart`:

```dart
Future<MyType> myNewQuery(String param) {
  return _queries.myNewQuery(param);
}
```

### Method 3: Adding New Transaction Operation

1. Open `modules/transactions/constants.dart`
2. Add constant:

```dart
static const String rpcMyMethod = 'kanari_myMethod';
```

1. Open `modules/transactions/operations.dart`
2. Add new method:

```dart
Future<TransactionResult> myOperation({
  required KanariWallet wallet,
  required String param,
}) async {
  // Get sequence number
  final account = await queries.getAccount(wallet.address);
  
  // Prepare transaction data and params
  // ...
  
  return _signAndSubmit(
    wallet: wallet,
    txData: txData,
    rpcMethod: TransactionConstants.rpcMyMethod,
    params: params,
  );
}
```

1. Add wrapper in `kanari_client.dart`

## 📊 Benefits of This Architecture

### ✅ Maintainability

- Each module has clear responsibility
- Can fix only the relevant part without affecting others
- Can test each module separately

### ✅ Extensibility

- Easy to add new features by adding to the relevant module
- Can create custom modules (e.g., escrow, swap, nft)
- Template modules are ready to use, reducing development time

### ✅ Testability

- Can mock each module separately
- Easier unit testing
- Integration testing separated by module

### ✅ Code Organization

- Files are smaller (from 600+ lines → ~200 lines per file)
- Easier to find code
- Clearer flow understanding

### ✅ Developer Experience

- Clear separation between read/write operations
- Consistent patterns across all modules
- Comprehensive documentation and examples
- Ready-to-use templates for common features

## ⚠️ Caution

1. **Do not edit Core unless necessary**: `core/` is the shared layer used by all modules
2. **Maintain Backward Compatibility**: Do not delete or change the signature of public methods in `KanariClient`
3. **Use Constants**: Values used frequently should be stored in `constants.dart`
4. **Error Handling**: Every method should have appropriate error handling
5. **Document Changes**: When adding a new module, update ARCHITECTURE.md

## 📋 Module Status

| Module | Status | Description |
|--------|--------|-------------|
| transactions | ✅ Complete | Basic transactions (transfer, execute, publish, burn) |
| queries | ✅ Complete | All read operations (account, balance, blocks, etc.) |
| escrow | ✅ Complete | Escrow/deal management system |
| tokens | 📝 Template | Fungible tokens (create, mint, burn, transfer) |
| nft | 📝 Template | NFT collections and minting |
| defi | 📝 Template | DEX swaps, liquidity pools |
| governance | 🔜 Planned | DAO voting and proposals |
| staking | 🔜 Planned | Token staking and rewards |
| identity | 🔜 Planned | DID and verification |
| oracle | 🔜 Planned | Price feeds and data |

## 🔗 Related Files

- [Module Development Guide](./modules/MODULE_DEVELOPMENT_GUIDE.md) - Guide for developing new modules
- [Escrow Client Pattern](../../example_move/kanari_kit/lib/src/escrow_client.dart)
- [Escrow Module Structure](../../example_move/kanari_kit/lib/src/modules/escrow/)
- [Core Utilities](./core/)
- [Transaction Operations](./modules/transactions/)
- [Query Operations](./modules/queries.dart)
- [Template Modules](./modules/) - Tokens, NFT, DeFi templates

## 📖 Additional Resources

- [Kanari System Package Documentation](../../../crates/kanari-frameworks/packages/kanari-system/docs/)
- [Move Language Guide](https://move-language.github.io/move/)
- [BCS Serialization Spec](https://github.com/diem/bcs)

---

**Last Updated**: 2026-05-12
**Version**: 2.0.0 (Modular Architecture)
