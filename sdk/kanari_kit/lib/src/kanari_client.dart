import 'dart:convert';

import 'package:http/http.dart' as http;
import 'models/rpc_response.dart';
import 'models/health.dart';
import 'models/account.dart';
import 'models/block.dart';
import 'models/stats.dart';
import 'models/transaction.dart';
import 'models/module.dart';
import 'models/environment.dart';
import 'package:kanari_crypto/kanari_crypto.dart';
import 'kanari_wallet.dart';
import 'package:bcs/bcs.dart';

class KanariClient {
  final String url;
  final http.Client _client;

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

  KanariClient(this.url, {http.Client? client})
    : _client = client ?? http.Client();

  factory KanariClient.fromEnvironment(
    KanariEnvironment environment, {
    http.Client? client,
  }) {
    return KanariClient(environment.rpcUrl, client: client);
  }

  Future<RpcResponse<T>> _request<T>(
    String method,
    dynamic params,
    T Function(Object? json) fromJsonT,
  ) async {
    final body = {
      'jsonrpc': '2.0',
      'method': method,
      'params': params,
      'id': DateTime.now().millisecondsSinceEpoch,
    };

    final response = await _client.post(
      Uri.parse(url),
      headers: {'Content-Type': 'application/json'},
      body: jsonEncode(body),
    );

    if (response.statusCode != 200) {
      throw Exception(
        'Failed to connect to Kanari RPC: ${response.statusCode}',
      );
    }

    final jsonResponse = jsonDecode(response.body) as Map<String, dynamic>;
    return RpcResponse<T>.fromJson(jsonResponse, fromJsonT);
  }

  // Account & Balance
  Future<AccountInfo> getAccount(String address) async {
    final normalizedAddress = _normalizeAddress(address);
    final resp = await _request(
      'kanari_getAccount',
      normalizedAddress,
      (j) => AccountInfo.fromJson(j as Map<String, dynamic>),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<int> getBalance(String address) async {
    final normalizedAddress = _normalizeAddress(address);
    final resp = await _request(
      'kanari_getBalance',
      normalizedAddress,
      (j) => j as int,
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<TokenBalance> getTokenBalance(String address, String tokenType) async {
    final normalizedAddress = _normalizeAddress(address);
    final resp = await _request(
      'kanari_getTokenBalance',
      {'address': normalizedAddress, 'token_type': tokenType},
      (j) => TokenBalance.fromJson(j as Map<String, dynamic>),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<List<TokenBalance>> getAllBalances(String address) async {
    final normalizedAddress = _normalizeAddress(address);
    final resp = await _request(
      'kanari_getAllBalances',
      {'address': normalizedAddress},
      (j) => (j as List)
          .map((e) => TokenBalance.fromJson(e as Map<String, dynamic>))
          .toList(),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  // Blocks & Transactions
  Future<BlockInfo> getBlock(int height) async {
    final resp = await _request('kanari_getBlock', {
      'height': height,
    }, (j) => BlockInfo.fromJson(j as Map<String, dynamic>));
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<int> getBlockHeight() async {
    final resp = await _request('kanari_getBlockHeight', {}, (j) => j as int);
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<TransactionDetails> getTransaction(String hash) async {
    final resp = await _request(
      'kanari_getTransaction',
      {'hash': hash},
      (j) => TransactionDetails.fromJson(j as Map<String, dynamic>),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<BlockchainStats> getStats() async {
    final resp = await _request(
      'kanari_getStats',
      {},
      (j) => BlockchainStats.fromJson(j as Map<String, dynamic>),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  // Health
  Future<HealthStatus> getHealth() async {
    final resp = await _request(
      'kanari_health',
      {},
      (j) => HealthStatus.fromJson(j as Map<String, dynamic>),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  /// Publish a Move module to the blockchain
  Future<TransactionResult> publishModule({
    required KanariWallet wallet,
    required List<int> moduleBytes,
    required String moduleName,
    int gasLimit = 100000,
    int gasPrice = 1000,
    bool? executeImmediate,
  }) async {
    // 1. Get current sequence number
    final account = await getAccount(wallet.address);
    final sequenceNumber = account.sequenceNumber;

    // 2. Normalize sender address
    final senderAddress = _getSenderForTx(wallet);

    // 3. Sign the transaction
    // The Node expects the signature of the BCS-serialized Transaction enum.
    final serializedTx = _transactionBcs.serialize({
      'PublishModule': {
        'sender': senderAddress,
        'module_bytes': moduleBytes,
        'module_name': moduleName,
        'gas_limit': gasLimit,
        'gas_price': gasPrice,
        'sequence_number': sequenceNumber,
      },
    }).toBytes();

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

    // 4. Submit the transaction
    final params = {
      'sender': senderAddress,
      'module_bytes': moduleBytes,
      'module_name': moduleName,
      'gas_limit': gasLimit,
      'gas_price': gasPrice,
      'sequence_number': sequenceNumber,
      'signature': signature.toList(),
      'execute_immediate': executeImmediate,
    };

    final resp = await _request(
      'kanari_publishModule',
      params,
      (j) => TransactionResult.fromJson(j as Map<String, dynamic>),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<ModuleInfo> getModule(String address, String name) async {
    final normalizedAddress = _normalizeAddress(address);
    final resp = await _request('kanari_getModule', {
      'address': normalizedAddress,
      'name': name,
    }, (j) => ModuleInfo.fromJson(j as Map<String, dynamic>));
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<List<ModuleInfo>> listModules() async {
    final resp = await _request(
      'kanari_listModules',
      {},
      (j) => (j as List)
          .map((e) => ModuleInfo.fromJson(e as Map<String, dynamic>))
          .toList(),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<VerifyModuleResult> verifyModule(List<int> moduleBytes) async {
    final resp = await _request(
      'kanari_verifyModule',
      {'module_bytes': moduleBytes},
      (j) => VerifyModuleResult.fromJson(j as Map<String, dynamic>),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  String _getSenderForTx(KanariWallet wallet) {
    // CRITICAL: Always use tagged address for ALL curve types
    // This is required for timing-safe signature verification per security spec
    // Format: CURVE:0xPUBKEY (e.g., 'K256:0xabc...', 'Ed25519:0x123...')
    return wallet.taggedAddress;
  }

  /// Normalize address to 0x followed by 64 hex characters (32 bytes)
  /// This matches how the Rust Address type is serialized to String.
  /// 
  /// IMPORTANT: Address MUST be exactly 64 hex characters (excluding 0x prefix).
  /// Addresses that are too short or too long will be rejected.
  String _normalizeAddress(String addr) {
    var clean = addr.startsWith('0x') ? addr.substring(2) : addr;
    
    // Validate hex characters
    if (!RegExp(r'^[0-9a-fA-F]+$').hasMatch(clean)) {
      throw ArgumentError('Invalid hexadecimal characters in address: $clean');
    }
    
    // CRITICAL: Address MUST be exactly 64 hex characters (32 bytes)
    // This prevents ambiguity and ensures compatibility with Rust backend
    if (clean.length != 64) {
      throw ArgumentError(
        'Address must be exactly 64 hex characters (32 bytes). '
        'Got ${clean.length} characters. '
        'Example: 0x${'1'.padLeft(64, '0')}',
      );
    }
    
    return '0x${clean.toLowerCase()}';
  }

  /// Transfer KANARI tokens from one account to another
  Future<TransactionResult> transfer({
    required KanariWallet wallet,
    required String recipient,
    required int amount,
    int gasLimit = 100000,
    int gasPrice = 1000,
  }) async {
    // 1. Get current sequence number for the sender
    final account = await getAccount(wallet.address);
    final sequenceNumber = account.sequenceNumber;

    // 2. Normalize addresses to full 64-char hex strings
    final senderAddress = _getSenderForTx(wallet);
    final normalizedRecipient = _normalizeAddress(recipient);

    // 3. Sign the transaction
    // The Node expects the signature of the BCS-serialized Transaction enum.
    final serializedTx = _transactionBcs.serialize({
      'Transfer': {
        'from': senderAddress,
        'to': normalizedRecipient,
        'amount': amount,
        'gas_limit': gasLimit,
        'gas_price': gasPrice,
        'sequence_number': sequenceNumber,
      },
    }).toBytes();

    List<int> messageToSign;
    try {
      // In Rust implementation, transactions are hashed with Blake3 before signing.
      // We must match this behavior to ensure signature verification succeeds.
      messageToSign = await blake3HashApi(data: serializedTx);
    } catch (e) {
      // Fallback for testing where RustLib is not initialized
      if (e.toString().contains(
        'flutter_rust_bridge has not been initialized',
      )) {
        messageToSign = serializedTx;
      } else {
        rethrow;
      }
    }
    final signature = await wallet.sign(messageToSign);

    // 4. Submit the transaction
    final params = {
      'sender': senderAddress,
      'recipient': normalizedRecipient,
      'amount': amount,
      'gas_limit': gasLimit,
      'gas_price': gasPrice,
      'sequence_number': sequenceNumber,
      'signature': signature.toList(),
    };

    final resp = await _request(
      'kanari_submitTransaction',
      params,
      (j) => TransactionResult.fromJson(j as Map<String, dynamic>),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  /// Execute a Move function
  Future<TransactionResult> executeFunction({
    required KanariWallet wallet,
    required String package,
    required String module,
    required String function,
    List<String> typeArgs = const [],
    List<List<int>> args = const [],
    int gasLimit = 100000,
    int gasPrice = 1000,
    bool? executeImmediate,
  }) async {
    // 1. Get current sequence number
    final account = await getAccount(wallet.address);
    final sequenceNumber = account.sequenceNumber;

    // 2. Normalize addresses
    final senderAddress = _getSenderForTx(wallet);
    final packageAddress = _normalizeAddress(package);

    // 3. Sign the transaction
    // The Node expects the signature of the BCS-serialized Transaction enum.
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

    // 4. Submit the transaction
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
      'signature': signature.toList(),
      'execute_immediate': executeImmediate,
    };

    final resp = await _request(
      'kanari_callFunction',
      params,
      (j) => TransactionResult.fromJson(j as Map<String, dynamic>),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  /// Burn KANARI tokens (restricted to system/admin)
  Future<TransactionResult> burn({
    required KanariWallet wallet,
    required int amount,
    int gasLimit = 100000,
    int gasPrice = 1000,
  }) async {
    // 1. Get current sequence number
    final account = await getAccount(wallet.address);
    final sequenceNumber = account.sequenceNumber;

    // 2. Normalize sender address
    final senderAddress = _getSenderForTx(wallet);

    // 3. Sign the transaction
    // The Node expects the signature of the BCS-serialized Transaction enum.
    final serializedTx = _transactionBcs.serialize({
      'Burn': {
        'from': senderAddress,
        'amount': amount,
        'gas_limit': gasLimit,
        'gas_price': gasPrice,
        'sequence_number': sequenceNumber,
      },
    }).toBytes();

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

    // 4. Submit the transaction
    final params = {
      'sender': senderAddress,
      'amount': amount,
      'gas_limit': gasLimit,
      'gas_price': gasPrice,
      'sequence_number': sequenceNumber,
      'signature': signature.toList(),
    };

    final resp = await _request(
      'kanari_submitTransaction',
      params,
      (j) => TransactionResult.fromJson(j as Map<String, dynamic>),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  void close() {
    _client.close();
  }
}
