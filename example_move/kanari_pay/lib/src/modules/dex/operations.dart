// modules/dex/operations.dart
/// DEX transaction operations

import 'dart:convert';
import 'package:http/http.dart' as http;

import '../../core/bcs_serializers.dart';
import '../../kanari_wallet.dart';
import '../../models/transaction.dart';
import '../queries.dart';
import '../transactions/operations.dart';
import 'constants.dart';

class DexOperations {
  final String url;
  final QueriesModule queries;
  final TransactionOperations transactions;
  final http.Client client;

  DexOperations(this.url, this.queries, this.client) 
    : transactions = TransactionOperations(url, queries, client);

  /// Create a new liquidity pool
  Future<TransactionResult> createPool({
    required KanariWallet wallet,
    required String coinTypeA,
    required String coinTypeB,
    int feePercent = 30, // 0.3% default (in basis points)
    int gasLimit = 100000,
    int gasPrice = 1000,
  }) async {
    try {
      print('[DEX] Creating pool: $coinTypeA / $coinTypeB with fee $feePercent bps');

      // Encode arguments
      final feeBytes = BcsSerializers.encodeU64(feePercent);

      // Execute function
      return await transactions.executeFunction(
        wallet: wallet,
        package: DexConstants.packageAddress,
        module: DexConstants.dexModule,
        function: DexConstants.fnCreatePool,
        typeArgs: [coinTypeA, coinTypeB],
        args: [feeBytes],
        gasLimit: gasLimit,
        gasPrice: gasPrice,
      );
    } catch (e) {
      throw Exception('Failed to create pool: $e');
    }
  }

  /// Add liquidity to an existing pool
  Future<TransactionResult> addLiquidity({
    required KanariWallet wallet,
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
    required int amountA,
    required int amountB,
    int gasLimit = 200000,
    int gasPrice = 1000,
  }) async {
    try {
      print('[DEX] Adding liquidity: $amountA $coinTypeA + $amountB $coinTypeB');

      // Normalize pool object ID to ensure correct format
      final normalizedPoolId = poolObjectId.startsWith('0x') 
          ? poolObjectId 
          : '0x$poolObjectId';
      
      // Encode arguments - pool ID as address (32 bytes), amounts as u64
      final poolIdBytes = BcsSerializers.hexToBytes(normalizedPoolId);
      final amountABytes = BcsSerializers.encodeU64(amountA);
      final amountBBytes = BcsSerializers.encodeU64(amountB);

      // Execute function
      return await transactions.executeFunction(
        wallet: wallet,
        package: DexConstants.packageAddress,
        module: DexConstants.dexModule,
        function: DexConstants.fnAddLiquidity,
        typeArgs: [coinTypeA, coinTypeB],
        args: [poolIdBytes, amountABytes, amountBBytes],
        gasLimit: gasLimit,
        gasPrice: gasPrice,
      );

    } catch (e) {
      print('[DEX] Error adding liquidity: $e');
      throw Exception('Failed to add liquidity: $e');
    }
  }

  /// Remove liquidity from a pool
  Future<TransactionResult> removeLiquidity({
    required KanariWallet wallet,
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
    required int lpTokenAmount,
    int gasLimit = 200000,
    int gasPrice = 1000,
  }) async {
    try {
      print('[DEX] Removing liquidity: $lpTokenAmount LP tokens');

      // Normalize pool object ID
      final normalizedPoolId = poolObjectId.startsWith('0x') 
          ? poolObjectId 
          : '0x$poolObjectId';
      
      // Encode arguments
      final poolIdBytes = BcsSerializers.hexToBytes(normalizedPoolId);
      final lpAmountBytes = BcsSerializers.encodeU64(lpTokenAmount);

      // Execute function
      return await transactions.executeFunction(
        wallet: wallet,
        package: DexConstants.packageAddress,
        module: DexConstants.dexModule,
        function: DexConstants.fnRemoveLiquidity,
        typeArgs: [coinTypeA, coinTypeB],
        args: [poolIdBytes, lpAmountBytes],
        gasLimit: gasLimit,
        gasPrice: gasPrice,
      );

    } catch (e) {
      print('[DEX] Error removing liquidity: $e');
      throw Exception('Failed to remove liquidity: $e');
    }
  }

  /// Swap Coin A for Coin B
  Future<TransactionResult> swapAForB({
    required KanariWallet wallet,
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
    required int amountIn,
    int gasLimit = 150000,
    int gasPrice = 1000,
  }) async {
    try {
      print('[DEX] Swapping $amountIn $coinTypeA for $coinTypeB');

      // Normalize pool object ID
      final normalizedPoolId = poolObjectId.startsWith('0x') 
          ? poolObjectId 
          : '0x$poolObjectId';
      
      // Encode arguments
      final poolIdBytes = BcsSerializers.hexToBytes(normalizedPoolId);
      final amountInBytes = BcsSerializers.encodeU64(amountIn);

      // Execute function
      return await transactions.executeFunction(
        wallet: wallet,
        package: DexConstants.packageAddress,
        module: DexConstants.dexModule,
        function: DexConstants.fnSwapAForB,
        typeArgs: [coinTypeA, coinTypeB],
        args: [poolIdBytes, amountInBytes],
        gasLimit: gasLimit,
        gasPrice: gasPrice,
      );

    } catch (e) {
      print('[DEX] Error swapping A for B: $e');
      throw Exception('Failed to swap A for B: $e');
    }
  }

  /// Swap Coin B for Coin A
  Future<TransactionResult> swapBForA({
    required KanariWallet wallet,
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
    required int amountIn,
    int gasLimit = 150000,
    int gasPrice = 1000,
  }) async {
    try {
      print('[DEX] Swapping $amountIn $coinTypeB for $coinTypeA');

      // Normalize pool object ID
      final normalizedPoolId = poolObjectId.startsWith('0x') 
          ? poolObjectId 
          : '0x$poolObjectId';
      
      // Encode arguments
      final poolIdBytes = BcsSerializers.hexToBytes(normalizedPoolId);
      final amountInBytes = BcsSerializers.encodeU64(amountIn);

      // Execute function
      return await transactions.executeFunction(
        wallet: wallet,
        package: DexConstants.packageAddress,
        module: DexConstants.dexModule,
        function: DexConstants.fnSwapBForA,
        typeArgs: [coinTypeA, coinTypeB],
        args: [poolIdBytes, amountInBytes],
        gasLimit: gasLimit,
        gasPrice: gasPrice,
      );

    } catch (e) {
      print('[DEX] Error swapping B for A: $e');
      throw Exception('Failed to swap B for A: $e');
    }
  }
}
