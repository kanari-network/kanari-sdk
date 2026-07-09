// modules/escrow/queries.dart
// Escrow view functions and queries.

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
    final result = await _callViewFunction(
      wallet: wallet,
      functionName: EscrowConstants.fnGetState,
      coinType: coinType,
      args: TransactionArgs()..addObjectId(dealObjectId),
    );

    if (result.isEmpty) return 0;

    final firstResult = result.first;
    if (firstResult is int) return firstResult;
    if (firstResult is Map<String, dynamic>) {
      final resultValue = firstResult['result'];
      if (resultValue is int) return resultValue;
    }

    return 0;
  }

  /// Get all deals for buyer
  Future<List<Map<String, dynamic>>> getAllDeals({
    required KanariWallet wallet,
    required String buyerAddress,
  }) async {
    try {
      final account = await rpc.getOwner(buyerAddress);
      final allObjects = account.ownedObjects ?? [];

      final dealObjects = allObjects
          .where((obj) => obj.type.contains('::escrow::EscrowDeal<'))
          .toList();

      final deals = <Map<String, dynamic>>[];

      for (final obj in dealObjects) {
        final objectId = obj.id;
        final objectType = obj.type;
        final coinType = BcsUtils.extractCoinTypeFromObjectType(objectType);

        if (coinType == null) {
          continue;
        }

        final dealDetails = await getDealDetailsByObjectId(
          wallet: wallet,
          dealObjectId: objectId,
          coinType: coinType,
        );

        final proofObj = allObjects
            .where((o) => o.type.contains('::escrow::EscrowProof'))
            .firstOrNull;
        final proofId = proofObj?.id;

        deals.add({
          'object_id': objectId,
          'coin_type': coinType,
          'object_type': objectType,
          'proof_id': proofId,
          ...dealDetails,
        });
      }

      return deals;
    } catch (_) {
      return [];
    }
  }

  /// Get deal details by Object ID
  Future<Map<String, dynamic>> getDealDetailsByObjectId({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
  }) async {
    final result = await _callViewFunction(
      wallet: wallet,
      functionName: EscrowConstants.fnGetDealDetails,
      coinType: coinType,
      args: TransactionArgs()..addObjectId(dealObjectId),
    );

    if (result.isEmpty) {
      return {};
    }

    final firstResult = result.first;
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

    return {};
  }

  /// Generic view function caller
  Future<List<dynamic>> _callViewFunction({
    required KanariWallet wallet,
    required String functionName,
    required String coinType,
    required TransactionArgs args,
  }) {
    final normalizedToken = BcsUtils.normalizeTokenType(coinType);
    final packageAddr = BcsUtils.normalizeAddress(
      EscrowConstants.packageAddress,
    );

    return _viewFunction(
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
    final parts = function.split('::');
    if (parts.length != 3) {
      throw Exception('Invalid function format: $function');
    }

    final package = BcsUtils.normalizeAddress(parts[0]);
    final module = parts[1];
    final functionName = parts[2];

    final argsHex = arguments
        .map(
          (bytes) =>
              '0x${bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join()}',
        )
        .toList();

    final requestData = {
      'sender': wallet.taggedAddress,
      'package': package,
      'module': module,
      'function': functionName,
      'type_args': typeArguments,
      'args': argsHex,
    };

    final body = {
      'jsonrpc': '2.0',
      'method': 'kanari_viewFunction',
      'params': [requestData],
      'id': DateTime.now().millisecondsSinceEpoch,
    };

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
    return result is List ? result : [result];
  }
}
