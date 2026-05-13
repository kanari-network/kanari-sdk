// modules/transactions/operations.dart
/// Transaction operations module

import 'package:bcs/bcs.dart';
import 'package:http/http.dart' as http;
import 'package:kanari_crypto/kanari_crypto.dart';

import '../../core/bcs_serializers.dart';
import '../../core/rpc_utils.dart';
import '../../kanari_wallet.dart';
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

  TransactionOperations(this.url, this.queries, this.client);

  /// Get sender address for transaction (tagged format)
  String _getSenderForTx(KanariWallet wallet) {
    // CRITICAL: Always use tagged address for ALL curve types
    // This is required for timing-safe signature verification per security spec
    // Format: CURVE:0xPUBKEY (e.g., 'K256:0xabc...', 'Ed25519:0x123...')
    return wallet.taggedAddress;
  }

  /// Sign and submit transaction
  Future<TransactionResult> _signAndSubmit({
    required KanariWallet wallet,
    required Map<String, dynamic> txData,
    required String rpcMethod,
    required Map<String, dynamic> params,
  }) async {
    // Serialize transaction
    final serializedTx = _transactionBcs.serialize(txData).toBytes();

    // Hash with Blake3
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

    // Sign
    final signature = await wallet.sign(messageToSign);

    // Add signature to params
    params['signature'] = signature.toList();

    // Submit via RPC
    final resp = await RpcUtils.request(
      client,
      url,
      rpcMethod,
      params,
      (j) => TransactionResult.fromJson(j as Map<String, dynamic>),
    );

    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  /// Publish a Move module to the blockchain
  Future<TransactionResult> publishModule({
    required KanariWallet wallet,
    required List<int> moduleBytes,
    required String moduleName,
    int gasLimit = TransactionConstants.defaultGasLimit,
    int gasPrice = TransactionConstants.defaultGasPrice,
    bool? executeImmediate,
  }) async {
    // Get current sequence number
    final account = await queries.getAccount(wallet.address);
    final sequenceNumber = account.sequenceNumber;

    // Normalize sender address
    final senderAddress = _getSenderForTx(wallet);

    // Prepare transaction data
    final txData = {
      'PublishModule': {
        'sender': senderAddress,
        'module_bytes': moduleBytes,
        'module_name': moduleName,
        'gas_limit': gasLimit,
        'gas_price': gasPrice,
        'sequence_number': sequenceNumber,
      },
    };

    // Prepare RPC params
    final params = {
      'sender': senderAddress,
      'module_bytes': moduleBytes,
      'module_name': moduleName,
      'gas_limit': gasLimit,
      'gas_price': gasPrice,
      'sequence_number': sequenceNumber,
      'execute_immediate': executeImmediate,
    };

    return _signAndSubmit(
      wallet: wallet,
      txData: txData,
      rpcMethod: TransactionConstants.rpcPublishModule,
      params: params,
    );
  }

  /// Transfer KANARI tokens from one account to another
  Future<TransactionResult> transfer({
    required KanariWallet wallet,
    required String recipient,
    required int amount,
    int gasLimit = TransactionConstants.defaultGasLimit,
    int gasPrice = TransactionConstants.defaultGasPrice,
  }) async {
    // Get current sequence number
    final account = await queries.getAccount(wallet.address);
    final sequenceNumber = account.sequenceNumber;

    // Normalize addresses
    final senderAddress = _getSenderForTx(wallet);
    final normalizedRecipient = BcsSerializers.normalizeAddress(recipient);

    // Prepare transaction data
    final txData = {
      'Transfer': {
        'from': senderAddress,
        'to': normalizedRecipient,
        'amount': amount,
        'gas_limit': gasLimit,
        'gas_price': gasPrice,
        'sequence_number': sequenceNumber,
      },
    };

    // Prepare RPC params
    final params = {
      'sender': senderAddress,
      'recipient': normalizedRecipient,
      'amount': amount,
      'gas_limit': gasLimit,
      'gas_price': gasPrice,
      'sequence_number': sequenceNumber,
    };

    return _signAndSubmit(
      wallet: wallet,
      txData: txData,
      rpcMethod: TransactionConstants.rpcSubmitTransaction,
      params: params,
    );
  }

  /// Execute a Move function
  Future<TransactionResult> executeFunction({
    required KanariWallet wallet,
    required String package,
    required String module,
    required String function,
    List<String> typeArgs = const [],
    List<List<int>> args = const [],
    int gasLimit = TransactionConstants.defaultGasLimit,
    int gasPrice = 0,
    bool? executeImmediate,
  }) async {
    // Get current sequence number
    final account = await queries.getAccount(wallet.address);
    final sequenceNumber = account.sequenceNumber;

    // Normalize addresses
    final senderAddress = _getSenderForTx(wallet);
    final packageAddress = BcsSerializers.normalizeAddress(package);

    // Prepare transaction data
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

    // Prepare RPC params
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

  /// Burn KANARI tokens (restricted to system/admin)
  Future<TransactionResult> burn({
    required KanariWallet wallet,
    required int amount,
    int gasLimit = TransactionConstants.defaultGasLimit,
    int gasPrice = TransactionConstants.defaultGasPrice,
  }) async {
    // Get current sequence number
    final account = await queries.getAccount(wallet.address);
    final sequenceNumber = account.sequenceNumber;

    // Normalize sender address
    final senderAddress = _getSenderForTx(wallet);

    // Prepare transaction data
    final txData = {
      'Burn': {
        'from': senderAddress,
        'amount': amount,
        'gas_limit': gasLimit,
        'gas_price': gasPrice,
        'sequence_number': sequenceNumber,
      },
    };

    // Prepare RPC params
    final params = {
      'sender': senderAddress,
      'amount': amount,
      'gas_limit': gasLimit,
      'gas_price': gasPrice,
      'sequence_number': sequenceNumber,
    };

    return _signAndSubmit(
      wallet: wallet,
      txData: txData,
      rpcMethod: TransactionConstants.rpcSubmitTransaction,
      params: params,
    );
  }

  /// Transfer Custom Token
  Future<TransactionResult> transferToken({
    required KanariWallet wallet,
    required String recipient,
    required String tokenType,
    required int amount,
    int gasLimit = TransactionConstants.defaultGasLimit,
    int gasPrice = 0,
  }) async {
    // Get Account & Objects
    final account = await queries.getAccount(wallet.address);
    final normalizedRecipient = BcsSerializers.normalizeAddress(recipient);

    // Find the coin object ID matching the token type
    String? coinObjectId;
    if (account.ownedObjects != null) {
      for (final obj in account.ownedObjects!) {
        final objToken = BcsSerializers.extractCoinTypeFromObjectType(obj.type);
        if (objToken == tokenType) {
          coinObjectId = obj.id;
          break;
        }
      }
    }

    if (coinObjectId == null) {
      throw Exception(
        "No Coin<$tokenType> objects found.\n"
        "This usually means you don't have a spendable Coin object for this token.",
      );
    }

    // Parse token format: address::module::struct
    final parts = tokenType.split('::');
    if (parts.length < 3) {
      throw ArgumentError(
        "Invalid token format. Expected: address::module::struct",
      );
    }
    final packageAddress = parts[0];
    final moduleName = parts[1];

    // Prepare Arguments
    final objectIdBytes = BcsSerializers.hexToBytes(coinObjectId);
    final amountBytes = BcsSerializers.encodeU64(amount);
    final recipientBytes = BcsSerializers.hexToBytes(normalizedRecipient);

    // Submit transaction using ExecuteFunction
    return executeFunction(
      wallet: wallet,
      package: packageAddress,
      module: moduleName,
      function: 'transfer_amount',
      typeArgs: [],
      args: [objectIdBytes, amountBytes, recipientBytes],
      gasLimit: gasLimit,
      gasPrice: gasPrice,
      executeImmediate: true,
    );
  }
}
