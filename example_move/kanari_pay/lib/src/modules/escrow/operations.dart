// modules/escrow/operations.dart
/// Escrow transaction operations

import '../../client/kanari_client.dart';
import '../../core/bcs_utils.dart';
import '../../kanari_wallet.dart';
import '../../models/transaction.dart';
import 'constants.dart';

class EscrowOperations {
  final KanariClient rpc;

  const EscrowOperations(this.rpc);

  /// Validate transaction result
  TransactionResult requireSuccess(TransactionResult result) {
    if (result.status.toLowerCase() == 'failed') {
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
    final normalizedToken = BcsUtils.normalizeTokenType(tokenType);

    print('[ESCROW] Creating deal:');
    print('[ESCROW]   Wallet: ${wallet.address}');
    print('[ESCROW]   Token: $normalizedToken');
    print('[ESCROW]   Amount: $amount');
    print('[ESCROW]   Seller: $sellerAddress');
    print('[ESCROW]   Deal ID: $dealId');
    print('[ESCROW]   Description: $description');

    try {
      // CRITICAL: Find owned Coin object for this token type
      final coinObjectId = await _findOwnedCoinObjectId(
        ownerAddress: wallet.address,
        tokenType: normalizedToken,
      );

      print('[ESCROW]   Coin Object ID: $coinObjectId');

      // Build args using TransactionArgs builder
      final args = TransactionArgs()
        ..addString(dealId)
        ..addAddress(sellerAddress)
        ..addAmount(amount)
        ..addString(description)
        ..addObjectId(coinObjectId);

      print('[ESCROW] Encoded args: ${args.length} total');

      final result = await rpc.executeFunction(
        wallet: wallet,
        package: EscrowConstants.packageAddress,
        module: EscrowConstants.module,
        function: EscrowConstants.fnCreateDeal,
        typeArgs: [normalizedToken],
        args: args.build(),
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
    required String ownerAddress,
    required String tokenType,
  }) async {
    print('[ESCROW] Searching for coin object:');
    print('[ESCROW]   Owner: $ownerAddress');
    print('[ESCROW]   Token Type: $tokenType');

    final account = await rpc.getAccount(ownerAddress);
    print(
      '[ESCROW]   Total owned objects: ${account.ownedObjects?.length ?? 0}',
    );

    for (final obj in account.ownedObjects ?? const []) {
      final objToken = BcsUtils.extractCoinTypeFromObjectType(obj.type);
      if (objToken != null && objToken == tokenType) {
        final normalizedId = BcsUtils.normalizeObjectId(obj.id);
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

  /// Confirm delivery (Seller only)
  Future<TransactionResult> confirmDelivery({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
    required String proofObjectId,
    int gasLimit = 100000,
    int gasPrice = 10,
  }) => _executeAction(
    wallet: wallet,
    coinType: coinType,
    functionName: EscrowConstants.fnConfirmDelivery,
    actionName: 'Confirming delivery',
    args: TransactionArgs()
      ..addObjectId(dealObjectId)
      ..addObjectId(proofObjectId),
    gasLimit: gasLimit,
    gasPrice: gasPrice,
  );

  /// Release funds (Buyer only)
  Future<TransactionResult> releaseFunds({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
    required String proofObjectId,
    int gasLimit = 100000,
    int gasPrice = 10,
  }) => _executeAction(
    wallet: wallet,
    coinType: coinType,
    functionName: EscrowConstants.fnReleaseFunds,
    actionName: 'Releasing funds',
    args: TransactionArgs()
      ..addObjectId(dealObjectId)
      ..addObjectId(proofObjectId),
    gasLimit: gasLimit,
    gasPrice: gasPrice,
  );

  /// Raise dispute
  Future<TransactionResult> raiseDispute({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
    required String reason,
    required String proofObjectId,
    int gasLimit = 100000,
    int gasPrice = 10,
  }) => _executeAction(
    wallet: wallet,
    coinType: coinType,
    functionName: EscrowConstants.fnRaiseDispute,
    actionName: 'Raising dispute',
    args: TransactionArgs()
      ..addObjectId(dealObjectId)
      ..addObjectId(proofObjectId)
      ..addString(reason),
    gasLimit: gasLimit,
    gasPrice: gasPrice,
  );

  /// Generic escrow action executor
  Future<TransactionResult> _executeAction({
    required KanariWallet wallet,
    required String coinType,
    required String functionName,
    required String actionName,
    required TransactionArgs args,
    int gasLimit = 100000,
    int gasPrice = 10,
  }) async {
    print('[ESCROW] Executing: $actionName');
    print('[ESCROW]   Function: $functionName');
    print('[ESCROW]   Coin Type: ${BcsUtils.normalizeTokenType(coinType)}');
    print('[ESCROW]   Args count: ${args.length}');

    try {
      final result = await rpc.executeFunction(
        wallet: wallet,
        package: EscrowConstants.packageAddress,
        module: EscrowConstants.module,
        function: functionName,
        typeArgs: [BcsUtils.normalizeTokenType(coinType)],
        args: args.build(),
        gasLimit: gasLimit,
        gasPrice: gasPrice,
      );

      print('[ESCROW] Result status: ${result.status}');
      if (result.errorMessage != null) {
        print('[ESCROW] Error: ${result.errorMessage}');
      }

      return result;
    } catch (e) {
      print('[ESCROW] Failed to execute $actionName: $e');
      rethrow;
    }
  }
}
