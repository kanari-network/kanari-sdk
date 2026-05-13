// modules/dex/operations.dart
/// DEX transaction operations

import 'package:http/http.dart' as http;

import '../../kanari_wallet.dart';
import '../../models/transaction.dart';
import '../../core/bcs_serializers.dart';

import '../../client/kanari_client.dart';
import 'constants.dart';

/// DEX Operations - handles all write operations
class DexOperations {
  final String url;
  final http.Client client;

  DexOperations(this.url, {http.Client? httpClient})
    : client = httpClient ?? http.Client();

  /// Create a new DEX pool
  Future<TransactionResult> createPool({
    required KanariWallet wallet,
    required String coinTypeA,
    required String coinTypeB,
    int feePercent = 30,
    int gasLimit = 100000,
    int gasPrice = 1000,
  }) async {
    print(
      '[DEX] Creating pool: $coinTypeA / $coinTypeB with fee $feePercent bps',
    );

    try {
      final kanariClient = KanariClient(url, client: client);

      // Encode fee_percent as u64
      final feeBytes = BcsSerializers.encodeU64(feePercent);

      final result = await kanariClient.executeFunction(
        wallet: wallet,
        package: DexConstants.packageAddress,
        module: DexConstants.dexModule,
        function: DexConstants.fnCreatePool,
        typeArgs: [coinTypeA, coinTypeB],
        args: [feeBytes],
        gasLimit: gasLimit,
        gasPrice: gasPrice,
      );

      print('[DEX] Pool created successfully: ${result.hash}');
      return result;
    } catch (e) {
      print('[DEX] Error creating pool: $e');
      rethrow;
    }
  }

  /// Add liquidity to a pool
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
    print('[DEX] Adding liquidity: $amountA $coinTypeA, $amountB $coinTypeB');
    print('[DEX] Pool: $poolObjectId (${poolObjectId.length} chars)');
    print('[DEX] Coin A: $coinAObjectId (${coinAObjectId.length} chars)');
    print('[DEX] Coin B: $coinBObjectId (${coinBObjectId.length} chars)');

    try {
      final kanariClient = KanariClient(url, client: client);

      // Normalize and encode all arguments according to Move contract signature:
      // pool_id: address, coin_a_id: address, coin_b_id: address, amount_a: u64, amount_b: u64
      final normalizedPoolId = BcsSerializers.normalizeObjectId(poolObjectId);
      final normalizedCoinAId = BcsSerializers.normalizeObjectId(coinAObjectId);
      final normalizedCoinBId = BcsSerializers.normalizeObjectId(coinBObjectId);

      print('[DEX] Normalized Pool ID: $normalizedPoolId');
      print('[DEX] Normalized Coin A ID: $normalizedCoinAId');
      print('[DEX] Normalized Coin B ID: $normalizedCoinBId');

      final poolIdBytes = BcsSerializers.hexToBytes(normalizedPoolId);
      final coinAIdBytes = BcsSerializers.hexToBytes(normalizedCoinAId);
      final coinBIdBytes = BcsSerializers.hexToBytes(normalizedCoinBId);
      final amountABytes = BcsSerializers.encodeU64(amountA);
      final amountBBytes = BcsSerializers.encodeU64(amountB);

      print('[DEX] Encoded arguments:');
      print('  - Pool ID: ${poolIdBytes.length} bytes');
      print('  - Coin A ID: ${coinAIdBytes.length} bytes');
      print('  - Coin B ID: ${coinBIdBytes.length} bytes');
      print('  - Amount A: $amountA (${amountABytes.length} bytes)');
      print('  - Amount B: $amountB (${amountBBytes.length} bytes)');

      final result = await kanariClient.executeFunction(
        wallet: wallet,
        package: DexConstants.packageAddress,
        module: DexConstants.dexModule,
        function: DexConstants.fnAddLiquidity,
        typeArgs: [coinTypeA, coinTypeB],
        args: [
          poolIdBytes,
          coinAIdBytes,
          coinBIdBytes,
          amountABytes,
          amountBBytes,
        ],
        gasLimit: gasLimit,
        gasPrice: gasPrice,
      );

      print('[DEX] Liquidity added successfully: ${result.hash}');
      return result;
    } catch (e) {
      print('[DEX] Error adding liquidity: $e');
      print('[DEX] Stack trace: ${StackTrace.current}');
      rethrow;
    }
  }

  /// Remove liquidity from a pool
  Future<TransactionResult> removeLiquidity({
    required KanariWallet wallet,
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
    required String lpCoinObjectId,
    int gasLimit = 100000,
    int gasPrice = 1000,
  }) async {
    print('[DEX] Removing liquidity from pool: $poolObjectId');
    print('[DEX] LP Coin: $lpCoinObjectId');

    try {
      final kanariClient = KanariClient(url, client: client);

      // Normalize and encode arguments according to Move contract:
      // pool_id: address, lp_coin_id: address
      final normalizedPoolId = BcsSerializers.normalizeObjectId(poolObjectId);
      final normalizedLpCoinId = BcsSerializers.normalizeObjectId(
        lpCoinObjectId,
      );

      final poolIdBytes = BcsSerializers.hexToBytes(normalizedPoolId);
      final lpCoinIdBytes = BcsSerializers.hexToBytes(normalizedLpCoinId);

      final result = await kanariClient.executeFunction(
        wallet: wallet,
        package: DexConstants.packageAddress,
        module: DexConstants.dexModule,
        function: DexConstants.fnRemoveLiquidity,
        typeArgs: [coinTypeA, coinTypeB],
        args: [poolIdBytes, lpCoinIdBytes],
        gasLimit: gasLimit,
        gasPrice: gasPrice,
      );

      print('[DEX] Liquidity removed successfully: ${result.hash}');
      return result;
    } catch (e) {
      print('[DEX] Error removing liquidity: $e');
      rethrow;
    }
  }

  /// Swap coin A for coin B
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
    print('[DEX] Swapping $amountIn $coinTypeA for $coinTypeB');
    print('[DEX] Pool: $poolObjectId');
    print('[DEX] Coin In: $coinInObjectId');

    try {
      final kanariClient = KanariClient(url, client: client);

      // Normalize and encode arguments according to Move contract:
      // pool_id: address, coin_in_id: address, amount_in: u64
      final normalizedPoolId = BcsSerializers.normalizeObjectId(poolObjectId);
      final normalizedCoinInId = BcsSerializers.normalizeObjectId(
        coinInObjectId,
      );

      final poolIdBytes = BcsSerializers.hexToBytes(normalizedPoolId);
      final coinInIdBytes = BcsSerializers.hexToBytes(normalizedCoinInId);
      final amountInBytes = BcsSerializers.encodeU64(amountIn);

      final result = await kanariClient.executeFunction(
        wallet: wallet,
        package: DexConstants.packageAddress,
        module: DexConstants.dexModule,
        function: DexConstants.fnSwapAForB,
        typeArgs: [coinTypeA, coinTypeB],
        args: [poolIdBytes, coinInIdBytes, amountInBytes],
        gasLimit: gasLimit,
        gasPrice: gasPrice,
      );

      print('[DEX] Swap successful: ${result.hash}');
      return result;
    } catch (e) {
      print('[DEX] Error swapping: $e');
      rethrow;
    }
  }

  /// Swap coin B for coin A
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
    print('[DEX] Swapping $amountIn $coinTypeB for $coinTypeA');
    print('[DEX] Pool: $poolObjectId');
    print('[DEX] Coin In: $coinInObjectId');

    try {
      final kanariClient = KanariClient(url, client: client);

      // Normalize and encode arguments according to Move contract:
      // pool_id: address, coin_in_id: address, amount_in: u64
      final normalizedPoolId = BcsSerializers.normalizeObjectId(poolObjectId);
      final normalizedCoinInId = BcsSerializers.normalizeObjectId(
        coinInObjectId,
      );

      final poolIdBytes = BcsSerializers.hexToBytes(normalizedPoolId);
      final coinInIdBytes = BcsSerializers.hexToBytes(normalizedCoinInId);
      final amountInBytes = BcsSerializers.encodeU64(amountIn);

      final result = await kanariClient.executeFunction(
        wallet: wallet,
        package: DexConstants.packageAddress,
        module: DexConstants.dexModule,
        function: DexConstants.fnSwapBForA,
        typeArgs: [coinTypeA, coinTypeB],
        args: [poolIdBytes, coinInIdBytes, amountInBytes],
        gasLimit: gasLimit,
        gasPrice: gasPrice,
      );

      print('[DEX] Swap successful: ${result.hash}');
      return result;
    } catch (e) {
      print('[DEX] Error swapping: $e');
      rethrow;
    }
  }
}
