// modules/escrow/operations.dart
// Escrow transaction operations.

import '../../client/kanari_client.dart';
import '../../core/bcs_utils.dart';
import '../../core/token_metadata.dart';
import '../../kanari_wallet.dart';
import '../../models/account.dart';
import '../../models/transaction.dart';
import '../transactions/constants.dart';
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
    int gasLimit = TransactionConstants.defaultGasLimit,
    int gasPrice = TransactionConstants.defaultGasPrice,
  }) async {
    final normalizedToken = BcsUtils.normalizeTokenType(tokenType);
    final ownedObjects = await rpc.getOwnedObjects(wallet.address);
    final coinObject = _findOwnedCoinObject(
      ownedObjects: ownedObjects,
      tokenType: normalizedToken,
    );
    _validateNativeEscrowGasSeparation(
      ownedObjects: ownedObjects,
      collateralCoinId: coinObject.id,
      tokenType: normalizedToken,
      requiredGas: gasLimit * gasPrice,
    );
    final coinObjectId = BcsUtils.normalizeObjectId(coinObject.id);

    final args = TransactionArgs()
      ..addString(dealId)
      ..addAddress(sellerAddress)
      ..addAmount(amount)
      ..addString(description)
      ..addObjectId(coinObjectId);

    return rpc.executeFunction(
      wallet: wallet,
      package: EscrowConstants.packageAddress,
      module: EscrowConstants.module,
      function: EscrowConstants.fnCreateDeal,
      typeArgs: [normalizedToken],
      args: args.build(),
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  /// Find owned Coin object for a specific token type
  ObjectInfo _findOwnedCoinObject({
    required List<ObjectInfo> ownedObjects,
    required String tokenType,
  }) {
    for (final obj in ownedObjects) {
      final objToken = BcsUtils.extractCoinTypeFromObjectType(obj.type);
      if (objToken != null && BcsUtils.tokenTypesEqual(objToken, tokenType)) {
        return obj;
      }
    }

    throw Exception(
      'No spendable Coin<$tokenType> object found in wallet.\n'
      'You need to have actual Coin objects, not just balance.\n'
      'Try minting or transferring tokens first.',
    );
  }

  void _validateNativeEscrowGasSeparation({
    required List<ObjectInfo> ownedObjects,
    required String collateralCoinId,
    required String tokenType,
    required int requiredGas,
  }) {
    if (!isKanariType(tokenType)) return;

    final collateralId = BcsUtils.normalizeObjectId(collateralCoinId);
    final hasSeparateGasCoin = ownedObjects.any((obj) {
      if (BcsUtils.normalizeObjectId(obj.id) == collateralId) return false;
      final objToken = BcsUtils.extractCoinTypeFromObjectType(obj.type);
      if (objToken == null || !isKanariType(objToken)) return false;
      final balance = BcsUtils.readCoinObjectBalance(obj.data);
      return balance != null && balance >= requiredGas;
    });

    if (!hasSeparateGasCoin) {
      throw Exception(
        'KANARI can be used in DeFi, but it needs a separate Coin<KANARI> '
        'object for gas. Split or fund a second KANARI coin object before '
        'creating this escrow.',
      );
    }
  }

  /// Confirm delivery (Seller only)
  Future<TransactionResult> confirmDelivery({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
    required String proofObjectId,
    int gasLimit = TransactionConstants.defaultGasLimit,
    int gasPrice = TransactionConstants.defaultGasPrice,
  }) => _executeAction(
    wallet: wallet,
    coinType: coinType,
    functionName: EscrowConstants.fnConfirmDelivery,
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
    int gasLimit = TransactionConstants.defaultGasLimit,
    int gasPrice = TransactionConstants.defaultGasPrice,
  }) => _executeAction(
    wallet: wallet,
    coinType: coinType,
    functionName: EscrowConstants.fnReleaseFunds,
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
    int gasLimit = TransactionConstants.defaultGasLimit,
    int gasPrice = TransactionConstants.defaultGasPrice,
  }) => _executeAction(
    wallet: wallet,
    coinType: coinType,
    functionName: EscrowConstants.fnRaiseDispute,
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
    required TransactionArgs args,
    int gasLimit = TransactionConstants.defaultGasLimit,
    int gasPrice = TransactionConstants.defaultGasPrice,
  }) {
    return rpc.executeFunction(
      wallet: wallet,
      package: EscrowConstants.packageAddress,
      module: EscrowConstants.module,
      function: functionName,
      typeArgs: [BcsUtils.normalizeTokenType(coinType)],
      args: args.build(),
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }
}
