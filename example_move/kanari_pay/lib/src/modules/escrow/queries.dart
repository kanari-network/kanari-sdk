// modules/escrow/queries.dart
/// Escrow view functions and queries

import 'dart:convert';
import 'package:http/http.dart' as http;

import '../../core/bcs_serializers.dart';
import '../../client/kanari_client.dart';
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
      final result = await _viewFunction(
        wallet: wallet,
        function:
            '${EscrowConstants.packageAddress}::${EscrowConstants.module}::${EscrowConstants.fnGetState}',
        typeArguments: [_normalizeTokenType(coinType)],
        arguments: [BcsSerializers.hexToBytes(dealObjectId)],
      );

      if (result.isEmpty) {
        print('[ESCROW] getDealStateByObjectId: Empty result');
        return 0;
      }

      final firstResult = result.first;
      print('[ESCROW] getDealStateByObjectId raw result: $firstResult');

      // Parse based on type
      if (firstResult is int) {
        return firstResult;
      } else if (firstResult is Map<String, dynamic>) {
        final resultValue = firstResult['result'];
        if (resultValue is int) {
          return resultValue;
        }
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
      // Get account info to access all owned objects
      final account = await rpc.getAccount(buyerAddress);
      final allObjects = account.ownedObjects ?? [];

      print('[ESCROW] Total owned objects: ${allObjects.length}');

      // Separate deal objects and proof objects
      final dealObjects = <Map<String, dynamic>>[];

      for (final obj in allObjects) {
        final objectType = obj.type;
        final objectId = obj.id;

        if (objectType.contains('::escrow::EscrowDeal<')) {
          dealObjects.add({'id': objectId, 'type': objectType});
          print('[ESCROW] Found EscrowDeal: $objectId');
        } else if (objectType.contains('::escrow::EscrowProof')) {
          // Try to extract deal_id from proof object
          // For now, we'll match them by checking proof objects after loading deals
          print('[ESCROW] Found EscrowProof: $objectId');
        }
      }

      print('[ESCROW] Found ${dealObjects.length} EscrowDeal objects');

      final deals = <Map<String, dynamic>>[];

      for (final dealObj in dealObjects) {
        final objectId = dealObj['id'] as String;
        final objectType = dealObj['type'] as String;

        // Extract coin type from object type
        final coinType = _extractCoinTypeFromObjectType(objectType);

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

        // Find matching proof object for this deal
        final proofId = await _findProofForDeal(
          allObjects: allObjects,
          dealObjectId: objectId,
        );

        if (proofId != null) {
          print('[ESCROW]   ✅ Found proof object: $proofId');
        } else {
          print('[ESCROW]   ⚠️ No proof object found for deal: $objectId');
        }

        deals.add({
          'object_id': objectId,
          'coin_type': coinType,
          'object_type': objectType,
          'proof_id': proofId, // Add proof_id to deal data
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

  /// Find proof object that belongs to a specific deal
  Future<String?> _findProofForDeal({
    required List<dynamic> allObjects,
    required String dealObjectId,
  }) async {
    // Strategy: Find EscrowProof objects and match them to deals
    // Since we can't read proof object data directly without a view function,
    // we'll try to find proof objects owned by the same address

    for (final obj in allObjects) {
      if (obj.type.contains('::escrow::EscrowProof')) {
        // For now, return the first proof object we find
        // TODO: Add a view function to match proof to deal properly
        return obj.id as String?;
      }
    }

    return null;
  }

  /// Get deal details by Object ID
  Future<Map<String, dynamic>> getDealDetailsByObjectId({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
  }) async {
    try {
      final result = await _viewFunction(
        wallet: wallet,
        function:
            '${EscrowConstants.packageAddress}::${EscrowConstants.module}::${EscrowConstants.fnGetDealDetails}',
        typeArguments: [_normalizeTokenType(coinType)],
        arguments: [BcsSerializers.hexToBytes(dealObjectId)],
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

  /// Execute view function
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

    final package = parts[0];
    final module = parts[1];
    final functionName = parts[2];

    final senderAddress = wallet.taggedAddress;
    final packageAddress = _normalizeAddress(package);

    // Convert args to hex strings for RPC
    final argsHex = arguments
        .map(
          (bytes) =>
              '0x${bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join()}',
        )
        .toList();

    // Build request data object
    final requestData = {
      'sender': senderAddress,
      'package': packageAddress,
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

    if (result is List) {
      return result;
    }

    return [result];
  }

  /// Extract coin type from object type
  String? _extractCoinTypeFromObjectType(String objectType) {
    // Format: 0xPKG::escrow::EscrowDeal<0xPKG::usdc::USDC>
    final match = RegExp(r'<([^>]+)>').firstMatch(objectType);
    return match?.group(1);
  }

  /// Normalize address
  String _normalizeAddress(String addr) {
    final clean = addr.startsWith('0x') ? addr.substring(2) : addr;
    return '0x${clean.padLeft(64, '0')}';
  }

  /// Normalize token type
  String _normalizeTokenType(String tokenType) {
    if (tokenType.startsWith('0x')) return tokenType;
    return '0x$tokenType';
  }
}
