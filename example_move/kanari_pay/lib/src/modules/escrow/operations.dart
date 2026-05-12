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

    final args = [
      BcsSerializers.hexToBytes(dealId),
      BcsSerializers.hexToBytes(sellerAddress),
      _u64ToBytes(amount),
      BcsSerializers.hexToBytes(description),
    ];

    return rpc.executeFunction(
      wallet: wallet,
      package: EscrowConstants.packageAddress,
      module: EscrowConstants.module,
      function: EscrowConstants.fnCreateDeal,
      typeArgs: [normalizedToken],
      args: args,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  /// Confirm delivery (Seller only)
  Future<TransactionResult> confirmDelivery({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
    int gasLimit = 100000,
    int gasPrice = 10,
  }) async {
    print('[ESCROW] Confirming delivery: $dealObjectId');

    return rpc.executeFunction(
      wallet: wallet,
      package: EscrowConstants.packageAddress,
      module: EscrowConstants.module,
      function: EscrowConstants.fnConfirmDelivery,
      typeArgs: [_normalizeTokenType(coinType)],
      args: [BcsSerializers.hexToBytes(dealObjectId)],
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  /// Release funds (Buyer only)
  Future<TransactionResult> releaseFunds({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
    int gasLimit = 100000,
    int gasPrice = 10,
  }) async {
    print('[ESCROW] Releasing funds: $dealObjectId');

    return rpc.executeFunction(
      wallet: wallet,
      package: EscrowConstants.packageAddress,
      module: EscrowConstants.module,
      function: EscrowConstants.fnReleaseFunds,
      typeArgs: [_normalizeTokenType(coinType)],
      args: [BcsSerializers.hexToBytes(dealObjectId)],
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
    int gasLimit = 100000,
    int gasPrice = 10,
  }) async {
    print('[ESCROW] Raising dispute: $dealObjectId');

    return rpc.executeFunction(
      wallet: wallet,
      package: EscrowConstants.packageAddress,
      module: EscrowConstants.module,
      function: EscrowConstants.fnRaiseDispute,
      typeArgs: [_normalizeTokenType(coinType)],
      args: [
        BcsSerializers.hexToBytes(dealObjectId),
        BcsSerializers.hexToBytes(reason),
      ],
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
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
