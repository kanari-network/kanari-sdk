import 'dart:typed_data';

import 'package:bcs/bcs.dart';
import 'package:http/http.dart' as http;
import 'package:kanari_crypto/kanari_crypto.dart';

import '../../core/bcs_utils.dart';
import '../../core/rpc_utils.dart';
import '../../core/token_utils.dart' as token_utils;
import '../../kanari_wallet.dart';
import '../../models/account.dart';
import '../../models/transaction.dart';
import '../queries.dart';
import 'constants.dart';

class TransactionOperations {
  final String url;
  final QueriesModule queries;
  final http.Client client;

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
  });

  TransactionOperations(this.url, this.queries, this.client);

  static const _systemCoinPackage = '0x2';
  static const _systemCoinModule = 'coin';
  static const _joinEntryFunction = 'join_entry';

  void _requirePositiveAmount(int amount, String name) {
    if (amount <= 0) {
      throw ArgumentError.value(amount, name, 'must be greater than 0');
    }
  }

  String _getSenderForTx(KanariWallet wallet) => wallet.taggedAddress;

  int _totalCost(int amount, int gasLimit, int gasPrice) {
    return amount + (gasLimit * gasPrice);
  }

  String? _normalizedTokenTypeFromCoinObject(String objectType) {
    final start = objectType.indexOf('<');
    final end = objectType.lastIndexOf('>');
    if (start == -1 || end == -1 || end <= start) {
      return null;
    }

    final outerType = objectType.substring(0, start);
    if (!outerType.endsWith('::coin::Coin') &&
        !outerType.endsWith('::coin::coin::Coin')) {
      return null;
    }

    final tokenType = objectType.substring(start + 1, end);
    return BcsUtils.normalizeTokenType(tokenType);
  }

  int? _readCoinBalance(List<int> data) {
    if (data.length < 40) {
      return null;
    }

    final balanceBytes = Uint8List.fromList(data.sublist(32, 40));
    return ByteData.sublistView(balanceBytes).getUint64(0, Endian.little);
  }

  List<_SpendableCoinObject> _spendableCoinObjects(
    AccountInfo account,
    String tokenType,
  ) {
    final wantedToken = BcsUtils.normalizeTokenType(tokenType);
    final coins = <_SpendableCoinObject>[];

    for (final obj in account.ownedObjects ?? const []) {
      final objToken = _normalizedTokenTypeFromCoinObject(obj.type);
      if (objToken == null ||
          !BcsUtils.tokenTypesEqual(objToken, wantedToken)) {
        continue;
      }

      final coinBalance = _readCoinBalance(obj.data);
      if (coinBalance == null || coinBalance <= 0) {
        continue;
      }

      coins.add(_SpendableCoinObject(id: obj.id, balance: coinBalance));
    }

    return coins;
  }

  _SelectedCoinObject _selectCoinObject(
    AccountInfo account,
    String tokenType,
    int requiredAmount,
  ) {
    final coins = _spendableCoinObjects(account, tokenType);
    int totalBalance = 0;
    _SpendableCoinObject? smallestSufficient;
    _SpendableCoinObject? largestAvailable;

    for (final coin in coins) {
      totalBalance += coin.balance;

      if (coin.balance >= requiredAmount &&
          (smallestSufficient == null ||
              coin.balance < smallestSufficient.balance)) {
        smallestSufficient = coin;
      }

      if (largestAvailable == null || coin.balance > largestAvailable.balance) {
        largestAvailable = coin;
      }
    }

    final selected = smallestSufficient ?? largestAvailable;
    if (selected == null) {
      throw Exception('No spendable Coin<$tokenType> object found.');
    }

    return _SelectedCoinObject(
      id: selected.id,
      selectedBalance: selected.balance,
      totalBalance: totalBalance,
    );
  }

  Future<TransactionResult> _signAndSubmit({
    required KanariWallet wallet,
    required Map<String, dynamic> txData,
    required String rpcMethod,
    required Map<String, dynamic> params,
  }) async {
    final serializedTx = _transactionBcs.serialize(txData).toBytes();

    List<int> messageToSign;
    try {
      messageToSign = await blake3HashApi(data: serializedTx);
    } catch (e) {
      if (e.toString().contains(
        'flutter_rust_bridge has not been initialized',
      )) {
        messageToSign = serializedTx;
      } else {
        rethrow;
      }
    }

    final signature = await wallet.sign(messageToSign);
    params['signature'] = signature.toList();

    final resp = await RpcUtils.request(
      client,
      url,
      rpcMethod,
      params,
      (j) => TransactionResult.fromJson(j as Map<String, dynamic>),
    );

    if (resp.error != null) throw Exception(resp.error!.message);

    final result = resp.result!;
    final status = result.status.toLowerCase();
    if (status != 'pending' &&
        status != 'executed' &&
        status != 'committed' &&
        status != 'success') {
      throw Exception(
        result.errorMessage?.isNotEmpty == true
            ? result.errorMessage
            : 'Transaction was not successful (status: ${result.status}, hash: ${result.hash})',
      );
    }

    return result;
  }

  Future<TransactionResult> _executeFunctionWithSequence({
    required KanariWallet wallet,
    required String package,
    required String module,
    required String function,
    required List<List<int>> args,
    required int sequenceNumber,
    List<String> typeArgs = const [],
    int gasLimit = TransactionConstants.defaultGasLimit,
    int gasPrice = TransactionConstants.defaultGasPrice,
    bool? executeImmediate,
  }) async {
    final senderAddress = _getSenderForTx(wallet);
    final packageAddress = BcsUtils.normalizeAnyAddress(package);

    final txData = {
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
    };

    final params = {
      'sender': senderAddress,
      'package': packageAddress,
      'module': module,
      'function': function,
      'type_args': typeArgs,
      'args': args,
      'gas_limit': gasLimit,
      'gas_price': gasPrice,
      'sequence_number': sequenceNumber,
      'execute_immediate': executeImmediate,
    };

    return _signAndSubmit(
      wallet: wallet,
      txData: txData,
      rpcMethod: TransactionConstants.rpcCallFunction,
      params: params,
    );
  }

  Future<TransactionResult> publishModule({
    required KanariWallet wallet,
    required List<int> moduleBytes,
    required String moduleName,
    int gasLimit = TransactionConstants.defaultGasLimit,
    int gasPrice = TransactionConstants.defaultGasPrice,
    bool? executeImmediate,
  }) async {
    final account = await queries.getOwner(wallet.address);
    final senderAddress = _getSenderForTx(wallet);

    final txData = {
      'PublishModule': {
        'sender': senderAddress,
        'module_bytes': moduleBytes,
        'module_name': moduleName,
        'gas_limit': gasLimit,
        'gas_price': gasPrice,
        'sequence_number': account.sequenceNumber,
      },
    };

    final params = {
      'sender': senderAddress,
      'module_bytes': moduleBytes,
      'module_name': moduleName,
      'gas_limit': gasLimit,
      'gas_price': gasPrice,
      'sequence_number': account.sequenceNumber,
      'execute_immediate': executeImmediate,
    };

    return _signAndSubmit(
      wallet: wallet,
      txData: txData,
      rpcMethod: TransactionConstants.rpcPublishModule,
      params: params,
    );
  }

  Future<_ConsolidatedCoinSelection> _prepareCoinForTransfer({
    required KanariWallet wallet,
    required String tokenType,
    required int amount,
    required int gasLimit,
    required int gasPrice,
  }) async {
    _requirePositiveAmount(amount, 'amount');
    final account = await queries.getOwner(wallet.address);
    final wantedToken = BcsUtils.normalizeTokenType(tokenType);
    final requiredAmount =
        BcsUtils.tokenTypesEqual(wantedToken, token_utils.kanariTokenType)
        ? _totalCost(amount, gasLimit, gasPrice)
        : amount;
    var selectedCoin = _selectCoinObject(account, wantedToken, requiredAmount);
    var nextSequence = account.sequenceNumber;

    if (selectedCoin.selectedBalance < requiredAmount) {
      if (selectedCoin.totalBalance < requiredAmount) {
        throw Exception(
          'Insufficient Coin<$wantedToken> balance.\n'
          'Required: $requiredAmount\n'
          'Best coin object: ${selectedCoin.selectedBalance}\n'
          'Total spendable across coin objects: ${selectedCoin.totalBalance}',
        );
      }

      final consolidation = await _consolidateCoinObjects(
        wallet: wallet,
        tokenType: wantedToken,
        spendableCoins: _spendableCoinObjects(account, wantedToken),
        requiredAmount: requiredAmount,
        startingSequence: account.sequenceNumber,
        gasLimit: gasLimit,
        gasPrice: gasPrice,
      );
      selectedCoin = consolidation.coin;
      nextSequence = consolidation.nextSequence;
    }

    return _ConsolidatedCoinSelection(
      coin: selectedCoin,
      sequenceNumber: nextSequence,
    );
  }

  Future<_ConsolidatedCoinSelection> _consolidateCoinObjects({
    required KanariWallet wallet,
    required String tokenType,
    required List<_SpendableCoinObject> spendableCoins,
    required int requiredAmount,
    required int startingSequence,
    required int gasLimit,
    required int gasPrice,
  }) async {
    final orderedCoins = [...spendableCoins]
      ..sort((a, b) => b.balance.compareTo(a.balance));

    if (orderedCoins.isEmpty) {
      throw Exception('No spendable Coin<$tokenType> object found.');
    }

    final primary = orderedCoins.first;
    final totalBalance = orderedCoins.fold<int>(
      0,
      (sum, coin) => sum + coin.balance,
    );
    var accumulated = primary.balance;
    var sequenceNumber = startingSequence;

    for (final coin in orderedCoins.skip(1)) {
      if (accumulated >= requiredAmount) {
        break;
      }

      await _executeFunctionWithSequence(
        wallet: wallet,
        package: _systemCoinPackage,
        module: _systemCoinModule,
        function: _joinEntryFunction,
        typeArgs: [tokenType],
        args: [
          BcsUtils.hexToBytes(BcsUtils.normalizeObjectId(primary.id)),
          BcsUtils.hexToBytes(BcsUtils.normalizeObjectId(coin.id)),
        ],
        gasLimit: gasLimit,
        gasPrice: gasPrice,
        sequenceNumber: sequenceNumber,
        executeImmediate: true,
      );
      sequenceNumber += 1;
      accumulated += coin.balance;
    }

    return _ConsolidatedCoinSelection(
      coin: _SelectedCoinObject(
        id: primary.id,
        selectedBalance: accumulated,
        totalBalance: totalBalance,
      ),
      sequenceNumber: sequenceNumber,
    );
  }

  Future<TransactionResult> transferWithCoinObject({
    required KanariWallet wallet,
    required String coinObjectId,
    required String recipient,
    required int amount,
    int gasLimit = TransactionConstants.defaultGasLimit,
    int gasPrice = TransactionConstants.defaultGasPrice,
    int? sequenceNumber,
  }) async {
    _requirePositiveAmount(amount, 'amount');
    final account = sequenceNumber == null
        ? await queries.getOwner(wallet.address)
        : null;
    final normalizedRecipient = BcsUtils.normalizeAddress(recipient);

    return _executeFunctionWithSequence(
      wallet: wallet,
      package: '0x2',
      module: 'kanari',
      function: 'transfer',
      args: [
        BcsUtils.hexToBytes(BcsUtils.normalizeObjectId(coinObjectId)),
        BcsUtils.encodeU64(amount),
        BcsUtils.hexToBytes(normalizedRecipient),
      ],
      gasLimit: gasLimit,
      gasPrice: gasPrice,
      sequenceNumber: sequenceNumber ?? account!.sequenceNumber,
      executeImmediate: true,
    );
  }

  Future<TransactionResult> transferTokenWithCoinObject({
    required KanariWallet wallet,
    required String coinObjectId,
    required String recipient,
    required String tokenType,
    required int amount,
    int gasLimit = TransactionConstants.defaultGasLimit,
    int gasPrice = TransactionConstants.defaultGasPrice,
    int? sequenceNumber,
  }) async {
    _requirePositiveAmount(amount, 'amount');
    final account = sequenceNumber == null
        ? await queries.getOwner(wallet.address)
        : null;
    final normalizedRecipient = BcsUtils.normalizeAddress(recipient);
    final wantedToken = BcsUtils.normalizeTokenType(tokenType);

    final parts = wantedToken.split('::');
    if (parts.length < 3) {
      throw ArgumentError(
        'Invalid token format. Expected: address::module::struct',
      );
    }

    return _executeFunctionWithSequence(
      wallet: wallet,
      package: parts[0],
      module: parts[1],
      function: 'transfer_amount',
      typeArgs: const <String>[],
      args: [
        BcsUtils.hexToBytes(BcsUtils.normalizeObjectId(coinObjectId)),
        BcsUtils.encodeU64(amount),
        BcsUtils.hexToBytes(normalizedRecipient),
      ],
      gasLimit: gasLimit,
      gasPrice: gasPrice,
      sequenceNumber: sequenceNumber ?? account!.sequenceNumber,
      executeImmediate: true,
    );
  }

  Future<TransactionResult> transfer({
    required KanariWallet wallet,
    required String recipient,
    required int amount,
    int gasLimit = TransactionConstants.defaultGasLimit,
    int gasPrice = TransactionConstants.defaultGasPrice,
  }) async {
    final prepared = await _prepareCoinForTransfer(
      wallet: wallet,
      tokenType: token_utils.kanariTokenType,
      amount: amount,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );

    return transferWithCoinObject(
      wallet: wallet,
      coinObjectId: prepared.coin.id,
      recipient: recipient,
      amount: amount,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
      sequenceNumber: prepared.sequenceNumber,
    );
  }

  Future<TransactionResult> executeFunction({
    required KanariWallet wallet,
    required String package,
    required String module,
    required String function,
    List<String> typeArgs = const [],
    List<List<int>> args = const [],
    int gasLimit = TransactionConstants.defaultGasLimit,
    int gasPrice = TransactionConstants.defaultGasPrice,
    bool? executeImmediate,
  }) async {
    final account = await queries.getOwner(wallet.address);
    return _executeFunctionWithSequence(
      wallet: wallet,
      package: package,
      module: module,
      function: function,
      typeArgs: typeArgs,
      args: args,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
      sequenceNumber: account.sequenceNumber,
      executeImmediate: executeImmediate,
    );
  }

  Future<TransactionResult> burn({
    required KanariWallet wallet,
    required int amount,
    int gasLimit = TransactionConstants.defaultGasLimit,
    int gasPrice = TransactionConstants.defaultGasPrice,
  }) async {
    _requirePositiveAmount(amount, 'amount');
    final account = await queries.getOwner(wallet.address);
    return _executeFunctionWithSequence(
      wallet: wallet,
      package: '0x2',
      module: 'kanari',
      function: 'burn_amount',
      args: [BcsUtils.encodeU64(amount)],
      gasLimit: gasLimit,
      gasPrice: gasPrice,
      sequenceNumber: account.sequenceNumber,
      executeImmediate: true,
    );
  }

  Future<TransactionResult> transferToken({
    required KanariWallet wallet,
    required String recipient,
    required String tokenType,
    required int amount,
    int gasLimit = TransactionConstants.defaultGasLimit,
    int gasPrice = TransactionConstants.defaultGasPrice,
  }) async {
    final prepared = await _prepareCoinForTransfer(
      wallet: wallet,
      tokenType: tokenType,
      amount: amount,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );

    return transferTokenWithCoinObject(
      wallet: wallet,
      coinObjectId: prepared.coin.id,
      recipient: recipient,
      tokenType: tokenType,
      amount: amount,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
      sequenceNumber: prepared.sequenceNumber,
    );
  }

  Future<TransactionResult> joinCoinObjects({
    required KanariWallet wallet,
    required String primaryCoinObjectId,
    required String mergeCoinObjectId,
    required String tokenType,
    int gasLimit = TransactionConstants.defaultGasLimit,
    int gasPrice = TransactionConstants.defaultGasPrice,
    int? sequenceNumber,
  }) async {
    final account = sequenceNumber == null
        ? await queries.getOwner(wallet.address)
        : null;

    return _executeFunctionWithSequence(
      wallet: wallet,
      package: _systemCoinPackage,
      module: _systemCoinModule,
      function: _joinEntryFunction,
      typeArgs: [BcsUtils.normalizeTokenType(tokenType)],
      args: [
        BcsUtils.hexToBytes(BcsUtils.normalizeObjectId(primaryCoinObjectId)),
        BcsUtils.hexToBytes(BcsUtils.normalizeObjectId(mergeCoinObjectId)),
      ],
      gasLimit: gasLimit,
      gasPrice: gasPrice,
      sequenceNumber: sequenceNumber ?? account!.sequenceNumber,
      executeImmediate: true,
    );
  }
}

class _SpendableCoinObject {
  final String id;
  final int balance;

  const _SpendableCoinObject({required this.id, required this.balance});
}

class _SelectedCoinObject {
  final String id;
  final int selectedBalance;
  final int totalBalance;

  const _SelectedCoinObject({
    required this.id,
    required this.selectedBalance,
    required this.totalBalance,
  });
}

class _ConsolidatedCoinSelection {
  final _SelectedCoinObject coin;
  final int sequenceNumber;

  const _ConsolidatedCoinSelection({
    required this.coin,
    required this.sequenceNumber,
  });
}
