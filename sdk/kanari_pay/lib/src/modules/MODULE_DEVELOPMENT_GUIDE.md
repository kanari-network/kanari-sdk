# Module Development Guide

Module development guide for Kanari SDK - How to add new features to the system

## 📋 Table of Contents

1. [Module Structure](#module-structure)
2. [Steps to Create a New Module](#steps-to-create-a-new-module)
3. [Example: Creating Token Module](#example-creating-token-module)
4. [Best Practices](#best-practices)
5. [Testing](#testing)

---

## Module Structure

Each module should have the following structure:

``md
modules/[module_name]/
├── constants.dart      # Constants (addresses, function names, etc.)
├── operations.dart     # Transaction operations (write)
├── queries.dart        # Query operations (read)
└── [module_name].dart  # Barrel file (exports)

```

### What is the purpose of each file?

#### 1. `constants.dart`

Store all module constants:

- Package addresses
- Module names
- Function names (entry functions & view functions)
- Object types
- State constants (if any)

```dart
class MyModuleConstants {
  const MyModuleConstants._();
  
  static const String packageAddress = '0x...';
  static const String module = 'my_module';
  
  // Entry functions
  static const String fnCreate = 'create';
  static const String fnUpdate = 'update';
  
  // View functions
  static const String fnGetInfo = 'get_info';
  
  // Object types
  static const String objectTypeItem = 'MyObject';
}
```

#### 2. `operations.dart`

Manage transaction operations (write to blockchain):

```dart
class MyModuleOperations {
  final String url;
  final QueriesModule queries;
  final http.Client client;

  MyModuleOperations(this.url, this.queries, this.client);

  Future<TransactionResult> myOperation({
    required KanariWallet wallet,
    required String param1,
    int gasLimit = 100000,
    int gasPrice = 1000,
  }) async {
    // 1. Get owner-centric state / sequence number
    final owner = await queries.getOwner(wallet.address);
    
    // 2. Prepare arguments (BCS encode if needed)
    final args = [BcsSerializers.hexToBytes(param1)];
    
    // 3. Execute function
    return queries.executeFunction(
      wallet: wallet,
      package: MyModuleConstants.packageAddress,
      module: MyModuleConstants.module,
      function: MyModuleConstants.fnCreate,
      typeArgs: [],
      args: args,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }
}
```

#### 3. `queries.dart`

Manage query operations (read from blockchain):

```dart
class MyModuleQueries {
  final String url;
  final QueriesModule baseQueries;
  final http.Client client;

  MyModuleQueries(this.url, this.baseQueries, this.client);

  Future<Map<String, dynamic>> getInfo(String objectId) async {
    // Use view function or RPC call
    final result = await RpcUtils.executeViewFunction(
      client,
      url,
      MyModuleConstants.packageAddress,
      MyModuleConstants.module,
      MyModuleConstants.fnGetInfo,
      [],
      [BcsSerializers.hexToBytes(objectId)],
    );
    
    // Parse and return result
    return {
      'field1': result[0],
      'field2': result[1],
    };
  }
}
```

#### 4. `[module_name].dart` (Barrel File)

Export everything in the module:

```dart
export 'constants.dart';
export 'operations.dart';
export 'queries.dart';
```

---

## Steps to Create a New Module

### Step 1: Create Folder and Files

```bash
mkdir lib/src/modules/my_feature
cd lib/src/modules/my_feature
touch constants.dart operations.dart queries.dart my_feature.dart
```

### Step 2: Define Constants

Open `constants.dart` and define constants:

```dart
class MyFeatureConstants {
  const MyFeatureConstants._();
  
  static const String packageAddress = '0x...';
  static const String module = 'my_feature';
  
  // Add your constants here
}
```

### Step 3: Implement Operations

Open `operations.dart` and create transaction functions:

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

  // Add your operations here
}
```

### Step 4: Implement Queries

Open `queries.dart` and create query functions:

```dart
import '../queries.dart';
import 'constants.dart';

class MyFeatureQueries {
  final String url;
  final QueriesModule baseQueries;
  final http.Client client;

  MyFeatureQueries(this.url, this.baseQueries, this.client);

  // Add your queries here
}
```

### Step 5: Create Barrel File

Open `my_feature.dart`:

```dart
export 'constants.dart';
export 'operations.dart';
export 'queries.dart';
```

### Step 6: Export in modules.dart

Open `modules/modules.dart` and add:

```dart
export 'my_feature/my_feature.dart';
```

### Step 7: Add Wrapper in KanariClient (Optional)

If you want to use it directly through `KanariClient`:

```dart
// In kanari_client.dart

late final MyFeatureOperations _myFeatureOps;
late final MyFeatureQueries _myFeatureQueries;

KanariClient(this.url, {http.Client? client})
  : _client = client ?? http.Client() {
  // ... existing initialization ...
  _myFeatureOps = MyFeatureOperations(url, _queries, _client);
  _myFeatureQueries = MyFeatureQueries(url, _queries, _client);
}

// Add wrapper methods
Future<TransactionResult> myFeatureOperation({...}) {
  return _myFeatureOps.myOperation(...);
}

Future<MyType> myFeatureQuery(...) {
  return _myFeatureQueries.getInfo(...);
}
```

---

## Example: Creating Token Module

See existing templates at:

- [`modules/tokens/constants.dart`](./tokens/constants.dart)
- [`modules/tokens/operations.dart`](./tokens/operations.dart)
- [`modules/tokens/queries.dart`](./tokens/queries.dart)

### Actual Implementation

When ready to implement:

1. **Remove `throw UnimplementedError()`** from every method
2. **Write actual logic** using the same pattern as escrow module
3. **Test** with unit tests

Example implementation of `createCurrency`:

```dart
Future<TransactionResult> createCurrency({
  required KanariWallet wallet,
  required String name,
  required String symbol,
  required int decimals,
  int gasLimit = 100000,
  int gasPrice = 1000,
}) async {
  // Encode arguments
  final nameBytes = BcsSerializers.hexToBytes(name);
  final symbolBytes = BcsSerializers.hexToBytes(symbol);
  final decimalsBytes = BcsSerializers.encodeU64(decimals);

  // Execute function
  return await queries.executeFunction(
    wallet: wallet,
    package: TokenConstants.packageAddress,
    module: TokenConstants.coinModule,
    function: TokenConstants.fnCreateCurrency,
    typeArgs: [],
    args: [nameBytes, symbolBytes, decimalsBytes],
    gasLimit: gasLimit,
    gasPrice: gasPrice,
  );
}
```

---

## Best Practices

### ✅ Do

1. **Use Constants Always**: Don't hardcode addresses or function names
2. **Separate Operations and Queries Clearly**: Write vs Read
3. **Document Every Method**: Use doc comments (`///`)
4. **Handle Errors**: Check errors from RPC responses
5. **Use Type Safety**: Use strong typing as much as possible
6. **Follow Naming Convention**:
   - Operations: `createXxx`, `updateXxx`, `deleteXxx`
   - Queries: `getXxx`, `listXxx`, `hasXxx`

### ❌ Don't

1. **Don't Put Business Logic in Client Layer**: Keep it in modules
2. **Don't Hardcode Values**: Use constants always
3. **Don't Forget Error Handling**: Every method should have error handling
4. **Don't Duplicate Code**: Use shared utilities from `core/`

---

## Testing

### Unit Test Template

``dart
import 'package:test/test.dart';
import 'package:kanari_kit/src/modules/my_feature/my_feature.dart';

void main() {
  group('MyFeatureOperations', () {
    test('should create item successfully', () async {
      // TODO: Implement test
    });

    test('should fail with invalid parameters', () async {
      // TODO: Implement test
    });
  });

  group('MyFeatureQueries', () {
    test('should get item info', () async {
      // TODO: Implement test
    });
  });
}

```

### Integration Test

``dart
import 'package:test/test.dart';
import 'package:kanari_kit/kanari_kit.dart';

void main() {
  test('Full workflow test', () async {
    final client = KanariClient('http://localhost:3000');
    final wallet = await KanariWallet.create();
    
    // Test operations
    final result = await client.myFeatureOperation(
      wallet: wallet,
      param: 'value',
    );
    
    expect(result.status, 'success');
    
    // Test queries
    final info = await client.myFeatureQuery('object_id');
    expect(info, isNotNull);
  });
}
```

---

## 📚 Resources

- [Escrow Module Example](../escrow/) - Complete module example
- [Architecture Overview](../ARCHITECTURE.md) - Architecture overview
- [Core Utilities](../core/) - Shared utility functions

---

## 🆘 Need Help?

If you have any issues or questions:

1. Check escrow module as an example
2. Read ARCHITECTURE.md to understand the overview
3. Verify imports are correct
4. Run `dart analyze` to find errors

Happy coding! 🚀
