// modules/dex/queries.dart
// DEX query operations

import 'package:http/http.dart' as http;

import '../../core/bcs_serializers.dart';
import '../../core/rpc_utils.dart';
import '../queries.dart';
import 'constants.dart';

class DexQueries {
  final String url;
  final QueriesModule baseQueries;
  final http.Client client;

  DexQueries(this.url, this.baseQueries, this.client);

  /// Get pool information by querying the pool object directly
  Future<Map<String, dynamic>> getPoolInfo({
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
  }) async {
    try {
      print('[DEX] Getting pool info for: $poolObjectId');

      // Normalize pool ID to ensure proper format
      final normalizedPoolId = BcsSerializers.normalizeObjectId(poolObjectId);

      // Encode pool_id as address (32 bytes)
      final poolIdBytes = BcsSerializers.hexToBytes(normalizedPoolId);

      // Call get_pool_info view function which returns (u64, u64, u64, u64)
      // Returns: (reserve_a, reserve_b, lp_supply, fee_percent)
      final result = await RpcUtils.executeViewFunction(
        client,
        url,
        DexConstants.packageAddress,
        DexConstants.dexModule,
        DexConstants.fnGetPoolInfo,
        [coinTypeA, coinTypeB],
        [poolIdBytes],
      );

      print('[DEX] Pool info result: $result');

      // Parse the tuple result: (reserve_a, reserve_b, lp_supply, fee_percent)
      if (result.length >= 4) {
        return {
          'pool_id': normalizedPoolId,
          'coin_type_a': coinTypeA,
          'coin_type_b': coinTypeB,
          'reserve_a': result[0] as int,
          'reserve_b': result[1] as int,
          'lp_supply': result[2] as int,
          'fee_percent': result[3] as int,
        };
      }

      throw Exception(
        'Invalid pool info response: expected 4 values, got ${result.length}',
      );
    } catch (e) {
      print('[DEX] Error getting pool info: $e');
      // Return defaults to prevent UI crashes
      return {
        'pool_id': poolObjectId,
        'coin_type_a': coinTypeA,
        'coin_type_b': coinTypeB,
        'reserve_a': 0,
        'reserve_b': 0,
        'lp_supply': 0,
        'fee_percent': 30,
      };
    }
  }

  /// Calculate swap output amount for A -> B
  Future<int> calculateSwapAForBOutput({
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
    required int amountIn,
  }) async {
    try {
      print('[DEX] Calculating swap A->B output for: $amountIn');

      // Prepare arguments with normalization
      final normalizedPoolId = BcsSerializers.normalizeObjectId(poolObjectId);
      final poolIdBytes = BcsSerializers.hexToBytes(normalizedPoolId);
      final amountBytes = BcsSerializers.encodeU64(amountIn);

      // Call view function
      final result = await RpcUtils.executeViewFunction(
        client,
        url,
        DexConstants.packageAddress,
        DexConstants.dexModule,
        DexConstants.fnGetSwapAForBOutput,
        [coinTypeA, coinTypeB],
        [poolIdBytes, amountBytes],
      );

      // Return first element as int
      return result[0] as int;
    } catch (e) {
      print('[DEX] Error calculating swap output: $e');
      throw Exception('Failed to calculate swap output: $e');
    }
  }

  /// Calculate swap output amount for B -> A
  Future<int> calculateSwapBForAOutput({
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
    required int amountIn,
  }) async {
    try {
      print('[DEX] Calculating swap B->A output for: $amountIn');

      // Prepare arguments with normalization
      final normalizedPoolId = BcsSerializers.normalizeObjectId(poolObjectId);
      final poolIdBytes = BcsSerializers.hexToBytes(normalizedPoolId);
      final amountBytes = BcsSerializers.encodeU64(amountIn);

      // Call view function
      final result = await RpcUtils.executeViewFunction(
        client,
        url,
        DexConstants.packageAddress,
        DexConstants.dexModule,
        DexConstants.fnGetSwapBForAOutput,
        [coinTypeA, coinTypeB],
        [poolIdBytes, amountBytes],
      );

      // Return first element as int
      return result[0] as int;
    } catch (e) {
      print('[DEX] Error calculating swap output: $e');
      throw Exception('Failed to calculate swap output: $e');
    }
  }

  /// Get user's LP token balance for a pool
  Future<int> getLpTokenBalance({
    required String userAddress,
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
  }) async {
    try {
      print('[DEX] Getting LP balance for user: $userAddress');

      // Query user's account to get owned objects
      final account = await baseQueries.getAccount(userAddress);

      // Filter for LP tokens matching this pool
      final lpTokenType =
          '${DexConstants.packageAddress}::${DexConstants.dexModule}::${DexConstants.lpTokenTypePrefix}<$coinTypeA, $coinTypeB>';

      int totalBalance = 0;
      if (account.ownedObjects != null) {
        for (final obj in account.ownedObjects!) {
          if (obj.type.contains(lpTokenType)) {
            // Note: To get actual balance, you need to decode the object data
            // This is a simplified version - in production, decode BCS data from obj.data
            // For now, we return 0 and let developers implement custom decoding
            print('[DEX] Found LP token object: ${obj.id}');
            // TODO: Decode obj.data to extract coin balance
          }
        }
      }

      return totalBalance;
    } catch (e) {
      print('[DEX] Error getting LP balance: $e');
      throw Exception('Failed to get LP token balance: $e');
    }
  }

  /// List all pools created by user
  Future<List<Map<String, dynamic>>> getUserPools(String userAddress) async {
    try {
      print('[DEX] Getting pools for user: $userAddress');

      // Query user's account
      final account = await baseQueries.getAccount(userAddress);

      // Filter for Pool objects from our DEX module
      final pools = <Map<String, dynamic>>[];
      if (account.ownedObjects != null) {
        for (final obj in account.ownedObjects!) {
          final type = obj.type;
          if (type.contains(
            '${DexConstants.packageAddress}::${DexConstants.dexModule}::${DexConstants.poolType}',
          )) {
            pools.add({'object_id': obj.id, 'type': type, 'owner': obj.owner});
          }
        }
      }

      return pools;
    } catch (e) {
      print('[DEX] Error getting user pools: $e');
      throw Exception('Failed to get user pools: $e');
    }
  }
}
