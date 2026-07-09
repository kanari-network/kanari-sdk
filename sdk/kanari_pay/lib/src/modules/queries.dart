import 'package:http/http.dart' as http;

import '../core/rpc_utils.dart';
import '../models/account.dart';
import '../models/block.dart';
import '../models/health.dart';
import '../models/module.dart';
import '../models/stats.dart';
import '../models/transaction.dart';

class QueriesModule {
  final String url;
  final http.Client client;

  QueriesModule(this.url, this.client);

  Future<AccountInfo> getAccount(String address) async {
    return getOwner(address);
  }

  Future<AccountInfo> getOwner(String address) async {
    final normalizedAddress = _normalizeAddress(address);
    final resp = await RpcUtils.request(
      client,
      url,
      'kanari_getOwner',
      normalizedAddress,
      (j) => AccountInfo.fromJson(j as Map<String, dynamic>),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<TokenBalance> getTokenBalance(String address, String tokenType) async {
    final normalizedAddress = _normalizeAddress(address);
    final resp = await RpcUtils.request(
      client,
      url,
      'kanari_getTokenBalance',
      {'address': normalizedAddress, 'token_type': tokenType},
      (j) => TokenBalance.fromJson(j as Map<String, dynamic>),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<ObjectInfo> getObject(String objectId) async {
    final normalizedObjectId = _normalizeObjectId(objectId);
    final resp = await RpcUtils.request(
      client,
      url,
      'kanari_getObject',
      {'object_id': normalizedObjectId},
      (j) => ObjectInfo.fromJson(j as Map<String, dynamic>),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<List<ObjectInfo>> getOwnedObjects(
    String owner, {
    String? objectType,
  }) async {
    final normalizedOwner = _normalizeAddress(owner);
    final params = <String, dynamic>{'owner': normalizedOwner};
    if (objectType != null && objectType.trim().isNotEmpty) {
      params['object_type'] = objectType;
    }

    final resp = await RpcUtils.request(
      client,
      url,
      'kanari_getOwnedObjects',
      params,
      (j) {
        final map = j as Map<String, dynamic>;
        final objects =
            map['objects'] as List<dynamic>? ??
            map['owned_objects'] as List<dynamic>? ??
            const [];
        return objects
            .map((item) => ObjectInfo.fromJson(item as Map<String, dynamic>))
            .toList();
      },
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<List<TokenBalance>> getAllBalances(String address) async {
    final normalizedAddress = _normalizeAddress(address);
    final resp = await RpcUtils.request(
      client,
      url,
      'kanari_getAllBalances',
      {'address': normalizedAddress},
      (j) {
        final map = j as Map<String, dynamic>;
        final balancesList = map['balances'] as List<dynamic>? ?? const [];
        return balancesList
            .map((e) => TokenBalance.fromJson(e as Map<String, dynamic>))
            .toList();
      },
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<List<TokenInfo>> listTokens() async {
    final resp = await RpcUtils.request(
      client,
      url,
      'kanari_listTokens',
      {},
      (j) => (j as List<dynamic>)
          .map((e) => TokenInfo.fromJson(e as Map<String, dynamic>))
          .toList(),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<CheckpointInfo> getCheckpoint(int height) async {
    final resp = await RpcUtils.request(
      client,
      url,
      'kanari_getBlock',
      {'height': height},
      (j) => BlockInfo.fromJson(j as Map<String, dynamic>),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<BlockInfo> getBlock(int height) {
    return getCheckpoint(height);
  }

  Future<int> getCheckpointHeight() async {
    final resp = await RpcUtils.request(
      client,
      url,
      'kanari_getBlockHeight',
      {},
      (j) => j as int,
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<int> getBlockHeight() {
    return getCheckpointHeight();
  }

  Future<TransactionDetails> getTransaction(String hash) async {
    final resp = await RpcUtils.request(
      client,
      url,
      'kanari_getTransaction',
      {'hash': hash},
      (j) => TransactionDetails.fromJson(j as Map<String, dynamic>),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<List<TransactionDetails>> getAllTransactions({
    int limit = 50,
    String? account,
  }) async {
    final params = <String, dynamic>{'limit': limit};
    if (account != null && account.trim().isNotEmpty) {
      params['account'] = _normalizeAddress(account);
    }

    final resp = await RpcUtils.request(
      client,
      url,
      'kanari_getAllTransactions',
      params,
      (j) {
        final items = switch (j) {
          final List<dynamic> list => list,
          final Map<String, dynamic> map =>
            map['transactions'] as List<dynamic>? ??
                map['result'] as List<dynamic>? ??
                const <dynamic>[],
          _ => const <dynamic>[],
        };

        return items
            .map(
              (item) =>
                  TransactionDetails.fromJson(item as Map<String, dynamic>),
            )
            .toList();
      },
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<BlockchainStats> getStats() async {
    final resp = await RpcUtils.request(
      client,
      url,
      'kanari_getStats',
      {},
      (j) => BlockchainStats.fromJson(j as Map<String, dynamic>),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<HealthStatus> getHealth() async {
    final resp = await RpcUtils.request(
      client,
      url,
      'kanari_health',
      {},
      (j) => HealthStatus.fromJson(j as Map<String, dynamic>),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<ModuleInfo> getModule(String address, String name) async {
    final normalizedAddress = _normalizeAddress(address);
    final resp = await RpcUtils.request(
      client,
      url,
      'kanari_getModule',
      {'address': normalizedAddress, 'name': name},
      (j) => ModuleInfo.fromJson(j as Map<String, dynamic>),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<List<ModuleInfo>> listModules() async {
    final resp = await RpcUtils.request(
      client,
      url,
      'kanari_listModules',
      {},
      (j) => (j as List<dynamic>)
          .map((e) => ModuleInfo.fromJson(e as Map<String, dynamic>))
          .toList(),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  Future<VerifyModuleResult> verifyModule(List<int> moduleBytes) async {
    final resp = await RpcUtils.request(
      client,
      url,
      'kanari_verifyModule',
      {'module_bytes': moduleBytes},
      (j) => VerifyModuleResult.fromJson(j as Map<String, dynamic>),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  String _normalizeAddress(String addr) {
    var clean = addr.startsWith('0x') ? addr.substring(2) : addr;

    if (!RegExp(r'^[0-9a-fA-F]+$').hasMatch(clean)) {
      throw ArgumentError('Invalid hexadecimal characters in address: $clean');
    }

    if (clean.length > 64) {
      throw ArgumentError(
        'Address must be exactly 64 hex characters (32 bytes). Got ${clean.length} characters.',
      );
    }

    return '0x${clean.padLeft(64, '0').toLowerCase()}';
  }

  String _normalizeObjectId(String objectId) {
    return _normalizeAddress(objectId);
  }
}
