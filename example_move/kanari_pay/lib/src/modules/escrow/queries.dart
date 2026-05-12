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
      // 🔥 FIXED: Params must be an OBJECT, not an array
      final body = {
        'jsonrpc': '2.0',
        'method': 'kanari_getOwnedObjects',
        'params': {
          'owner': buyerAddress,
          'object_type': '${EscrowConstants.packageAddress}::${EscrowConstants.module}::${EscrowConstants.objectTypeDeal}', // Filter for EscrowDeal objects only
        },
        'id': DateTime.now().millisecondsSinceEpoch,
      };

      print('[ESCROW] Request params: ${body['params']}');

      final response = await http.post(
        Uri.parse(rpc.url),
        headers: {'Content-Type': 'application/json'},
        body: jsonEncode(body),
      );

      if (response.statusCode != 200) {
        print('[ESCROW] RPC error: ${response.statusCode}');
        print('[ESCROW] Response body: ${response.body}');
        return [];
      }

      final jsonResponse = jsonDecode(response.body) as Map<String, dynamic>;
      
      if (jsonResponse['error'] != null) {
        print('[ESCROW] RPC error response: ${jsonResponse['error']}');
        return [];
      }
      
      if (jsonResponse['result'] != null &&
          jsonResponse['result']['objects'] != null) {
        final objects = (jsonResponse['result']['objects'] as List)
            .cast<Map<String, dynamic>>();
        print('[ESCROW] Parsed ${objects.length} objects from RPC response');

        final deals = <Map<String, dynamic>>[];

        for (final obj in objects) {
          final objectId = obj['id'] as String?;
          final objectType = obj['type_'] as String?; // Rust uses type_ (with underscore)

          if (objectId != null && objectType != null) {
            print('[ESCROW] Found escrow deal:');
            print('[ESCROW]   Object ID: $objectId');
            print('[ESCROW]   Object Type: $objectType');

            // Extract coin type from object type
            // Format: 0xPKG::escrow::EscrowDeal<0xPKG::usdc::USDC>
            final coinType = _extractCoinTypeFromObjectType(objectType);

            if (coinType != null) {
              //  NEW: Get deal details via view function
              print('[ESCROW] Fetching deal details for: $objectId');
              final dealDetails = await getDealDetailsByObjectId(
                wallet: wallet,
                dealObjectId: objectId,
                coinType: coinType,
              );

              deals.add({
                'object_id': objectId,
                'coin_type': coinType,
                'object_type': objectType,
                ...dealDetails, // Merge deal details (deal_id, buyer, seller, amount)
              });
              print('[ESCROW]   ✅ Deal details loaded: ${dealDetails.keys}');
            } else {
              print('[ESCROW]   ⚠️ Could not extract coin type from: $objectType');
            }
          }
        }

        print('[ESCROW] Found ${deals.length} escrow deals');
        if (deals.isNotEmpty) {
          print('[ESCROW] First deal keys: ${deals.first.keys}');
          print('[ESCROW] First deal: ${deals.first}');
        }

        return deals;
      }

      print('[ESCROW] No escrow deals found');
      return [];
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
