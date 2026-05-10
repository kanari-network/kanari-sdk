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
  }) async {
    final result = await _executeFunctionDetailed(
      wallet: wallet,
      package: escrowPackageAddress,
      module: 'escrow',
      function: 'confirm_delivery',
      typeArgs: [coinType],
      args: [_hexToBytes(dealObjectId)],
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
  }) async {
    final result = await _executeFunctionDetailed(
      wallet: wallet,
      package: escrowPackageAddress,
      module: 'escrow',
      function: 'release_funds',
      typeArgs: [coinType],
      args: [_hexToBytes(dealObjectId)],
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
  }) async {
    final result = await _executeFunctionDetailed(
      wallet: wallet,
      package: escrowPackageAddress,
      module: 'escrow',
      function: 'raise_dispute',
      typeArgs: [coinType],
      args: [_hexToBytes(dealObjectId)],
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
    final result = await _viewFunction(
      wallet: wallet,
      function: '$escrowPackageAddress::escrow::get_state',
      typeArguments: [coinType],
      arguments: [_hexToBytes(dealObjectId)],
    );

    if (result.isNotEmpty && result.first is int) {
      return result.first as int;
    }
    return 0;
  }

  /// Get full deal details by Object ID
  Future<Map<String, dynamic>> getDealDetailsByObjectId({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
  }) async {
    final result = await _viewFunction(
      wallet: wallet,
      function: '$escrowPackageAddress::escrow::get_deal_details',
      typeArguments: [coinType],
      arguments: [_hexToBytes(dealObjectId)],
    );

    if (result.isNotEmpty && result.first is Map) {
      return result.first as Map<String, dynamic>;
    }
    return {};
  }

  /// Get deal state by buyer address (backward compatible)
  Future<int> getDealState({
    required KanariWallet wallet,
    required String buyerAddress,
  }) async {
    final refs = await _getEscrowObjectRefs(buyerAddress);
    return getDealStateByObjectId(
      wallet: wallet,
      dealObjectId: refs.dealObjectId,
      coinType: refs.coinType,
    );
  }

  /// Get full deal details by buyer address (backward compatible)
  Future<Map<String, dynamic>> getDealDetails({
    required KanariWallet wallet,
    required String buyerAddress,
  }) async {
    final refs = await _getEscrowObjectRefs(buyerAddress);
    final result = await _viewFunction(
      wallet: wallet,
      function: '$escrowPackageAddress::escrow::get_deal_details',
      typeArguments: [refs.coinType],
      arguments: [_hexToBytes(refs.dealObjectId)],
    );

    // Parse from DealCreated event emitted by get_deal_details
    if (result.isNotEmpty && result.first is Map) {
      return result.first as Map<String, dynamic>;
    }
    return {};
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

            if (result.isNotEmpty && result.first is Map) {
              final dealData = result.first as Map<String, dynamic>;
              dealData['object_id'] = dealObjectId;
              dealData['coin_type'] = coinType;
              deals.add(dealData);
            }
          } catch (_) {
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

    final account = await rpc.getAccount(wallet.address);
    final sequenceNumber = account.sequenceNumber;
    final senderAddress = wallet.taggedAddress;
    final packageAddress = _normalizeAddress(package);

    final serializedTx = _transactionBcs.serialize({
      'ExecuteFunction': {
        'sender': senderAddress,
        'module': '$packageAddress::$module',
        'function': functionName,
        'type_args': typeArguments,
        'args': arguments,
        'gas_limit': 100000, // Default gas limit for view calls
        'gas_price': 10,
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
        'function': functionName,
        'type_args': typeArguments,
        'args': arguments,
        'gas_limit': 100000,
        'gas_price': 10,
        'sequence_number': sequenceNumber,
        'signature': signature.toList(),
        'execute_immediate': true,
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
          'View function call failed on-chain.';
      throw Exception(reason);
    }

    // For view functions, extract return values from events
    final events = resultJson['changeset']?['events'] as List<dynamic>? ?? [];

    // For get_state: parse state from DealStateChanged event
    if (functionName == 'get_state' && events.isNotEmpty) {
      final event = events.first as Map<String, dynamic>?;
      if (event != null) {
        final eventData = event['event_data'] as List<dynamic>?;
        if (eventData != null && eventData.length >= 12) {
          // event_data format: [length, ...deal_id_bytes, old_state, new_state, actor, timestamp]
          // Find deal_id length first
          final dealIdLength = eventData[0] as int? ?? 0;
          final oldStateIndex = 1 + dealIdLength;

          if (oldStateIndex < eventData.length) {
            final oldState = eventData[oldStateIndex] as int?;
            if (oldState != null) {
              return [oldState];
            }
          }
        }
      }
    }

    // For get_proof_count: parse count from proof object
    if (functionName == 'get_proof_count') {
      // Since this is a read-only operation using borrow_global,
      // we need to fetch the actual object data from RPC
      // For now, return empty and let caller handle it
      return [];
    }

    // For get_deal_details: parse from DealCreated event
    if (functionName == 'get_deal_details' && events.isNotEmpty) {
      final event = events.first as Map<String, dynamic>?;
      if (event != null) {
        final eventData = event['event_data'] as List<dynamic>?;
        if (eventData != null && eventData.length >= 4) {
          // DealCreated struct: { deal_id: String, buyer: address, seller: address, amount: u64 }
          // BCS event_data format: [deal_id_length, deal_id_bytes..., buyer_address(32), seller_address(32), amount(8 bytes LE)]

          var offset = 0;

          // 1. Parse deal_id (String with length prefix)
          final dealIdLength = eventData[offset++] as int? ?? 0;
          if (offset + dealIdLength > eventData.length) {
            return [];
          }
          final dealIdBytes = eventData
              .sublist(offset, offset + dealIdLength)
              .cast<int>();
          final dealId = String.fromCharCodes(dealIdBytes);
          offset += dealIdLength;

          // 2. Parse buyer address (32 bytes)
          if (offset + 32 > eventData.length) {
            return [];
          }
          final buyerBytes = eventData.sublist(offset, offset + 32).cast<int>();
          final buyer = _bytesToAddress(buyerBytes);
          offset += 32;

          // 3. Parse seller address (32 bytes)
          if (offset + 32 > eventData.length) {
            return [];
          }
          final sellerBytes = eventData
              .sublist(offset, offset + 32)
              .cast<int>();
          final seller = _bytesToAddress(sellerBytes);
          offset += 32;

          // 4. Parse amount (u64, 8 bytes little-endian)
          if (offset + 8 > eventData.length) {
            return [];
          }
          final amountBytes = eventData.sublist(offset, offset + 8).cast<int>();
          final amount = _bytesToU64(amountBytes);

          print('[ESCROW] Parsed deal details:');
          print('[ESCROW]   Deal ID: $dealId');
          print('[ESCROW]   Buyer: $buyer');
          print('[ESCROW]   Seller: $seller');
          print('[ESCROW]   Amount: $amount');

          return [
            {
              'deal_id': dealId,
              'buyer': buyer,
              'seller': seller,
              'amount': amount,
            },
          ];
        }
      }
    }

    // For check_deal_state: parse boolean result
    if (functionName == 'check_deal_state' && events.isNotEmpty) {
      final event = events.first as Map<String, dynamic>?;
      if (event != null) {
        final eventData = event['event_data'] as List<dynamic>?;
        if (eventData != null && eventData.length >= 12) {
          final dealIdLength = eventData[0] as int? ?? 0;
          final oldStateIndex = 1 + dealIdLength;
          final newStateIndex = oldStateIndex + 1;

          if (newStateIndex < eventData.length) {
            final newState = eventData[newStateIndex] as int?;
            if (newState != null) {
              // If new_state > 0, it means state matched
              return [newState > 0 ? 1 : 0];
            }
          }
        }
      }
    }

    return [];
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
    return objectType.substring(genericStart + 1, genericEnd).trim();
  }

  List<int> _hexToBytes(String hexStr) {
    final clean = hexStr.startsWith('0x') ? hexStr.substring(2) : hexStr;
    final bytes = <int>[];
    for (var i = 0; i < clean.length; i += 2) {
      bytes.add(int.parse(clean.substring(i, i + 2), radix: 16));
    }
    return bytes;
  }

  String _bytesToAddress(List<int> bytes) {
    if (bytes.length != 32) {
      throw Exception('Invalid address bytes length: ${bytes.length}');
    }
    final hex = bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();
    return '0x$hex';
  }

  int _bytesToU64(List<int> bytes) {
    if (bytes.length != 8) {
      throw Exception('Invalid u64 bytes length: ${bytes.length}');
    }
    final data = ByteData.view(Uint8List.fromList(bytes).buffer);
    return data.getUint64(0, Endian.little);
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
