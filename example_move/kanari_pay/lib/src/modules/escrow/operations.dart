// modules/escrow/operations.dart
/// Escrow transaction operations

import '../../core/bcs_serializers.dart';
import '../../client/kanari_client.dart';
import '../../kanari_wallet.dart';
import '../../models/transaction.dart';
import 'constants.dart';

class EscrowOperations {
  final KanariClient rpc;

  const EscrowOperations(this.rpc);

  /// Validate transaction result
  TransactionResult requireSuccess(TransactionResult result) {
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

  /// Create new escrow deal
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
    final normalizedToken = _normalizeTokenType(tokenType);

    print('[ESCROW] Creating deal:');
    print('[ESCROW]   Wallet: ${wallet.address}');
    print('[ESCROW]   Token: $normalizedToken');
    print('[ESCROW]   Amount: $amount');
    print('[ESCROW]   Seller: $sellerAddress');
    print('[ESCROW]   Deal ID: $dealId');
    print('[ESCROW]   Description: $description');

    try {
      //  CRITICAL: Find owned Coin object for this token type
      final coinObjectId = await _findOwnedCoinObjectId(
        client: rpc,
        ownerAddress: wallet.address,
        tokenType: normalizedToken,
      );

      final normalizedCoinObjectId = _normalizeObjectId(coinObjectId);
      print('[ESCROW]   Coin Object ID: $coinObjectId');
      print('[ESCROW]   Normalized Coin Object ID: $normalizedCoinObjectId');

      // Move VM expects raw 32-byte object ID for &mut Coin reference
      final coinObjectBytes = _hexToBytes(normalizedCoinObjectId);
      print('[ESCROW]   Coin Object bytes length: ${coinObjectBytes.length}');

      final args = [
        BcsSerializers.encodeString(dealId),
        BcsSerializers.hexToBytes(
          BcsSerializers.normalizeAddress(sellerAddress),
        ),
        _u64ToBytes(amount),
        BcsSerializers.encodeString(description),
        coinObjectBytes, // buyer_coin: &mut Coin<CoinType>
      ];

      print('[ESCROW] Encoded args:');
      print('[ESCROW]   dealId bytes: ${args[0]}');
      print('[ESCROW]   sellerAddress bytes: ${args[1]}');
      print('[ESCROW]   amount bytes: ${args[2]}');
      print('[ESCROW]   description bytes: ${args[3]}');
      print('[ESCROW]   coinObject bytes: ${args[4]}');
      print('[ESCROW]   Total args: 5');

      final result = await rpc.executeFunction(
        wallet: wallet,
        package: EscrowConstants.packageAddress,
        module: EscrowConstants.module,
        function: EscrowConstants.fnCreateDeal,
        typeArgs: [normalizedToken],
        args: args,
        gasLimit: gasLimit,
        gasPrice: gasPrice,
      );

      print('[ESCROW] Transaction result: ${result.status}');
      print('[ESCROW]   Hash: ${result.hash}');
      print('[ESCROW]   Gas used: ${result.gasUsed}');
      if (result.errorMessage != null) {
        print('[ESCROW]   Error: ${result.errorMessage}');
      } else if (result.status.toLowerCase() == 'failed') {
        print('[ESCROW]   ⚠️ Transaction failed but no error message returned');
        print(
          '[ESCROW]   This may indicate a Move VM abort without error details',
        );
      }

      return result;
    } catch (e, stackTrace) {
      print('[ESCROW] ERROR creating deal: $e');
      print('[ESCROW] Stack trace: $stackTrace');
      rethrow;
    }
  }

  /// Find owned Coin object for a specific token type
  Future<String> _findOwnedCoinObjectId({
    required KanariClient client,
    required String ownerAddress,
    required String tokenType,
  }) async {
    print('[ESCROW] Searching for coin object:');
    print('[ESCROW]   Owner: $ownerAddress');
    print('[ESCROW]   Token Type: $tokenType');

    final account = await client.getAccount(ownerAddress);
    print(
      '[ESCROW]   Total owned objects: ${account.ownedObjects?.length ?? 0}',
    );

    for (final obj in account.ownedObjects ?? const []) {
      final objToken = _coinTokenFromObjectType(obj.type);
      if (objToken != null && objToken == tokenType) {
        final normalizedId = _normalizeObjectId(obj.id);
        print('[ESCROW]   Found coin object: ${obj.id}');
        print('[ESCROW]   Object type: ${obj.type}');
        return normalizedId;
      }
    }

    throw Exception(
      'No spendable Coin<$tokenType> object found in wallet.\n'
      'You need to have actual Coin objects, not just balance.\n'
      'Try minting or transferring tokens first.',
    );
  }

  /// Extract coin token type from object type string
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

  /// Normalize object ID to standard format
  String _normalizeObjectId(String objectId) {
    var clean = objectId.startsWith('0x') ? objectId.substring(2) : objectId;
    if (clean.isEmpty || !RegExp(r'^[0-9a-fA-F]+$').hasMatch(clean)) {
      throw ArgumentError('Invalid object ID format: $objectId');
    }
    clean = clean.padLeft(64, '0').toLowerCase();
    if (clean.length != 64) {
      throw ArgumentError(
        'Object ID must be 32 bytes (64 hex chars). Got ${clean.length} characters.',
      );
    }
    return '0x$clean';
  }

  /// Convert hex string to bytes
  List<int> _hexToBytes(String hexStr) {
    final clean = hexStr.startsWith('0x') ? hexStr.substring(2) : hexStr;
    final bytes = <int>[];
    for (var i = 0; i < clean.length; i += 2) {
      bytes.add(int.parse(clean.substring(i, i + 2), radix: 16));
    }
    return bytes;
  }

  /// Confirm delivery (Seller only)
  Future<TransactionResult> confirmDelivery({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
    required String proofObjectId,
    int gasLimit = 100000,
    int gasPrice = 10,
  }) async {
    print('[ESCROW] Confirming delivery:');
    print('[ESCROW]   Deal: $dealObjectId');
    print('[ESCROW]   Proof: $proofObjectId');

    return _executeEscrowAction(
      wallet: wallet,
      coinType: coinType,
      functionName: EscrowConstants.fnConfirmDelivery,
      args: [
        BcsSerializers.hexToBytes(dealObjectId),
        _prepareProofObject(proofObjectId),
      ],
      actionName: 'Confirming delivery',
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  /// Release funds (Buyer only)
  Future<TransactionResult> releaseFunds({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
    required String proofObjectId,
    int gasLimit = 100000,
    int gasPrice = 10,
  }) async {
    print('[ESCROW] Releasing funds: $dealObjectId');
    print('[ESCROW]   Proof: $proofObjectId');

    return _executeEscrowAction(
      wallet: wallet,
      coinType: coinType,
      functionName: EscrowConstants.fnReleaseFunds,
      args: [
        BcsSerializers.hexToBytes(dealObjectId),
        _prepareProofObject(proofObjectId),
      ],
      actionName: 'Releasing funds',
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  /// Raise dispute
  Future<TransactionResult> raiseDispute({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
    required String reason,
    required String proofObjectId,
    int gasLimit = 100000,
    int gasPrice = 10,
  }) async {
    print('[ESCROW] Raising dispute: $dealObjectId');
    print('[ESCROW]   Proof: $proofObjectId');

    return _executeEscrowAction(
      wallet: wallet,
      coinType: coinType,
      functionName: EscrowConstants.fnRaiseDispute,
      args: [
        BcsSerializers.hexToBytes(dealObjectId),
        _prepareProofObject(proofObjectId),
        BcsSerializers.encodeString(reason),
      ],
      actionName: 'Raising dispute',
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  /// Generic escrow action executor
  Future<TransactionResult> _executeEscrowAction({
    required KanariWallet wallet,
    required String coinType,
    required String functionName,
    required List<List<int>> args,
    required String actionName,
    int gasLimit = 100000,
    int gasPrice = 10,
  }) {
    return rpc.executeFunction(
      wallet: wallet,
      package: EscrowConstants.packageAddress,
      module: EscrowConstants.module,
      function: functionName,
      typeArgs: [_normalizeTokenType(coinType)],
      args: args,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  /// Prepare proof object for transaction
  List<int> _prepareProofObject(String proofObjectId) {
    final normalizedId = _normalizeObjectId(proofObjectId);
    final proofBytes = _hexToBytes(normalizedId);

    print(
      '[ESCROW]   Normalized Proof: $normalizedId (${proofBytes.length} bytes)',
    );

    return proofBytes;
  }

  /// Normalize token type
  String _normalizeTokenType(String tokenType) {
    if (tokenType.startsWith('0x')) return tokenType;
    return '0x$tokenType';
  }

  /// Convert u64 to bytes (little endian)
  List<int> _u64ToBytes(int value) {
    final bytes = <int>[];
    for (var i = 0; i < 8; i++) {
      bytes.add((value >> (i * 8)) & 0xFF);
    }
    return bytes;
  }
}
