import 'package:http/http.dart' as http;
import 'package:flutter/foundation.dart';

import '../kanari_wallet.dart';
import '../models/transaction.dart';
import '../modules/dex/dex.dart';
import '../modules/queries.dart';

/// DexClient - facade that uses internal modules
class DexClient {
  final String url;
  final http.Client client;
  late final DexOperations _operations;
  late final DexQueries _queries;
  late final QueriesModule _baseQueries;

  DexClient(this.url, {http.Client? httpClient})
    : client = httpClient ?? http.Client() {
    _baseQueries = QueriesModule(url, client);
    _operations = DexOperations(url, httpClient: client);
    _queries = DexQueries(url, _baseQueries, client);
  }

  // ==================== OPERATIONS ====================

  /// Create pool wrapper
  Future<TransactionResult> createPool({
    required KanariWallet wallet,
    required String coinTypeA,
    required String coinTypeB,
    int feePercent = 30,
    int gasLimit = 100000,
    int gasPrice = 1000,
  }) async {
    return _operations.createPool(
      wallet: wallet,
      coinTypeA: coinTypeA,
      coinTypeB: coinTypeB,
      feePercent: feePercent,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  /// Add liquidity wrapper
  Future<TransactionResult> addLiquidity({
    required KanariWallet wallet,
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
    required String coinAObjectId,
    required String coinBObjectId,
    required int amountA,
    required int amountB,
    int gasLimit = 100000,
    int gasPrice = 1000,
  }) async {
    return _operations.addLiquidity(
      wallet: wallet,
      poolObjectId: poolObjectId,
      coinTypeA: coinTypeA,
      coinTypeB: coinTypeB,
      coinAObjectId: coinAObjectId,
      coinBObjectId: coinBObjectId,
      amountA: amountA,
      amountB: amountB,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  /// Remove liquidity wrapper
  Future<TransactionResult> removeLiquidity({
    required KanariWallet wallet,
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
    required String lpCoinObjectId,
    int gasLimit = 100000,
    int gasPrice = 1000,
  }) async {
    return _operations.removeLiquidity(
      wallet: wallet,
      poolObjectId: poolObjectId,
      coinTypeA: coinTypeA,
      coinTypeB: coinTypeB,
      lpCoinObjectId: lpCoinObjectId,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  /// Swap A for B wrapper
  Future<TransactionResult> swapAForB({
    required KanariWallet wallet,
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
    required String coinInObjectId,
    required int amountIn,
    int gasLimit = 100000,
    int gasPrice = 1000,
  }) async {
    return _operations.swapAForB(
      wallet: wallet,
      poolObjectId: poolObjectId,
      coinTypeA: coinTypeA,
      coinTypeB: coinTypeB,
      coinInObjectId: coinInObjectId,
      amountIn: amountIn,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  /// Swap B for A wrapper
  Future<TransactionResult> swapBForA({
    required KanariWallet wallet,
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
    required String coinInObjectId,
    required int amountIn,
    int gasLimit = 100000,
    int gasPrice = 1000,
  }) async {
    return _operations.swapBForA(
      wallet: wallet,
      poolObjectId: poolObjectId,
      coinTypeA: coinTypeA,
      coinTypeB: coinTypeB,
      coinInObjectId: coinInObjectId,
      amountIn: amountIn,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  // ==================== QUERIES ====================

  /// Get all objects owned by address
  Future<List<Map<String, dynamic>>> getOwnedObjects(String address) async {
    return _baseQueries.getOwnedObjects(address);
  }

  /// Get all user pools with proper parsing
  Future<List<Map<String, dynamic>>> getUserPools(String address) async {
    final objects = await getOwnedObjects(address);
    debugPrint('[DEX Client] Total objects found: ${objects.length}');
    final pools = <Map<String, dynamic>>[];

    for (final obj in objects) {
      final objType = obj['type'] as String? ?? '';

      // Debug: Print all object types to see what we have
      debugPrint('[DEX Client] Object type: $objType');

      // Match pattern: package::dex_v1::Pool<CoinTypeA, CoinTypeB>
      if (objType.contains('Pool<') && objType.contains('dex_v1')) {
        try {
          debugPrint('[DEX Client] Found pool object! Type: $objType');

          // Extract coin types from the type string
          // Format: 0x...::dex_v1::Pool<0x...::module::TokenA, 0x...::module::TokenB>
          final typeMatch = RegExp(
            r'Pool<([^,]+),\s*([^>]+)>',
          ).firstMatch(objType);

          if (typeMatch != null) {
            final coinTypeA = typeMatch.group(1)?.trim() ?? '';
            final coinTypeB = typeMatch.group(2)?.trim() ?? '';

            // Get pool ID from object
            final poolId =
                obj['id'] as String? ?? obj['objectId'] as String? ?? '';

            debugPrint('[DEX Client] Pool ID: $poolId');
            debugPrint('[DEX Client] Coin A: $coinTypeA');
            debugPrint('[DEX Client] Coin B: $coinTypeB');

            pools.add({
              'id': poolId,
              'objectId': poolId,
              'pool_id': poolId,
              'coin_type_a': coinTypeA,
              'coin_type_b': coinTypeB,
              'type': objType,
            });
          }
        } catch (e) {
          debugPrint('[DEX Client] Failed to parse pool type: $e');
        }
      }
    }

    debugPrint('[DEX Client] Total pools found: ${pools.length}');
    return pools;
  }

  /// Get all tokens owned by address
  Future<List<String>> getUserTokens(String address) async {
    final objects = await getOwnedObjects(address);
    final tokenTypes = <String>{};

    for (final obj in objects) {
      final objType = obj['type'] as String? ?? '';

      // Match pattern: 0x2::coin::Coin<...>
      if (objType.startsWith('0x2::coin::Coin<') && objType.endsWith('>')) {
        final tokenType = objType.substring(16, objType.length - 1);
        tokenTypes.add(tokenType);
      }
    }

    return tokenTypes.toList();
  }

  /// Get pool info by calling view function
  Future<Map<String, dynamic>> getPoolInfo({
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
  }) async {
    return _queries.getPoolInfo(
      poolObjectId: poolObjectId,
      coinTypeA: coinTypeA,
      coinTypeB: coinTypeB,
    );
  }

  /// Calculate expected output for swap A -> B
  Future<int> calculateSwapAForBOutput({
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
    required int amountIn,
  }) async {
    return _queries.calculateSwapAForBOutput(
      poolObjectId: poolObjectId,
      coinTypeA: coinTypeA,
      coinTypeB: coinTypeB,
      amountIn: amountIn,
    );
  }

  /// Calculate expected output for swap B -> A
  Future<int> calculateSwapBForAOutput({
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
    required int amountIn,
  }) async {
    return _queries.calculateSwapBForAOutput(
      poolObjectId: poolObjectId,
      coinTypeA: coinTypeA,
      coinTypeB: coinTypeB,
      amountIn: amountIn,
    );
  }
}
