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
import 'kanari_wallet.dart';

class KanariClient {
  final String url;
  final http.Client _client;

  KanariClient(this.url, {http.Client? client}) : _client = client ?? http.Client();

  factory KanariClient.fromEnvironment(KanariEnvironment environment, {http.Client? client}) {
    return KanariClient(environment.rpcUrl, client: client);
  }

  Future<RpcResponse<T>> _request<T>(
    String method,
    Map<String, dynamic> params,
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
      throw Exception('Failed to connect to Kanari RPC: ${response.statusCode}');
    }

    final jsonResponse = jsonDecode(response.body) as Map<String, dynamic>;
    return RpcResponse<T>.fromJson(jsonResponse, fromJsonT);
  }

  // Account & Balance
  Future<AccountInfo> getAccount(String address) async {
    final resp = await _request('kanari_getAccount', {'address': address}, (j) => AccountInfo.fromJson(j as Map<String, dynamic>));
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<int> getBalance(String address) async {
    final resp = await _request('kanari_getBalance', {'address': address}, (j) => j as int);
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<TokenBalance> getTokenBalance(String address, String tokenType) async {
    final resp = await _request('kanari_getTokenBalance', {'address': address, 'token_type': tokenType}, (j) => TokenBalance.fromJson(j as Map<String, dynamic>));
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<List<TokenBalance>> getAllBalances(String address) async {
    final resp = await _request('kanari_getAllBalances', {'address': address}, (j) => (j as List).map((e) => TokenBalance.fromJson(e as Map<String, dynamic>)).toList());
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  // Blocks & Transactions
  Future<BlockInfo> getBlock(int height) async {
    final resp = await _request('kanari_getBlock', {'height': height}, (j) => BlockInfo.fromJson(j as Map<String, dynamic>));
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<int> getBlockHeight() async {
    final resp = await _request('kanari_getBlockHeight', {}, (j) => j as int);
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<TransactionDetails> getTransaction(String hash) async {
    final resp = await _request('kanari_getTransaction', {'hash': hash}, (j) => TransactionDetails.fromJson(j as Map<String, dynamic>));
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<BlockchainStats> getStats() async {
    final resp = await _request('kanari_getStats', {}, (j) => BlockchainStats.fromJson(j as Map<String, dynamic>));
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  // Health
  Future<HealthStatus> getHealth() async {
    final resp = await _request('kanari_health', {}, (j) => HealthStatus.fromJson(j as Map<String, dynamic>));
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  // Module operations
  Future<TransactionResult> publishModule({
    required String sender,
    required List<int> moduleBytes,
    required String moduleName,
    required int gasLimit,
    required int gasPrice,
    required int sequenceNumber,
    List<int>? signature,
    bool? executeImmediate,
  }) async {
    final params = {
      'sender': sender,
      'module_bytes': moduleBytes,
      'module_name': moduleName,
      'gas_limit': gasLimit,
      'gas_price': gasPrice,
      'sequence_number': sequenceNumber,
      'signature': signature,
      'execute_immediate': executeImmediate,
    };
    final resp = await _request('kanari_publishModule', params, (j) => TransactionResult.fromJson(j as Map<String, dynamic>));
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<ModuleInfo> getModule(String address, String name) async {
    final resp = await _request('kanari_getModule', {'address': address, 'name': name}, (j) => ModuleInfo.fromJson(j as Map<String, dynamic>));
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<List<ModuleInfo>> listModules() async {
    final resp = await _request('kanari_listModules', {}, (j) => (j as List).map((e) => ModuleInfo.fromJson(e as Map<String, dynamic>)).toList());
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<VerifyModuleResult> verifyModule(List<int> moduleBytes) async {
    final resp = await _request('kanari_verifyModule', {'module_bytes': moduleBytes}, (j) => VerifyModuleResult.fromJson(j as Map<String, dynamic>));
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
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

    // 2. Prepare transaction data (simplified for this RPC version)
    // In a real scenario, we might need to sign the full transaction bytes.
    // Here we'll follow the SubmitTransactionRequest structure from Rust.
    final txData = {
      'sender': wallet.address,
      'recipient': recipient,
      'amount': amount,
      'gas_limit': gasLimit,
      'gas_price': gasPrice,
      'sequence_number': sequenceNumber,
    };

    // 3. Sign the transaction (represented as JSON string or bytes depending on server)
    // For now, we sign the JSON representation as a simple way to demonstrate.
    final messageToSign = utf8.encode(jsonEncode(txData));
    final signature = await wallet.sign(messageToSign);

    // 4. Submit the transaction
    final params = {
      'transaction': {
        ...txData,
        'signature': signature.toList(),
      }
    };

    final resp = await _request('kanari_submitTransaction', params, (j) => TransactionResult.fromJson(j as Map<String, dynamic>));
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  void close() {
    _client.close();
  }
}
