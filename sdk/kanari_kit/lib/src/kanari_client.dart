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
import 'utils/bcs_writer.dart';

class KanariClient {
  final String url;
  final http.Client _client;

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
    int gasPrice = 1,
    bool? executeImmediate,
  }) async {
    // 1. Get current sequence number
    final account = await getAccount(wallet.address);
    final sequenceNumber = account.sequenceNumber;

    // 2. Normalize sender address
    final senderAddress = _normalizeAddress(wallet.address);

    // 3. Sign the transaction
    // The Node expects the signature of the BCS-serialized Transaction enum.
    final writer = BcsWriter();
    // Variant 0: PublishModule (ULEB128)
    writer.writeULEB128(0);
    writer.writeString(senderAddress);
    writer.writeVectorU8(moduleBytes);
    writer.writeString(moduleName);
    writer.writeU64(gasLimit);
    writer.writeU64(gasPrice);
    writer.writeU64(sequenceNumber);

    final serializedTx = writer.toBytes();
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

  /// Normalize address to 0x followed by 64 hex characters (32 bytes)
  /// This matches how the Rust Address type is serialized to String.
  String _normalizeAddress(String addr) {
    var clean = addr.startsWith('0x') ? addr.substring(2) : addr;
    if (clean.length < 64) {
      clean = clean.padLeft(64, '0');
    }
    return '0x${clean.toLowerCase()}';
  }

  /// Transfer KANARI tokens from one account to another
  Future<TransactionResult> transfer({
    required KanariWallet wallet,
    required String recipient,
    required int amount,
    int gasLimit = 2000,
    int gasPrice = 1,
  }) async {
    // 1. Get current sequence number for the sender
    final account = await getAccount(wallet.address);
    final sequenceNumber = account.sequenceNumber;

    // 2. Normalize addresses to full 64-char hex strings
    final senderAddress = _normalizeAddress(wallet.address);
    final normalizedRecipient = _normalizeAddress(recipient);

    // 3. Sign the transaction
    // The Node expects the signature of the BCS-serialized Transaction enum.
    final writer = BcsWriter();
    // Variant 2: Transfer (ULEB128)
    writer.writeULEB128(2);
    writer.writeString(senderAddress);
    writer.writeString(normalizedRecipient);
    writer.writeU64(amount);
    writer.writeU64(gasLimit);
    writer.writeU64(gasPrice);
    writer.writeU64(sequenceNumber);

    final serializedTx = writer.toBytes();
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

  /// Convert hex address to raw 32 bytes
  List<int> _addressToBytes(String addr) {
    final clean = addr.startsWith('0x') ? addr.substring(2) : addr;
    final bytes = <int>[];
    for (var i = 0; i < clean.length; i += 2) {
      bytes.add(int.parse(clean.substring(i, i + 2), radix: 16));
    }
    return bytes;
  }

  void close() {
    _client.close();
  }
}
