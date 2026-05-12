// modules/dex/queries.dart
/// DEX query operations

import 'dart:convert';
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

  /// Get pool information using view function
  Future<Map<String, dynamic>> getPoolInfo({
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
  }) async {
    try {
      print('[DEX] Getting pool info for: $poolObjectId');

      // Prepare arguments - pool object ID must be in 0x format and encoded as bytes
      final normalizedPoolId = poolObjectId.startsWith('0x') 
          ? poolObjectId 
          : '0x$poolObjectId';
      final poolIdBytes = BcsSerializers.hexToBytes(normalizedPoolId);
      
      // Call view function
      final result = await RpcUtils.executeViewFunction(
        client,
        url,
        DexConstants.packageAddress,
        DexConstants.dexModule,
        DexConstants.fnGetPoolInfo,
        [coinTypeA, coinTypeB],
        [poolIdBytes],
      );

      // Result should be an array: [reserve_a, reserve_b, lp_supply, fee_percent]
      if (result.length < 4) {
        throw Exception('Invalid response format from get_pool_info');
      }

      return {
        'pool_id': poolObjectId,
        'coin_type_a': coinTypeA,
        'coin_type_b': coinTypeB,
        'reserve_a': result[0] as int,
        'reserve_b': result[1] as int,
        'lp_supply': result[2] as int,
        'fee_percent': result[3] as int,
      };
    } catch (e) {
      print('[DEX] Error getting pool info: $e');
      throw Exception('Failed to get pool info: $e');
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

      // Prepare arguments
      final normalizedPoolId = poolObjectId.startsWith('0x') 
          ? poolObjectId 
          : '0x$poolObjectId';
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

      // Prepare arguments
      final normalizedPoolId = poolObjectId.startsWith('0x') 
          ? poolObjectId 
          : '0x$poolObjectId';
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
      final lpTokenType = '${DexConstants.packageAddress}::${DexConstants.dexModule}::${DexConstants.lpTokenTypePrefix}<$coinTypeA, $coinTypeB>';
      
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
          if (type.contains('${DexConstants.packageAddress}::${DexConstants.dexModule}::${DexConstants.poolType}')) {
            pools.add({
              'object_id': obj.id,
              'type': type,
              'owner': obj.owner,
            });
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
