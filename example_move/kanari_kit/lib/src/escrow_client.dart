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

  Future<TransactionResult> confirmDelivery({
    required KanariWallet wallet,
    required String buyerAddress,
    int gasLimit = 100000,
    int gasPrice = 10,
  }) async {
    final refs = await _getEscrowObjectRefs(buyerAddress);
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
    int gasLimit = 100000,
    int gasPrice = 10,
  }) async {
    final refs = await _getEscrowObjectRefs(buyerAddress);
    final result = await _executeFunctionDetailed(
      wallet: wallet,
      package: escrowPackageAddress,
      module: 'escrow',
      function: 'raise_dispute',
      typeArgs: [refs.coinType],
      args: [_hexToBytes(refs.dealObjectId), _hexToBytes(refs.proofObjectId)],
      gasLimit: gasLimit,
      gasPrice: gasPrice,
      executeImmediate: true,
    );
    return _requireSuccess(result);
  }

  Future<int> getDealState(String buyerAddress) async {
    final refs = await _getEscrowObjectRefs(buyerAddress);
    final result = await _viewFunction(
      function: '$escrowPackageAddress::escrow::get_state',
      typeArguments: [refs.coinType],
      arguments: [_hexToBytes(refs.dealObjectId)],
    );
    return result.isNotEmpty ? (result.first as int) : 0;
  }

  Future<int> getProofCount(String buyerAddress) async {
    final refs = await _getEscrowObjectRefs(buyerAddress);
    final result = await _viewFunction(
      function: '$escrowPackageAddress::escrow::get_proof_count',
      typeArguments: const [],
      arguments: [_hexToBytes(refs.proofObjectId)],
    );
    return result.isNotEmpty ? (result.first as int) : 0;
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
    required String function,
    required List<String> typeArguments,
    required List<List<int>> arguments,
  }) async {
    final body = {
      'jsonrpc': '2.0',
      'method': 'kanari_view',
      'params': {
        'function': function,
        'type_arguments': typeArguments,
        'arguments': arguments,
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

    return jsonResponse['result'] as List<dynamic>? ?? const [];
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
