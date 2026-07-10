// modules/escrow/queries.dart
// Escrow view functions and queries.

import 'dart:convert';
import 'package:http/http.dart' as http;

import '../../client/kanari_client.dart';
import '../../core/bcs_utils.dart';
import '../../kanari_wallet.dart';
import '../../models/account.dart';
import '../../models/transaction.dart';
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
      args: TransactionArgs(),
      objectInputId: dealObjectId,
    );

    if (result.isEmpty) return 0;

    return _jsonInt(_unwrapViewResult(result.first));
  }

  /// Get all deals for buyer
  Future<List<Map<String, dynamic>>> getAllDeals({
    required KanariWallet wallet,
    required String buyerAddress,
  }) async {
    final allObjects = await _getEscrowCandidateObjects(buyerAddress);

    final dealObjects = allObjects
        .where((obj) => _isEscrowDealType(obj.type))
        .toList();

    final proofObjects = allObjects
        .where((obj) => _isEscrowProofType(obj.type))
        .toList();

    final deals = <Map<String, dynamic>>[];

    for (final obj in dealObjects) {
      final objectId = obj.id;
      final objectType = obj.type;
      final coinType = BcsUtils.extractCoinTypeFromObjectType(objectType);

      if (coinType == null) {
        continue;
      }

      Map<String, dynamic> dealDetails;
      int? state;
      try {
        dealDetails = await getDealDetailsByObjectId(
          wallet: wallet,
          dealObjectId: objectId,
          coinType: coinType,
        );
        state = await getDealStateByObjectId(
          wallet: wallet,
          dealObjectId: objectId,
          coinType: coinType,
        );
      } catch (_) {
        final decoded = _decodeDealObjectData(obj);
        if (decoded == null) {
          // Object indexes can briefly expose stale or partially indexed escrow
          // candidates. Skip unreadable candidates instead of failing the screen.
          continue;
        }
        dealDetails = decoded;
        state = decoded['state'] as int?;
      }

      final proofId = proofObjects.isEmpty ? null : proofObjects.first.id;

      deals.add({
        'object_id': objectId,
        'coin_type': coinType,
        'object_type': objectType,
        'proof_id': proofId,
        'state': state,
        ...dealDetails,
      });
    }

    return deals;
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
      args: TransactionArgs(),
      objectInputId: dealObjectId,
    );

    if (result.isEmpty) {
      return {};
    }

    final resultValue = _unwrapViewResult(result.first);
    if (resultValue is List && resultValue.length >= 4) {
      return {
        'deal_id': resultValue[0].toString(),
        'buyer': resultValue[1].toString(),
        'seller': resultValue[2].toString(),
        'amount': _jsonInt(resultValue[3]),
      };
    }

    return {};
  }

  Future<Map<String, dynamic>?> getDealFromEffects({
    required KanariWallet wallet,
    required TransactionEffectsInfo effects,
    required String buyerAddress,
    required String fallbackCoinType,
  }) async {
    final dealChanges = effects.objectChanges
        .where((change) => _isEscrowDealType(change.objectType ?? ''))
        .toList();
    final proofChanges = effects.objectChanges
        .where((change) => _isEscrowProofType(change.objectType ?? ''))
        .toList();

    for (final dealChange in dealChanges) {
      final objectId = dealChange.objectRef.objectId;
      if (objectId.isEmpty) continue;

      final owner = dealChange.owner;
      if (owner != null &&
          owner.isNotEmpty &&
          !_sameAddress(owner, buyerAddress)) {
        continue;
      }

      final objectType = dealChange.objectType ?? '';
      final coinType =
          BcsUtils.extractCoinTypeFromObjectType(objectType) ?? fallbackCoinType;

      try {
        final details = await getDealDetailsByObjectId(
          wallet: wallet,
          dealObjectId: objectId,
          coinType: coinType,
        );
        final state = await getDealStateByObjectId(
          wallet: wallet,
          dealObjectId: objectId,
          coinType: coinType,
        );
        final proofId = _matchingProofId(proofChanges, buyerAddress);

        return {
          'object_id': objectId,
          'coin_type': coinType,
          'object_type': objectType,
          'proof_id': proofId,
          'state': state,
          ...details,
        };
      } catch (_) {
        // The transaction may still be pending in consensus; object index fallback can pick it up later.
      }
    }

    return null;
  }

  Future<List<ObjectInfo>> _getEscrowCandidateObjects(String buyerAddress) async {
    final byId = <String, ObjectInfo>{};

    void addAll(Iterable<ObjectInfo> objects) {
      for (final obj in objects) {
        if (obj.id.isNotEmpty) {
          byId[obj.id.toLowerCase()] = obj;
        }
      }
    }

    addAll(await rpc.getOwnedObjects(buyerAddress));

    try {
      addAll(await rpc.getObjects(owner: buyerAddress));
    } catch (_) {
      // Older nodes may not expose kanari_getObjects; owned objects above still works.
    }

    if (!byId.values.any((obj) => _isEscrowDealType(obj.type))) {
      try {
        final indexedObjects = await rpc.getObjects();
        addAll(
          indexedObjects.where(
            (obj) =>
                _sameAddress(obj.owner, buyerAddress) &&
                (_isEscrowDealType(obj.type) || _isEscrowProofType(obj.type)),
          ),
        );
      } catch (_) {
        // Best-effort index fallback only.
      }
    }

    return byId.values.toList();
  }

  bool _isEscrowDealType(String type) {
    return type.contains(
          '::${EscrowConstants.module}::${EscrowConstants.objectTypeDeal}<',
        ) ||
        type.contains('${EscrowConstants.objectTypeDeal}<');
  }

  bool _isEscrowProofType(String type) {
    return type.contains(
          '::${EscrowConstants.module}::${EscrowConstants.objectTypeProof}',
        ) ||
        type.contains(EscrowConstants.objectTypeProof);
  }

  bool _sameAddress(String left, String right) {
    try {
      return BcsUtils.normalizeAddress(left) == BcsUtils.normalizeAddress(right);
    } catch (_) {
      return left.toLowerCase() == right.toLowerCase();
    }
  }

  Map<String, dynamic>? _decodeDealObjectData(ObjectInfo object) {
    final data = object.data;
    var offset = 0;

    List<int>? readBytes(int length) {
      if (length < 0 || offset + length > data.length) return null;
      final bytes = data.sublist(offset, offset + length);
      offset += length;
      return bytes;
    }

    int? readUleb128() {
      var value = 0;
      var shift = 0;
      while (offset < data.length) {
        final byte = data[offset++];
        value |= (byte & 0x7f) << shift;
        if ((byte & 0x80) == 0) return value;
        shift += 7;
        if (shift > 28) return null;
      }
      return null;
    }

    String? readString() {
      final length = readUleb128();
      if (length == null) return null;
      final bytes = readBytes(length);
      if (bytes == null) return null;
      return utf8.decode(bytes, allowMalformed: true);
    }

    int? readU64() {
      final bytes = readBytes(8);
      if (bytes == null) return null;
      var value = 0;
      for (var i = 0; i < bytes.length; i++) {
        value |= bytes[i] << (8 * i);
      }
      return value;
    }

    final objectIdBytes = readBytes(32);
    if (objectIdBytes == null) return null;

    final dealId = readString();
    final buyerBytes = readBytes(32);
    final sellerBytes = readBytes(32);
    final amount = readU64();
    final description = readString();
    if (dealId == null ||
        buyerBytes == null ||
        sellerBytes == null ||
        amount == null ||
        description == null ||
        offset >= data.length) {
      return null;
    }

    final state = data[offset++];

    return {
      'deal_id': dealId,
      'buyer': BcsUtils.bytesToHex(buyerBytes),
      'seller': BcsUtils.bytesToHex(sellerBytes),
      'amount': amount,
      'description': description,
      'state': state,
    };
  }

  dynamic _unwrapViewResult(dynamic value) {
    if (value is Map<String, dynamic> && value.containsKey('result')) {
      return value['result'];
    }
    return value;
  }

  int _jsonInt(dynamic value) {
    if (value is int) return value;
    if (value is num) return value.toInt();
    if (value is String) return int.tryParse(value) ?? 0;
    return 0;
  }

  String? _matchingProofId(
    List<ObjectChangeInfo> proofChanges,
    String buyerAddress,
  ) {
    for (final proofChange in proofChanges) {
      final owner = proofChange.owner;
      if (owner != null &&
          owner.isNotEmpty &&
          !_sameAddress(owner, buyerAddress)) {
        continue;
      }
      final objectId = proofChange.objectRef.objectId;
      if (objectId.isNotEmpty) return objectId;
    }
    return null;
  }

  /// Generic view function caller
  Future<List<dynamic>> _callViewFunction({
    required KanariWallet wallet,
    required String functionName,
    required String coinType,
    required TransactionArgs args,
    String? objectInputId,
  }) async {
    final normalizedToken = BcsUtils.normalizeTokenType(coinType);
    final packageAddr = BcsUtils.normalizeAddress(
      EscrowConstants.packageAddress,
    );
    final objectInputs = objectInputId == null
        ? null
        : [await _objectInputForView(objectInputId)];

    return _viewFunction(
      wallet: wallet,
      function: '$packageAddr::${EscrowConstants.module}::$functionName',
      typeArguments: [normalizedToken],
      arguments: args.build(),
      objectInputs: objectInputs,
    );
  }

  Future<Map<String, dynamic>> _objectInputForView(String objectId) async {
    final object = await rpc.getObject(objectId);
    final objectRef = <String, dynamic>{
      'object_id': BcsUtils.normalizeObjectId(object.id),
      'version': object.version,
      if (object.digest != null && object.digest!.isNotEmpty)
        'digest': object.digest,
    };

    return {
      'object_ref': objectRef,
      'owner': {'AddressOwner': BcsUtils.normalizeAddress(object.owner)},
      'mutable': false,
    };
  }

  /// Execute view function via RPC
  Future<List<dynamic>> _viewFunction({
    required KanariWallet wallet,
    required String function,
    required List<String> typeArguments,
    required List<List<int>> arguments,
    List<Map<String, dynamic>>? objectInputs,
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
      if (objectInputs != null && objectInputs.isNotEmpty)
        'object_inputs': objectInputs,
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
