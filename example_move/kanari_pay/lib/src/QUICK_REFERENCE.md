# Quick Reference Guide

Quick reference guide for Kanari SDK Modular Architecture

## 📋 Table of Contents

- [Import Statements](#import-statements)
- [Common Operations](#common-operations)
- [Module Templates](#module-templates)
- [Error Handling](#error-handling)
- [Testing](#testing)

---

## Import Statements

### Basic Usage

```dart
import 'package:kanari_kit/kanari_kit.dart';
// or
import 'package:kanari_kit/src/kanari_client.dart';
```

### Advanced Module Access

```dart
import 'package:kanari_kit/src/modules/modules.dart';
import 'package:kanari_kit/src/core/core.dart';
```

### Specific Modules

```dart
// Transactions
import 'package:kanari_kit/src/modules/transactions/transactions.dart';

// Queries
import 'package:kanari_kit/src/modules/queries.dart';

// Template modules (when implemented)
import 'package:kanari_kit/src/modules/tokens/tokens.dart';
import 'package:kanari_kit/src/modules/nft/nft.dart';
import 'package:kanari_kit/src/modules/defi/defi.dart';
```

---

## Common Operations

### Initialize Client

```dart
// From URL
final client = KanariClient('http://localhost:3000');

// From Environment
final client = KanariClient.fromEnvironment(KanariEnvironment.testnet);

// With custom HTTP client
final client = KanariClient(
  'http://localhost:3000',
  client: customHttpClient,
);
```

### Query Operations (Read)

```dart
// Get account info
final account = await client.getAccount(address);

// Get balance
final balance = await client.getBalance(address);

// Get token balance
final tokenBalance = await client.getTokenBalance(address, tokenType);

// Get all balances
final balances = await client.getAllBalances(address);

// Get block
final block = await client.getBlock(height);

// Get transaction
final tx = await client.getTransaction(hash);

// Get stats
final stats = await client.getStats();

// Get health
final health = await client.getHealth();
```

### Transaction Operations (Write)

```dart
// Transfer KANARI
await client.transfer(
  wallet: wallet,
  recipient: recipientAddress,
  amount: 1000,
  gasLimit: 100000,
  gasPrice: 1000,
);

// Execute Move function
await client.executeFunction(
  wallet: wallet,
  package: packageAddress,
  module: 'module_name',
  function: 'function_name',
  typeArgs: ['0x...::token::TOKEN'],
  args: [arg1, arg2],
  gasLimit: 100000,
  gasPrice: 0,
);

// Publish module
await client.publishModule(
  wallet: wallet,
  moduleBytes: moduleBytes,
  moduleName: 'my_module',
  gasLimit: 100000,
  gasPrice: 1000,
);

// Burn tokens
await client.burn(
  wallet: wallet,
  amount: 100,
);

// Transfer custom token
await client.transferToken(
  wallet: wallet,
  recipient: recipientAddress,
  tokenType: '0x...::james::JAMES',
  amount: 500,
);
```

### Direct Module Access

```dart
// Access queries directly
final queries = client.queries;
final account = await queries.getAccount(address);

// Access transactions directly
final txOps = client.transactions;
await txOps.transfer(
  wallet: wallet,
  recipient: address,
  amount: 1000,
);
```

---

## Module Templates

### Creating a New Module

#### Step 1: Create Structure

```bash
mkdir lib/src/modules/my_feature
cd lib/src/modules/my_feature
touch constants.dart operations.dart queries.dart my_feature.dart
```

#### Step 2: Constants Template

```dart
class MyFeatureConstants {
  const MyFeatureConstants._();
  
  static const String packageAddress = '0x...';
  static const String module = 'my_module';
  
  // Entry functions
  static const String fnCreate = 'create';
  static const String fnUpdate = 'update';
  
  // View functions
  static const String fnGetInfo = 'get_info';
}
```

#### Step 3: Operations Template

```dart
import '../../core/bcs_serializers.dart';
import '../../kanari_wallet.dart';
import '../../models/transaction.dart';
import '../queries.dart';
import 'constants.dart';

class MyFeatureOperations {
  final String url;
  final QueriesModule queries;
  final http.Client client;

  MyFeatureOperations(this.url, this.queries, this.client);

  Future<TransactionResult> create({
    required KanariWallet wallet,
    required String param,
    int gasLimit = 100000,
    int gasPrice = 1000,
  }) async {
    return await queries.executeFunction(
      wallet: wallet,
      package: MyFeatureConstants.packageAddress,
      module: MyFeatureConstants.module,
      function: MyFeatureConstants.fnCreate,
      args: [BcsSerializers.hexToBytes(param)],
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }
}
```

#### Step 4: Queries Template

```dart
import '../queries.dart';
import 'constants.dart';

class MyFeatureQueries {
  final String url;
  final QueriesModule baseQueries;
  final http.Client client;

  MyFeatureQueries(this.url, this.baseQueries, this.client);

  Future<Map<String, dynamic>> getInfo(String id) async {
    // Implementation here
    throw UnimplementedError();
  }
}
```

#### Step 5: Barrel File

```dart
export 'constants.dart';
export 'operations.dart';
export 'queries.dart';
```

#### Step 6: Export in modules.dart

```dart
export 'my_feature/my_feature.dart';
```

---

## Error Handling

### Basic Error Handling

```dart
try {
  final result = await client.transfer(
    wallet: wallet,
    recipient: address,
    amount: 1000,
  );
  
  if (result.status.toLowerCase() == 'success') {
    print('Transaction successful: ${result.hash}');
  } else {
    print('Transaction failed: ${result.errorMessage}');
  }
} catch (e) {
  print('Error: $e');
}
```

### Validation Before Transaction

```dart
// Check balance before transfer
final balance = await client.getBalance(wallet.address);
if (balance < amount + gasCost) {
  throw Exception('Insufficient balance');
}

// Validate addresses
try {
  BcsSerializers.normalizeAddress(recipientAddress);
} catch (e) {
  throw ArgumentError('Invalid recipient address');
}
```

### Custom Error Messages

```dart
Future<TransactionResult> safeTransfer({
  required KanariWallet wallet,
  required String recipient,
  required int amount,
}) async {
  // Validate recipient
  if (!recipient.startsWith('0x')) {
    throw ArgumentError('Recipient must be hex address');
  }
  
  // Validate amount
  if (amount <= 0) {
    throw ArgumentError('Amount must be positive');
  }
  
  // Check balance
  final balance = await getBalance(wallet.address);
  if (balance < amount) {
    throw Exception('Insufficient balance: have $balance, need $amount');
  }
  
  // Execute transfer
  return await transfer(
    wallet: wallet,
    recipient: recipient,
    amount: amount,
  );
}
```

---

## Testing

### Unit Test Template

```dart
import 'package:test/test.dart';
import 'package:mockito/mockito.dart';
import 'package:kanari_kit/src/modules/my_feature/my_feature.dart';

void main() {
  late MyFeatureOperations operations;
  late MockQueriesModule mockQueries;
  
  setUp(() {
    mockQueries = MockQueriesModule();
    operations = MyFeatureOperations(
      'http://test',
      mockQueries,
      MockHttpClient(),
    );
  });
  
  test('should create item successfully', () async {
    // Arrange
    when(mockQueries.getAccount(any)).thenAnswer(
      (_) async => AccountInfo(sequenceNumber: 0, ...)
    );
    
    // Act
    final result = await operations.create(
      wallet: testWallet,
      param: 'value',
    );
    
    // Assert
    expect(result.status, 'success');
  });
}
```

### Integration Test Template

```dart
import 'package:test/test.dart';
import 'package:kanari_kit/kanari_kit.dart';

void main() {
  test('Full workflow', () async {
    final client = KanariClient('http://localhost:3000');
    final wallet = await KanariWallet.create();
    
    // Fund wallet (via faucet or transfer)
    
    // Test query
    final account = await client.getAccount(wallet.address);
    expect(account, isNotNull);
    
    // Test transaction
    final result = await client.transfer(
      wallet: wallet,
      recipient: anotherAddress,
      amount: 100,
    );
    expect(result.status, 'success');
  });
}
```

---

## Utility Functions

### BCS Serialization

```dart
import 'package:kanari_kit/src/core/core.dart';

// Hex to bytes
final bytes = BcsSerializers.hexToBytes('0xabc123');

// Encode u64
final encoded = BcsSerializers.encodeU64(1000);

// Normalize address
final normalized = BcsSerializers.normalizeAddress('0x123');
// Returns: '0x0000000000000000000000000000000000000000000000000000000000000123'

// Extract coin type
final coinType = BcsSerializers.extractCoinTypeFromObjectType(
  '0x2::coin::Coin<0x2::usdc::USDC>'
);
// Returns: '0x2::usdc::USDC'
```

### RPC Utilities

```dart
import 'package:kanari_kit/src/core/core.dart';

// Generic RPC request
final response = await RpcUtils.request(
  httpClient,
  url,
  'kanari_customMethod',
  params,
  (json) => CustomType.fromJson(json),
);

// Execute view function
final result = await RpcUtils.executeViewFunction(
  httpClient,
  url,
  packageAddress,
  moduleName,
  functionName,
  typeArgs,
  arguments,
);
```

---

## Gas Settings

### Default Gas Values

```dart
// Standard transactions
const defaultGasLimit = 100000;
const defaultGasPrice = 1000;

// Complex operations (may need more gas)
const complexGasLimit = 500000;
const complexGasPrice = 2000;

// View functions (no gas needed)
const viewFunctionGasPrice = 0;
```

### Estimating Gas

```dart
// Start with default
int gasLimit = 100000;

// If transaction fails with out of gas, increase
try {
  await client.executeFunction(..., gasLimit: gasLimit);
} catch (e) {
  if (e.toString().contains('out of gas')) {
    gasLimit *= 2; // Double the gas limit
    // Retry with higher gas
  }
}
```

---

## Best Practices Checklist

- [ ] Use constants for addresses and function names
- [ ] Always normalize addresses before use
- [ ] Handle errors appropriately
- [ ] Validate inputs before transactions
- [ ] Use appropriate gas limits
- [ ] Document public methods
- [ ] Write unit tests for new features
- [ ] Follow naming conventions
- [ ] Keep modules focused (single responsibility)
- [ ] Use barrel files for clean exports

---

## Troubleshooting

### Common Issues

**Issue**: "Address must be exactly 64 hex characters"

```dart
// Solution: Use normalizeAddress
final normalized = BcsSerializers.normalizeAddress(shortAddress);
```

**Issue**: "flutter_rust_bridge has not been initialized"

```dart
// Solution: Ensure Rust bridge is initialized before signing
await initializeRustBridge();
```

**Issue**: "Insufficient gas"

```dart
// Solution: Increase gas limit
await client.transfer(..., gasLimit: 200000);
```

**Issue**: "Invalid signature"

```dart
// Solution: Ensure using tagged address
final sender = wallet.taggedAddress; // Not wallet.address
```

---

## Resources

- [Architecture Overview](./ARCHITECTURE.md)
- [Module Development Guide](./modules/MODULE_DEVELOPMENT_GUIDE.md)
- [Refactoring Summary](./REFACTORING_SUMMARY.md)
- [Architecture Diagrams](./ARCHITECTURE_DIAGRAM.md)

---

**Quick Reference Version**: 1.0.0  
**Last Updated**: 2026-05-12
