import 'dart:convert';
import 'dart:typed_data';

import 'package:bcs/bcs.dart';
import 'package:http/http.dart' as http;
import 'package:kanari_crypto/kanari_crypto.dart';

import 'kanari_client.dart';
import 'kanari_wallet.dart';
import 'models/account.dart';
import 'models/transaction.dart';

class EscrowObjectRefs {
  final String dealObjectId;
  final String proofObjectId;
  final String coinType;

  const EscrowObjectRefs({
    required this.dealObjectId,
    required this.proofObjectId,
    required this.coinType,
  });
}

class EscrowClient {
  static const String escrowPackageAddress =
      '0x3ba63b92aac5f2bff87e580e820b61faf1c5fe9ae12f0bc8addd931a340b3146';
  static const String kanariCoinType = '0x2::kanari::KANARI';
  static final _transactionBcs = Bcs.enumeration('Transaction', {
    'PublishModule': Bcs.struct('PublishModule', {
      'sender': Bcs.string(),
      'module_bytes': Bcs.vector(Bcs.u8()),
      'module_name': Bcs.string(),
      'gas_limit': Bcs.u64(),
      'gas_price': Bcs.u64(),
      'sequence_number': Bcs.u64(),
    }),
    'ExecuteFunction': Bcs.struct('ExecuteFunction', {
      'sender': Bcs.string(),
      'module': Bcs.string(),
      'function': Bcs.string(),
      'type_args': Bcs.vector(Bcs.string()),
      'args': Bcs.vector(Bcs.vector(Bcs.u8())),
      'gas_limit': Bcs.u64(),
      'gas_price': Bcs.u64(),
      'sequence_number': Bcs.u64(),
    }),
    'Transfer': Bcs.struct('Transfer', {
      'from': Bcs.string(),
      'to': Bcs.string(),
      'amount': Bcs.u64(),
      'gas_limit': Bcs.u64(),
      'gas_price': Bcs.u64(),
      'sequence_number': Bcs.u64(),
    }),
    'Burn': Bcs.struct('Burn', {
      'from': Bcs.string(),
      'amount': Bcs.u64(),
      'gas_limit': Bcs.u64(),
      'gas_price': Bcs.u64(),
      'sequence_number': Bcs.u64(),
    }),
  });

  final KanariClient rpc;

  const EscrowClient(this.rpc);

  TransactionResult _requireSuccess(TransactionResult result) {
    final status = result.status.toLowerCase();
    if (status == 'failed') {
      throw Exception(
        result.errorMessage?.isNotEmpty == true
            ? result.errorMessage!
            : 'Escrow transaction failed on-chain.',
      );
    }
    return result;
  }

  Future<TransactionResult> createDeal({
    required KanariWallet wallet,
    required String dealId,
    required String sellerAddress,
    required int amount,
    required String description,
    required String tokenType,
    int gasLimit = 100000,
    int gasPrice = 10,
  }) async {
    final wantedToken = _normalizeTokenType(tokenType);

    // Debug: Log wallet address and token type
    print('[ESCROW] Creating deal:');
    print('[ESCROW]   Wallet: ${wallet.address}');
    print('[ESCROW]   Token: $wantedToken');
    print('[ESCROW]   Amount: $amount');
    print('[ESCROW]   Seller: $sellerAddress');
    print('[ESCROW]   Deal ID: $dealId');
    print('[ESCROW]   Description: $description');

    final coinObjectId = await _findOwnedCoinObjectId(
      ownerAddress: wallet.address,
      tokenType: wantedToken,
    );

    final normalizedCoinObjectId = _normalizeObjectId(coinObjectId);
    print('[ESCROW]   Coin Object ID: $coinObjectId');
    if (normalizedCoinObjectId != coinObjectId) {
      print('[ESCROW]   Normalized Coin Object ID: $normalizedCoinObjectId');
    }

    // CRITICAL: For mutable reference objects (&mut Coin), Move VM expects
    // raw 32-byte object ID, NOT string or BCS-encoded value.
    // The VM will look up the full object from storage automatically.
    final coinObjectBytes = _hexToBytes(normalizedCoinObjectId);
    print('[ESCROW]   Coin Object bytes length: ${coinObjectBytes.length}');
    print(
      '[ESCROW]   Coin Object bytes hex: ${coinObjectBytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join()}',
    );

    // Serialize all arguments properly for Move VM
    final dealIdBytes = _encodeStringBcs(dealId);
    final sellerBytes = _encodeAddressBcs(sellerAddress);
    final amountBytes = _encodeU64Bcs(amount);
    final descriptionBytes = _encodeStringBcs(description);

    print('[ESCROW]   Deal ID bytes length: ${dealIdBytes.length}');
    print('[ESCROW]   Seller bytes length: ${sellerBytes.length}');
    print('[ESCROW]   Amount bytes length: ${amountBytes.length}');
    print('[ESCROW]   Description bytes length: ${descriptionBytes.length}');
    print('[ESCROW]   Total args: 5');

    final result = await _executeFunctionDetailed(
      wallet: wallet,
      package: escrowPackageAddress,
      module: 'escrow',
      function: 'create_deal',
      typeArgs: [wantedToken],
      args: [
        dealIdBytes, // deal_id: String (BCS serialized)
        sellerBytes, // seller: address (32 bytes)
        amountBytes, // amount: u64 (8 bytes LE)
        descriptionBytes, // description: String (BCS serialized)
        coinObjectBytes, // buyer_coin: &mut Coin (32 bytes object ID)
      ],
      gasLimit: gasLimit,
      gasPrice: gasPrice,
      executeImmediate: true,
    );
    return _requireSuccess(result);
  }

  /// Confirm delivery by Object ID
  Future<void> confirmDeliveryByObjectId({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
    required String proofObjectId,
  }) async {
    print('[ESCROW CLIENT] confirmDeliveryByObjectId:');
    print('[ESCROW CLIENT]   dealObjectId: $dealObjectId');
    print('[ESCROW CLIENT]   coinType: $coinType');
    print('[ESCROW CLIENT]   proofObjectId: $proofObjectId');

    final result = await _executeFunctionDetailed(
      wallet: wallet,
      package: escrowPackageAddress,
      module: 'escrow',
      function: 'confirm_delivery',
      typeArgs: [coinType],
      args: [_hexToBytes(dealObjectId), _hexToBytes(proofObjectId)],
      gasLimit: 5000000,
      gasPrice: 100,
    );
    _requireSuccess(result);
  }

  /// Release funds by Object ID
  Future<void> releaseFundsByObjectId({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
    required String proofObjectId,
  }) async {
    final result = await _executeFunctionDetailed(
      wallet: wallet,
      package: escrowPackageAddress,
      module: 'escrow',
      function: 'release_funds',
      typeArgs: [coinType],
      args: [_hexToBytes(dealObjectId), _hexToBytes(proofObjectId)],
      gasLimit: 5000000,
      gasPrice: 100,
    );
    _requireSuccess(result);
  }

  /// Raise dispute by Object ID
  Future<void> raiseDisputeByObjectId({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
    required String proofObjectId,
    String? reason,
  }) async {
    final args = <List<int>>[
      _hexToBytes(dealObjectId),
      _hexToBytes(proofObjectId),
    ];

    if (reason != null && reason.isNotEmpty) {
      // Add reason as String argument
      args.add(reason.codeUnits);
    }

    final result = await _executeFunctionDetailed(
      wallet: wallet,
      package: escrowPackageAddress,
      module: 'escrow',
      function: 'raise_dispute',
      typeArgs: [coinType],
      args: args,
      gasLimit: 5000000,
      gasPrice: 100,
    );
    _requireSuccess(result);
  }

  /// Confirm delivery (backward compatible - uses first deal)
  Future<TransactionResult> confirmDelivery({
    required KanariWallet wallet,
    required String buyerAddress,
    int gasLimit = 100000,
    int gasPrice = 10,
  }) async {
    final refs = await _getEscrowObjectRefs(buyerAddress);

    // Debug: Check current state before confirming
    final currentState = await getDealState(
      wallet: wallet,
      buyerAddress: buyerAddress,
    );
    print('[ESCROW] Current deal state: $currentState (expected: 1=LOCKED)');

    if (currentState != 1) {
      throw Exception(
        'Deal is not in LOCKED state. Current state: ${_getStateName(currentState)}. '
        'Cannot confirm delivery. Please create a new deal first.',
      );
    }

    final result = await _executeFunctionDetailed(
      wallet: wallet,
      package: escrowPackageAddress,
      module: 'escrow',
      function: 'confirm_delivery',
      typeArgs: [refs.coinType],
      args: [_hexToBytes(refs.dealObjectId), _hexToBytes(refs.proofObjectId)],
      gasLimit: gasLimit,
      gasPrice: gasPrice,
      executeImmediate: true,
    );
    return _requireSuccess(result);
  }

  Future<TransactionResult> releaseFunds({
    required KanariWallet wallet,
    required String buyerAddress,
    int gasLimit = 100000,
    int gasPrice = 10,
  }) async {
    final refs = await _getEscrowObjectRefs(buyerAddress);

    // Debug: Check current state before releasing
    final currentState = await getDealState(
      wallet: wallet,
      buyerAddress: buyerAddress,
    );
    print('[ESCROW] Current deal state: $currentState (expected: 2=DELIVERED)');

    if (currentState != 2) {
      throw Exception(
        'Deal is not in DELIVERED state. Current state: ${_getStateName(currentState)}. '
        'Seller must confirm delivery first.',
      );
    }

    final result = await _executeFunctionDetailed(
      wallet: wallet,
      package: escrowPackageAddress,
      module: 'escrow',
      function: 'release_funds',
      typeArgs: [refs.coinType],
      args: [_hexToBytes(refs.dealObjectId), _hexToBytes(refs.proofObjectId)],
      gasLimit: gasLimit,
      gasPrice: gasPrice,
      executeImmediate: true,
    );
    return _requireSuccess(result);
  }

  Future<TransactionResult> raiseDispute({
    required KanariWallet wallet,
    required String buyerAddress,
    required String reason, // เพิ่ม reason parameter
    int gasLimit = 100000,
    int gasPrice = 10,
  }) async {
    final refs = await _getEscrowObjectRefs(buyerAddress);

    // Serialize reason as BCS string
    final reasonBytes = _encodeStringBcs(reason);

    final result = await _executeFunctionDetailed(
      wallet: wallet,
      package: escrowPackageAddress,
      module: 'escrow',
      function: 'raise_dispute',
      typeArgs: [refs.coinType],
      args: [
        _hexToBytes(refs.dealObjectId),
        _hexToBytes(refs.proofObjectId),
        reasonBytes, // เพิ่ม reason argument
      ],
      gasLimit: gasLimit,
      gasPrice: gasPrice,
      executeImmediate: true,
    );
    return _requireSuccess(result);
  }

  /// Get deal state by Object ID
  Future<int> getDealStateByObjectId({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
  }) async {
    try {
      final result = await _viewFunction(
        wallet: wallet,
        function: '$escrowPackageAddress::escrow::get_state',
        typeArguments: [coinType],
        arguments: [_hexToBytes(dealObjectId)],
      );

      if (result.isEmpty) {
        print('[ESCROW] getDealStateByObjectId: Empty result');
        return 0;
      }

      final firstResult = result.first;
      print('[ESCROW] getDealStateByObjectId raw result: $firstResult');

      // Parse based on type (same logic as getDealState)
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

  /// Get full deal details by Object ID
  Future<Map<String, dynamic>> getDealDetailsByObjectId({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
  }) async {
    try {
      final result = await _viewFunction(
        wallet: wallet,
        function: '$escrowPackageAddress::escrow::get_deal_details',
        typeArguments: [coinType],
        arguments: [_hexToBytes(dealObjectId)],
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

  /// Get deal state by buyer address (using view function - now working!)
  Future<int> getDealState({
    required KanariWallet wallet,
    required String buyerAddress,
  }) async {
    try {
      // Query objects via RPC
      final deals = await _getEscrowDealsByOwner(wallet, buyerAddress);

      if (deals.isEmpty) {
        print('[ESCROW] No deals found for buyer: $buyerAddress');
        return 0; // STATE_NONE
      }

      // Get the first (latest) deal
      final firstDeal = deals.first;
      final dealObjectId = firstDeal['object_id'] as String;
      final coinType = firstDeal['coin_type'] as String;

      print('[ESCROW] Found ${deals.length} deal(s), using: $dealObjectId');
      print('[ESCROW] Coin type: $coinType');

      // Use view function to get state (now works with object preloading!)
      final result = await _viewFunction(
        wallet: wallet,
        function: '$escrowPackageAddress::escrow::get_state',
        typeArguments: [coinType],
        arguments: [_hexToBytes(dealObjectId)],
      );

      if (result.isEmpty) {
        print('[ESCROW] ⚠️ View function returned empty result');
        return 0;
      }

      // 🔥 FIXED: Parse result based on actual type
      final firstResult = result.first;
      print('[ESCROW] Raw result type: ${firstResult.runtimeType}');
      print('[ESCROW] Raw result value: $firstResult');

      int state;

      if (firstResult is int) {
        // Direct integer value
        state = firstResult;
        print('[ESCROW] ✅ Parsed direct int: $state');
      } else if (firstResult is Map<String, dynamic>) {
        // Result is wrapped in a Map: {action: view, result: 1, status: success}
        if (firstResult.containsKey('result')) {
          final resultValue = firstResult['result'];
          if (resultValue is int) {
            state = resultValue;
            print('[ESCROW] ✅ Parsed from Map.result: $state');
          } else if (resultValue is List) {
            // For functions returning arrays
            state = resultValue.isNotEmpty ? resultValue.first as int : 0;
            print('[ESCROW] ✅ Parsed from Map.result (List): $state');
          } else {
            print(
              '[ESCROW] ⚠️ Unexpected result value type: ${resultValue.runtimeType}',
            );
            return 0;
          }
        } else {
          // Unknown Map structure - log keys for debugging
          print(
            '[ESCROW] ⚠️ Unknown Map structure with keys: ${firstResult.keys}',
          );
          print('[ESCROW] Full Map: $firstResult');
          return 0;
        }
      } else if (firstResult is List) {
        // Result is nested in a List
        if (firstResult.isNotEmpty && firstResult.first is int) {
          state = firstResult.first as int;
          print('[ESCROW] ✅ Parsed from nested List: $state');
        } else {
          print('[ESCROW] ⚠️ Unexpected nested List: $firstResult');
          return 0;
        }
      } else {
        print('[ESCROW] ⚠️ Unexpected result type: ${firstResult.runtimeType}');
        print('[ESCROW] Value: $firstResult');
        return 0;
      }

      print('[ESCROW] ✅ View function returned state: $state');
      return state;
    } catch (e, stack) {
      print('[ESCROW] ❌ Error getting deal state: $e');
      print('[ESCROW] Stack: $stack');
      rethrow;
    }
  }

  /// Get full deal details by buyer address (backward compatible)
  Future<Map<String, dynamic>> getDealDetails({
    required KanariWallet wallet,
    required String buyerAddress,
  }) async {
    try {
      final refs = await _getEscrowObjectRefs(buyerAddress);
      final result = await _viewFunction(
        wallet: wallet,
        function: '$escrowPackageAddress::escrow::get_deal_details',
        typeArguments: [refs.coinType],
        arguments: [_hexToBytes(refs.dealObjectId)],
      );

      // Parse from view function result
      // Format: {action: view, result: [deal_id, buyer, seller, amount], status: success}
      if (result.isNotEmpty) {
        final firstResult = result.first;
        print('[ESCROW] get_deal_details raw result: $firstResult');

        if (firstResult is Map<String, dynamic>) {
          final resultValue = firstResult['result'];
          if (resultValue is List && resultValue.length >= 4) {
            // Extract deal details from array
            final dealId = resultValue[0] as String;
            final buyer = resultValue[1] as String;
            final seller = resultValue[2] as String;
            final amount = resultValue[3] as int;

            print('[ESCROW] ✅ Parsed deal details:');
            print('[ESCROW]   Deal ID: $dealId');
            print('[ESCROW]   Buyer: $buyer');
            print('[ESCROW]   Seller: $seller');
            print('[ESCROW]   Amount: $amount');

            return {
              'deal_id': dealId,
              'buyer': buyer,
              'seller': seller,
              'amount': amount,
              'object_id': refs.dealObjectId,
              'coin_type': refs.coinType,
            };
          }
        }
      }

      print('[ESCROW] ⚠️ Failed to parse deal details');
      return {};
    } catch (e, stack) {
      print('[ESCROW] ❌ Error getting deal details: $e');
      print('[ESCROW] Stack: $stack');
      rethrow;
    }
  }

  /// Get all deals for a buyer address
  Future<List<Map<String, dynamic>>> getAllDeals({
    required KanariWallet wallet,
    required String buyerAddress,
  }) async {
    final account = await rpc.getAccount(buyerAddress);
    final deals = <Map<String, dynamic>>[];

    for (final obj in account.ownedObjects ?? const <ObjectInfo>[]) {
      if (_isEscrowDealObject(obj.type)) {
        final dealObjectId = _normalizeObjectId(obj.id);
        final coinType = _extractCoinTypeFromObjectType(obj.type);

        if (coinType != null) {
          try {
            final result = await _viewFunction(
              wallet: wallet,
              function: '$escrowPackageAddress::escrow::get_deal_details',
              typeArguments: [coinType],
              arguments: [_hexToBytes(dealObjectId)],
            );

            // Parse from view function result
            // Format: {action: view, result: [deal_id, buyer, seller, amount], status: success}
            if (result.isNotEmpty) {
              final firstResult = result.first;

              if (firstResult is Map<String, dynamic>) {
                final resultValue = firstResult['result'];
                if (resultValue is List && resultValue.length >= 4) {
                  final dealData = {
                    'deal_id': resultValue[0] as String,
                    'buyer': resultValue[1] as String,
                    'seller': resultValue[2] as String,
                    'amount': resultValue[3] as int,
                    'object_id': dealObjectId,
                    'coin_type': coinType,
                  };

                  // Find proof object for this deal
                  final proofId = await _findProofForDeal(
                    buyerAddress,
                    dealObjectId,
                  );
                  if (proofId != null) {
                    dealData['proof_id'] = proofId;
                  }

                  deals.add(dealData);
                }
              }
            }
          } catch (e) {
            print('[ESCROW] Failed to get details for deal $dealObjectId: $e');
            continue;
          }
        }
      }
    }

    return deals;
  }

  /// Check if deal is in expected state
  Future<bool> checkDealState({
    required KanariWallet wallet,
    required String buyerAddress,
    required int expectedState,
  }) async {
    final refs = await _getEscrowObjectRefs(buyerAddress);
    final result = await _viewFunction(
      wallet: wallet,
      function: '$escrowPackageAddress::escrow::check_deal_state',
      typeArguments: [refs.coinType],
      arguments: [_hexToBytes(refs.dealObjectId), _encodeU64Bcs(expectedState)],
    );

    // Returns true if state matches (new_state == expected_state)
    if (result.isNotEmpty && result.first is int) {
      return (result.first as int) == expectedState;
    }
    return false;
  }

  Future<List<String>> getSpendableCoinTypes(String ownerAddress) async {
    final account = await rpc.getAccount(ownerAddress);
    final coinTypes = <String>{};

    for (final obj in account.ownedObjects ?? const <ObjectInfo>[]) {
      final objToken = _coinTokenFromObjectType(obj.type);
      if (objToken != null && objToken.isNotEmpty) {
        coinTypes.add(objToken);
      }
    }

    final sorted = coinTypes.toList()..sort();
    return sorted;
  }

  Future<List<dynamic>> _viewFunction({
    required KanariWallet wallet,
    required String function,
    required List<String> typeArguments,
    required List<List<int>> arguments,
  }) async {
    // Extract package, module, function from full function name
    // Format: "0xpackage::module::function"
    final parts = function.split('::');
    if (parts.length != 3) {
      throw Exception('Invalid function format: $function');
    }

    final package = parts[0];
    final module = parts[1];
    final functionName = parts[2];

    final senderAddress = wallet.taggedAddress;
    final packageAddress = _normalizeAddress(package);

    // 🔥 FIXED: Convert args to hex strings for RPC (like CLI does)
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

    // 🔥 CRITICAL: params must be an ARRAY containing the request object
    // NOT the request object directly!
    final body = {
      'jsonrpc': '2.0',
      'method': 'kanari_viewFunction',
      'params': [requestData], // ✅ Array wrapper around request data
      'id': DateTime.now().millisecondsSinceEpoch,
    };

    print('[ESCROW] Calling view function: $functionName');
    print('[ESCROW] Params: $body');

    final response = await http.post(
      Uri.parse(rpc.url),
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode(body),
    );

    if (response.statusCode != 200) {
      throw Exception(
        'Failed to connect to Kanari RPC: ${response.statusCode}',
      );
    }

    final jsonResponse = jsonDecode(response.body) as Map<String, dynamic>;

    if (jsonResponse['error'] != null) {
      final error = jsonResponse['error'] as Map<String, dynamic>;
      final errorMessage = error['message'] as String? ?? 'Unknown RPC error';

      print('[ESCROW] ❌ View function error: $errorMessage');
      throw Exception('View function execution failed: $errorMessage');
    }

    final resultJson = jsonResponse['result'];

    // View function returns result directly (not wrapped in transaction response)
    if (resultJson == null) {
      print('[ESCROW] ⚠️ View function returned null result');
      return [];
    }

    print('[ESCROW] ✅ View function result: $resultJson');

    // Parse result based on type
    if (resultJson is int) {
      // Single integer (e.g., state value)
      return [resultJson];
    } else if (resultJson is List) {
      // Array of values
      return resultJson;
    } else if (resultJson is Map) {
      // Complex object
      return [resultJson];
    }

    // Fallback: try to parse as JSON string
    return [resultJson.toString()];
  }

  Future<TransactionResult> _executeFunctionDetailed({
    required KanariWallet wallet,
    required String package,
    required String module,
    required String function,
    required List<String> typeArgs,
    required List<List<int>> args,
    required int gasLimit,
    required int gasPrice,
    bool executeImmediate = true,
  }) async {
    final account = await rpc.getAccount(wallet.address);
    final sequenceNumber = account.sequenceNumber;
    final senderAddress = wallet.taggedAddress;
    final packageAddress = _normalizeAddress(package);

    final serializedTx = _transactionBcs.serialize({
      'ExecuteFunction': {
        'sender': senderAddress,
        'module': '$packageAddress::$module',
        'function': function,
        'type_args': typeArgs,
        'args': args,
        'gas_limit': gasLimit,
        'gas_price': gasPrice,
        'sequence_number': sequenceNumber,
      },
    }).toBytes();

    List<int> messageToSign;
    try {
      messageToSign = await blake3HashApi(data: serializedTx);
    } catch (error) {
      if (error.toString().contains(
        'flutter_rust_bridge has not been initialized',
      )) {
        messageToSign = serializedTx;
      } else {
        rethrow;
      }
    }

    final signature = await wallet.sign(messageToSign);
    final body = {
      'jsonrpc': '2.0',
      'method': 'kanari_callFunction',
      'params': {
        'sender': senderAddress,
        'package': packageAddress,
        'module': module,
        'function': function,
        'type_args': typeArgs,
        'args': args,
        'gas_limit': gasLimit,
        'gas_price': gasPrice,
        'sequence_number': sequenceNumber,
        'signature': signature.toList(),
        'execute_immediate': executeImmediate,
      },
      'id': DateTime.now().millisecondsSinceEpoch,
    };

    final response = await http.post(
      Uri.parse(rpc.url),
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode(body),
    );

    if (response.statusCode != 200) {
      throw Exception(
        'Failed to connect to Kanari RPC: ${response.statusCode}',
      );
    }

    final jsonResponse = jsonDecode(response.body) as Map<String, dynamic>;
    if (jsonResponse['error'] != null) {
      final error = jsonResponse['error'] as Map<String, dynamic>;
      throw Exception(error['message'] ?? 'Unknown RPC error');
    }

    final resultJson = jsonResponse['result'];
    if (resultJson is! Map<String, dynamic>) {
      throw Exception('Invalid RPC transaction result.');
    }

    final status = (resultJson['status'] as String? ?? '').toLowerCase();
    if (status == 'failed') {
      final reason =
          _extractFailureReason(resultJson['changeset']) ??
          (resultJson['error_message'] as String?) ??
          'Escrow transaction failed on-chain.';
      resultJson['error_message'] = reason;
    }

    return TransactionResult.fromJson(resultJson);
  }

  Future<String> _findOwnedCoinObjectId({
    required String ownerAddress,
    required String tokenType,
  }) async {
    final account = await rpc.getAccount(ownerAddress);
    final availableCoinTypes = <String>{};

    print('[ESCROW] Searching for coin object:');
    print('[ESCROW]   Owner: $ownerAddress');
    print('[ESCROW]   Token Type: $tokenType');
    print(
      '[ESCROW]   Total owned objects: ${account.ownedObjects?.length ?? 0}',
    );

    for (final obj in account.ownedObjects ?? const <ObjectInfo>[]) {
      final objToken = _coinTokenFromObjectType(obj.type);
      if (objToken != null) {
        availableCoinTypes.add(objToken);
      }
      if (objToken == tokenType) {
        final normalizedId = _normalizeObjectId(obj.id);
        print('[ESCROW]   Found coin object: ${obj.id}');
        print('[ESCROW]   Normalized coin object ID: $normalizedId');
        print('[ESCROW]   Object type: ${obj.type}');
        print('[ESCROW]   Object version: ${obj.version}');
        return normalizedId;
      }
    }

    final trackedBalance = account.tokenBalances.entries
        .where((entry) => _normalizeTokenType(entry.key) == tokenType)
        .map((entry) => entry.value)
        .fold<int>(0, (sum, value) => sum + value);
    final nativeBalance = tokenType == kanariCoinType ? account.balance : 0;
    final availableTypesLabel = availableCoinTypes.isEmpty
        ? 'none'
        : availableCoinTypes.join(', ');

    print('[ESCROW]   No spendable coin object found!');
    print('[ESCROW]   Tracked balance: ${trackedBalance + nativeBalance}');
    print('[ESCROW]   Available coin types: $availableTypesLabel');

    throw Exception(
      'No spendable Coin<$tokenType> object was found in this wallet.\n'
      'This wallet has a balance of ${trackedBalance + nativeBalance}, but no actual Coin object exists.\n'
      'You may need to mint or transfer tokens to create a Coin object first.\n'
      'Tracked balance: ${trackedBalance + nativeBalance}\n'
      'Available coin object types: $availableTypesLabel',
    );
  }

  Future<EscrowObjectRefs> _getEscrowObjectRefs(String buyerAddress) async {
    final account = await rpc.getAccount(buyerAddress);
    final objects = [...?account.ownedObjects]
      ..sort((a, b) => b.version.compareTo(a.version));

    ObjectInfo? dealObject;
    ObjectInfo? proofObject;

    for (final obj in objects) {
      if (dealObject == null && _isEscrowDealObject(obj.type)) {
        dealObject = obj;
      } else if (proofObject == null && _isEscrowProofObject(obj.type)) {
        proofObject = obj;
      }
    }

    if (dealObject == null || proofObject == null) {
      throw Exception(
        'Escrow objects were not found under buyer address $buyerAddress.',
      );
    }

    final coinType = _escrowDealTokenFromObjectType(dealObject.type);
    if (coinType == null || coinType.isEmpty) {
      throw Exception(
        'Could not determine the escrow deal token type from object ${dealObject.id}.',
      );
    }

    return EscrowObjectRefs(
      dealObjectId: _normalizeObjectId(dealObject.id),
      proofObjectId: _normalizeObjectId(proofObject.id),
      coinType: coinType,
    );
  }

  String _normalizeObjectId(String objectId) {
    var clean = objectId.startsWith('0x') ? objectId.substring(2) : objectId;
    if (clean.isEmpty || !RegExp(r'^[0-9a-fA-F]+$').hasMatch(clean)) {
      throw ArgumentError('Invalid object ID format: $objectId');
    }
    clean = clean.padLeft(64, '0').toLowerCase();
    if (clean.length != 64) {
      throw ArgumentError(
        'Object ID must be 32 bytes (64 hex chars) after normalization. '
        'Got ${clean.length} characters for $objectId.',
      );
    }
    return '0x$clean';
  }

  String _normalizeTokenType(String tokenType) {
    final trimmed = tokenType.trim();
    if (trimmed.isEmpty) {
      throw ArgumentError('Token type is required.');
    }
    return trimmed;
  }

  String? _coinTokenFromObjectType(String objectType) {
    final start = objectType.indexOf('<');
    final end = objectType.lastIndexOf('>');
    if (start != -1 && end != -1) {
      final outer = objectType.substring(0, start);
      if (outer.endsWith('::coin::Coin') ||
          outer.endsWith('::coin::coin::Coin')) {
        return objectType.substring(start + 1, end);
      }
    }
    return null;
  }

  bool _isEscrowDealObject(String objectType) {
    return objectType.contains('::escrow::EscrowDeal<');
  }

  String? _extractCoinTypeFromObjectType(String objectType) {
    final start = objectType.indexOf('<');
    final end = objectType.lastIndexOf('>');
    if (start != -1 && end != -1) {
      return objectType.substring(start + 1, end);
    }
    return null;
  }

  bool _isEscrowProofObject(String objectType) {
    return objectType.contains('::escrow::EscrowProof');
  }

  String? _escrowDealTokenFromObjectType(String objectType) {
    final start = objectType.indexOf('::escrow::EscrowDeal<');
    if (start == -1) {
      return null;
    }
    final genericStart = objectType.indexOf('<', start);
    final genericEnd = objectType.lastIndexOf('>');
    if (genericStart == -1 || genericEnd == -1 || genericEnd <= genericStart) {
      return null;
    }
    return objectType.substring(genericStart + 1, genericEnd);
  }

  /// Get EscrowDeal objects by owner address (uses RPC query)
  Future<List<Map<String, dynamic>>> _getEscrowDealsByOwner(
    KanariWallet wallet,
    String ownerAddress,
  ) async {
    print('[ESCROW] Querying deals for owner: $ownerAddress');

    try {
      // Use RPC endpoint directly via HTTP POST
      final body = {
        'jsonrpc': '2.0',
        'method': 'kanari_getOwnedObjects',
        'params': {
          'owner': ownerAddress,
          'object_type':
              'escrow::EscrowDeal', // Simple filter works with any prefix
        },
        'id': DateTime.now().millisecondsSinceEpoch,
      };

      print('[ESCROW] RPC Request: $body');

      final response = await http.post(
        Uri.parse(rpc.url),
        headers: {'Content-Type': 'application/json'},
        body: jsonEncode(body),
      );

      print('[ESCROW] RPC Response Status: ${response.statusCode}');

      if (response.statusCode == 200) {
        print(
          '[ESCROW] RPC Response Body: ${response.body.substring(0, [200, response.body.length].reduce((a, b) => a < b ? a : b))}...',
        );

        final jsonResponse = jsonDecode(response.body) as Map<String, dynamic>;

        if (jsonResponse['result'] != null &&
            jsonResponse['result']['objects'] != null) {
          final objects = (jsonResponse['result']['objects'] as List)
              .cast<Map<String, dynamic>>();
          print('[ESCROW] Parsed ${objects.length} objects from RPC response');

          final deals = <Map<String, dynamic>>[];

          for (final obj in objects) {
            // Extract object ID and type
            final objectId = obj['id'] as String?;
            final objectType =
                obj['type_'] as String?; // Rust uses type_ (with underscore)

            if (objectId != null && objectType != null) {
              print('[ESCROW] Found escrow deal:');
              print('[ESCROW]   Object ID: $objectId');
              print('[ESCROW]   Object Type: $objectType');

              // Extract coin type from object type
              // Format: 0xPKG::escrow::EscrowDeal<0xPKG::usdc::USDC>
              final coinType = _extractCoinTypeFromObjectType(objectType);

              if (coinType != null) {
                deals.add({
                  'object_id': objectId,
                  'coin_type': coinType,
                  'object_type': objectType,
                });
                print('[ESCROW]   ✅ Extracted Coin Type: $coinType');
              } else {
                print(
                  '[ESCROW]   ⚠️ Could not extract coin type from: $objectType',
                );
              }
            }
          }

          print('[ESCROW] Found ${deals.length} escrow deals');
          if (deals.isNotEmpty) {
            print('[ESCROW] First object keys: ${deals.first.keys}');
          }

          return deals;
        }
      }

      print('[ESCROW] No escrow deals found');
      return [];
    } catch (e, stack) {
      print('[ESCROW] Error querying deals: $e');
      print('[ESCROW] Stack: $stack');
      return [];
    }
  }

  /// Find proof object for a given deal
  Future<String?> _findProofForDeal(
    String buyerAddress,
    String dealObjectId,
  ) async {
    try {
      print(
        '[ESCROW CLIENT] Finding proof for deal: ${dealObjectId.substring(0, 8)}...',
      );

      // Use getAccount which returns ownedObjects
      final account = await rpc.getAccount(buyerAddress);
      final objects = account.ownedObjects ?? [];

      for (final obj in objects) {
        final objectType = obj.type;
        if (_isEscrowProofObject(objectType)) {
          final objectId = obj.id;
          print(
            '[ESCROW CLIENT] Found EscrowProof object: ${objectId.substring(0, 8)}...',
          );

          // For now, assume the first proof object belongs to the deal
          // TODO: Verify by reading proof object data if needed
          return objectId;
        }
      }

      print('[ESCROW CLIENT] No EscrowProof object found for buyer');
      return null;
    } catch (e) {
      print('[ESCROW CLIENT] Error finding proof: $e');
      return null;
    }
  }

  List<int> _hexToBytes(String hexStr) {
    final clean = hexStr.startsWith('0x') ? hexStr.substring(2) : hexStr;
    final bytes = <int>[];
    for (var i = 0; i < clean.length; i += 2) {
      bytes.add(int.parse(clean.substring(i, i + 2), radix: 16));
    }
    return bytes;
  }

  List<int> _encodeU64Bcs(int value) {
    final data = ByteData(8);
    data.setUint64(0, value, Endian.little);
    return data.buffer.asUint8List().toList();
  }

  List<int> _encodeStringBcs(String value) {
    return Bcs.string().serialize(value).toBytes().toList();
  }

  List<int> _encodeAddressBcs(String address) {
    final clean = address.startsWith('0x') ? address.substring(2) : address;
    final padded = clean.padLeft(64, '0');
    return _hexToBytes(padded);
  }

  String _normalizeAddress(String addr) {
    final clean = addr.startsWith('0x') ? addr.substring(2) : addr;
    if (!RegExp(r'^[0-9a-fA-F]+$').hasMatch(clean)) {
      throw ArgumentError('Invalid hexadecimal characters in address: $clean');
    }
    if (clean.length != 64) {
      throw ArgumentError(
        'Address must be exactly 64 hex characters (32 bytes). Got ${clean.length} characters.',
      );
    }
    return '0x${clean.toLowerCase()}';
  }

  String _getStateName(int state) {
    switch (state) {
      case 1:
        return 'LOCKED (1)';
      case 2:
        return 'DELIVERED (2)';
      case 3:
        return 'COMPLETED (3)';
      case 4:
        return 'DISPUTED (4)';
      default:
        return 'UNKNOWN ($state)';
    }
  }

  String? _extractFailureReason(dynamic changeset) {
    final hints = <String>{};

    void visit(dynamic value, [String? key]) {
      if (value is Map) {
        for (final entry in value.entries) {
          visit(entry.value, entry.key.toString());
        }
        return;
      }
      if (value is List) {
        for (final item in value) {
          visit(item, key);
        }
        return;
      }
      if (value is String) {
        final normalizedKey = key?.toLowerCase() ?? '';
        final normalizedValue = value.trim();
        if (normalizedValue.isEmpty) return;
        if (_looksLikeFailureKey(normalizedKey) ||
            _looksLikeFailureValue(normalizedValue)) {
          hints.add(key == null ? normalizedValue : '$key: $normalizedValue');
        }
      }
      if (value is num) {
        final normalizedKey = key?.toLowerCase() ?? '';
        if (normalizedKey.contains('abort') ||
            normalizedKey.contains('error') ||
            normalizedKey.contains('status')) {
          hints.add(key == null ? '$value' : '$key: $value');
        }
      }
    }

    visit(changeset);
    if (hints.isNotEmpty) {
      return hints.join(' | ');
    }
    if (changeset == null) {
      return null;
    }
    final compact = jsonEncode(changeset);
    if (compact.length <= 320) {
      return compact;
    }
    return '${compact.substring(0, 320)}...';
  }

  bool _looksLikeFailureKey(String key) {
    return key.contains('abort') ||
        key.contains('error') ||
        key.contains('fail') ||
        key.contains('message') ||
        key.contains('status') ||
        key.contains('reason') ||
        key.contains('vm');
  }

  bool _looksLikeFailureValue(String value) {
    final lower = value.toLowerCase();
    return lower.contains('abort') ||
        lower.contains('error') ||
        lower.contains('fail') ||
        lower.contains('invalid') ||
        lower.contains('panic') ||
        lower.contains('out of gas') ||
        lower.contains('insufficient');
  }
}
