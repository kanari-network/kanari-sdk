// modules/escrow/queries.dart
/// Escrow view functions and queries

import 'dart:convert';
import 'package:http/http.dart' as http;

import '../../client/kanari_client.dart';
import '../../core/bcs_utils.dart';
import '../../kanari_wallet.dart';
import 'constants.dart';

class EscrowQueries {
  final KanariClient rpc;

  const EscrowQueries(this.rpc);

  /// Get deal state by Object ID
  Future<int> getDealStateByObjectId({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
  }) async {
    try {
      final result = await _callViewFunction(
        wallet: wallet,
        functionName: EscrowConstants.fnGetState,
        coinType: coinType,
        args: TransactionArgs()..addObjectId(dealObjectId),
      );

      if (result.isEmpty) return 0;

      final firstResult = result.first;
      print('[ESCROW] getDealStateByObjectId raw result: $firstResult');

      // Parse based on type
      if (firstResult is int) return firstResult;
      if (firstResult is Map<String, dynamic>) {
        final resultValue = firstResult['result'];
        if (resultValue is int) return resultValue;
      }

      print('[ESCROW] getDealStateByObjectId: Failed to parse result');
      return 0;
    } catch (e) {
      print('[ESCROW] getDealStateByObjectId error: $e');
      rethrow;
    }
  }

  /// Get all deals for buyer
  Future<List<Map<String, dynamic>>> getAllDeals({
    required KanariWallet wallet,
    required String buyerAddress,
  }) async {
    print('[ESCROW] Querying deals for owner: $buyerAddress');

    try {
      final account = await rpc.getAccount(buyerAddress);
      final allObjects = account.ownedObjects ?? [];
      print('[ESCROW] Total owned objects: ${allObjects.length}');

      // Filter deal objects
      final dealObjects = allObjects
          .where((obj) => obj.type.contains('::escrow::EscrowDeal<'))
          .toList();

      print('[ESCROW] Found ${dealObjects.length} EscrowDeal objects');

      final deals = <Map<String, dynamic>>[];

      for (final obj in dealObjects) {
        final objectId = obj.id;
        final objectType = obj.type;
        final coinType = BcsUtils.extractCoinTypeFromObjectType(objectType);

        if (coinType == null) {
          print('[ESCROW]   ⚠️ Could not extract coin type from: $objectType');
          continue;
        }

        print('[ESCROW] Fetching deal details for: $objectId');
        final dealDetails = await getDealDetailsByObjectId(
          wallet: wallet,
          dealObjectId: objectId,
          coinType: coinType,
        );

        // Find matching proof object
        final proofObj = allObjects
            .where((o) => o.type.contains('::escrow::EscrowProof'))
            .firstOrNull;
        final proofId = proofObj?.id;

        if (proofId != null) {
          print('[ESCROW]   ✅ Found proof object: $proofId');
        } else {
          print('[ESCROW]   ⚠️ No proof object found for deal: $objectId');
        }

        deals.add({
          'object_id': objectId,
          'coin_type': coinType,
          'object_type': objectType,
          'proof_id': proofId,
          ...dealDetails,
        });
      }

      print('[ESCROW] Found ${deals.length} escrow deals');
      if (deals.isNotEmpty) {
        print('[ESCROW] First deal keys: ${deals.first.keys}');
        print('[ESCROW] First deal: ${deals.first}');
      }

      return deals;
    } catch (e, stack) {
      print('[ESCROW] Error querying deals: $e');
      print('[ESCROW] Stack: $stack');
      return [];
    }
  }

  /// Get deal details by Object ID
  Future<Map<String, dynamic>> getDealDetailsByObjectId({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
  }) async {
    try {
      final result = await _callViewFunction(
        wallet: wallet,
        functionName: EscrowConstants.fnGetDealDetails,
        coinType: coinType,
        args: TransactionArgs()..addObjectId(dealObjectId),
      );

      if (result.isEmpty) {
        print('[ESCROW] getDealDetailsByObjectId: Empty result');
        return {};
      }

      final firstResult = result.first;
      print('[ESCROW] getDealDetailsByObjectId raw result: $firstResult');

      // Parse from {action: view, result: [deal_id, buyer, seller, amount], status: success}
      if (firstResult is Map<String, dynamic>) {
        final resultValue = firstResult['result'];
        if (resultValue is List && resultValue.length >= 4) {
          return {
            'deal_id': resultValue[0] as String,
            'buyer': resultValue[1] as String,
            'seller': resultValue[2] as String,
            'amount': resultValue[3] as int,
          };
        }
      }

      print('[ESCROW] getDealDetailsByObjectId: Failed to parse result');
      return {};
    } catch (e) {
      print('[ESCROW] getDealDetailsByObjectId error: $e');
      rethrow;
    }
  }

  /// Generic view function caller
  Future<List<dynamic>> _callViewFunction({
    required KanariWallet wallet,
    required String functionName,
    required String coinType,
    required TransactionArgs args,
  }) async {
    final normalizedToken = BcsUtils.normalizeTokenType(coinType);
    final packageAddr = BcsUtils.normalizeAddress(
      EscrowConstants.packageAddress,
    );

    return await _viewFunction(
      wallet: wallet,
      function: '$packageAddr::${EscrowConstants.module}::$functionName',
      typeArguments: [normalizedToken],
      arguments: args.build(),
    );
  }

  /// Execute view function via RPC
  Future<List<dynamic>> _viewFunction({
    required KanariWallet wallet,
    required String function,
    required List<String> typeArguments,
    required List<List<int>> arguments,
  }) async {
    // Extract package, module, function from full function name
    final parts = function.split('::');
    if (parts.length != 3) {
      throw Exception('Invalid function format: $function');
    }

    final package = BcsUtils.normalizeAddress(parts[0]);
    final module = parts[1];
    final functionName = parts[2];

    // Convert args to hex strings for RPC
    final argsHex = arguments
        .map(
          (bytes) =>
              '0x${bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join()}',
        )
        .toList();

    // Build request data object
    final requestData = {
      'sender': wallet.taggedAddress,
      'package': package,
      'module': module,
      'function': functionName,
      'type_args': typeArguments,
      'args': argsHex,
    };

    // params must be an ARRAY containing the request object
    final body = {
      'jsonrpc': '2.0',
      'method': 'kanari_viewFunction',
      'params': [requestData],
      'id': DateTime.now().millisecondsSinceEpoch,
    };

    print('[ESCROW] Calling view function: $functionName');

    final response = await http.post(
      Uri.parse(rpc.url),
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode(body),
    );

    if (response.statusCode != 200) {
      throw Exception('View function failed: ${response.statusCode}');
    }

    final jsonResponse = jsonDecode(response.body) as Map<String, dynamic>;

    if (jsonResponse.containsKey('error')) {
      throw Exception('View function error: ${jsonResponse['error']}');
    }

    final result = jsonResponse['result'];
    print('[ESCROW] View function result: $result');

    return result is List ? result : [result];
  }
}
