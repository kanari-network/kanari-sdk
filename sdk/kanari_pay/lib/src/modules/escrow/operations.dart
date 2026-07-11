// modules/escrow/operations.dart
// Escrow transaction operations.

import 'dart:async';

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
    var ownedObjects = await rpc.getOwnedObjects(wallet.address);
    var coinObject = _findOwnedCoinObject(
      ownedObjects: ownedObjects,
      tokenType: normalizedToken,
      requiredAmount: amount,
    );

    if (isKanariType(normalizedToken)) {
      ownedObjects = await _prepareNativeGasCoinIfNeeded(
        wallet: wallet,
        ownedObjects: ownedObjects,
        collateralCoin: coinObject,
        collateralAmount: amount,
        requiredGas: gasLimit * gasPrice,
        gasPrice: gasPrice,
      );
      coinObject = _findOwnedCoinObject(
        ownedObjects: ownedObjects,
        tokenType: normalizedToken,
        requiredAmount: amount,
      );
    }

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
    int? requiredAmount,
  }) {
    final candidates = ownedObjects.where((obj) {
      final objToken = BcsUtils.extractCoinTypeFromObjectType(obj.type);
      if (objToken == null || !BcsUtils.tokenTypesEqual(objToken, tokenType)) {
        return false;
      }

      if (requiredAmount == null) return true;
      final balance = BcsUtils.readCoinObjectBalance(obj.data);
      return balance != null && balance >= requiredAmount;
    }).toList()
      ..sort((a, b) {
        final aBalance = BcsUtils.readCoinObjectBalance(a.data) ?? 0;
        final bBalance = BcsUtils.readCoinObjectBalance(b.data) ?? 0;
        return bBalance.compareTo(aBalance);
      });

    if (candidates.isNotEmpty) {
      return candidates.first;
    }

    throw Exception(
      'No spendable Coin<$tokenType> object found in wallet.\n'
      'You need to have actual Coin objects, not just balance.\n'
      'Try minting or transferring tokens first.',
    );
  }

  bool _hasSeparateNativeGasCoin({
    required List<ObjectInfo> ownedObjects,
    required String collateralCoinId,
    required int requiredGas,
  }) {
    final collateralId = BcsUtils.normalizeObjectId(collateralCoinId);
    return ownedObjects.any((obj) {
      if (BcsUtils.normalizeObjectId(obj.id) == collateralId) return false;
      final objToken = BcsUtils.extractCoinTypeFromObjectType(obj.type);
      if (objToken == null || !isKanariType(objToken)) return false;
      final balance = BcsUtils.readCoinObjectBalance(obj.data);
      return balance != null && balance >= requiredGas;
    });
  }

  Future<List<ObjectInfo>> _prepareNativeGasCoinIfNeeded({
    required KanariWallet wallet,
    required List<ObjectInfo> ownedObjects,
    required ObjectInfo collateralCoin,
    required int collateralAmount,
    required int requiredGas,
    required int gasPrice,
  }) async {
    if (_hasSeparateNativeGasCoin(
      ownedObjects: ownedObjects,
      collateralCoinId: collateralCoin.id,
      requiredGas: requiredGas,
    )) {
      return ownedObjects;
    }

    final collateralBalance = BcsUtils.readCoinObjectBalance(collateralCoin.data);
    final prepareGasLimit = TransactionConstants.defaultGasLimit;
    final prepareGas = prepareGasLimit * gasPrice;
    final prepareAmount = requiredGas * 2;
    final requiredTotal = collateralAmount + prepareAmount + prepareGas;

    if (collateralBalance == null || collateralBalance < requiredTotal) {
      throw Exception(
        'KANARI can be used in DeFi, but it needs a separate Coin<KANARI> '
        'object for gas. This wallet does not have enough spendable KANARI '
        'to auto-prepare gas. Required at least $requiredTotal Mist, '
        'available ${collateralBalance ?? 0} Mist.',
      );
    }

    await rpc.transfer(
      wallet: wallet,
      recipient: wallet.address,
      amount: prepareAmount,
      gasLimit: prepareGasLimit,
      gasPrice: gasPrice,
    );

    for (var attempt = 0; attempt < 20; attempt++) {
      await Future<void>.delayed(const Duration(milliseconds: 500));
      final refreshed = await rpc.getOwnedObjects(wallet.address);
      if (_hasSeparateNativeGasCoin(
        ownedObjects: refreshed,
        collateralCoinId: collateralCoin.id,
        requiredGas: requiredGas,
      )) {
        return refreshed;
      }
    }

    throw Exception(
      'KANARI gas coin preparation was submitted, but the new gas coin is not '
      'indexed yet. Please retry creating the escrow in a few seconds.',
    );
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
