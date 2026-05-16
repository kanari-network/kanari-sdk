// modules/queries.dart
/// Query operations module for reading blockchain data

import 'package:http/http.dart' as http;

import '../core/rpc_utils.dart';
import '../models/account.dart';
import '../models/block.dart';
import '../models/stats.dart';
import '../models/transaction.dart';
import '../models/module.dart';
import '../models/health.dart';

class QueriesModule {
  final String url;
  final http.Client client;

  QueriesModule(this.url, this.client);

  /// Get account information
  Future<AccountInfo> getAccount(String address) async {
    final normalizedAddress = _normalizeAddress(address);
    final resp = await RpcUtils.request(
      client,
      url,
      'kanari_getAccount',
      normalizedAddress,
      (j) => AccountInfo.fromJson(j as Map<String, dynamic>),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  /// Get KANARI balance
  Future<int> getBalance(String address) async {
    final normalizedAddress = _normalizeAddress(address);
    final resp = await RpcUtils.request(
      client,
      url,
      'kanari_getBalance',
      normalizedAddress,
      (j) => j as int,
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  /// Get token balance
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

  /// Get all token balances
  Future<List<TokenBalance>> getAllBalances(String address) async {
    final normalizedAddress = _normalizeAddress(address);
    final resp = await RpcUtils.request(
      client,
      url,
      'kanari_getAllBalances',
      {'address': normalizedAddress},
      (j) {
        final map = j as Map<String, dynamic>;
        final balancesList = map['balances'] as List<dynamic>? ?? [];
        return balancesList
            .map((e) => TokenBalance.fromJson(e as Map<String, dynamic>))
            .toList();
      },
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  /// Get block by height
  Future<BlockInfo> getBlock(int height) async {
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

  /// Get current block height
  Future<int> getBlockHeight() async {
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

  /// Get transaction details
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

  /// Get blockchain statistics
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

  /// Get health status
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

  /// Get module information
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

  /// List all modules
  Future<List<ModuleInfo>> listModules() async {
    final resp = await RpcUtils.request(
      client,
      url,
      'kanari_listModules',
      {},
      (j) => (j as List)
          .map((e) => ModuleInfo.fromJson(e as Map<String, dynamic>))
          .toList(),
    );
    if (resp.error != null) throw Exception(resp.error!.message);
    return resp.result!;
  }

  /// Verify module bytecode
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

  /// Get all objects owned by an address
  Future<List<Map<String, dynamic>>> getOwnedObjects(String address) async {
    final account = await getAccount(address);
    
    // Convert ObjectInfo list to Map list for easier handling
    if (account.ownedObjects == null || account.ownedObjects!.isEmpty) {
      return [];
    }
    
    return account.ownedObjects!.map((obj) {
      return {
        'id': obj.id,
        'objectId': obj.id,
        'type': obj.type,
        'owner': obj.owner,
        'data': obj.data,
      };
    }).toList();
  }

  /// Normalize address to 0x followed by 64 hex characters.
  /// Short-form addresses like `0x2` are left-padded to 32 bytes.
  String _normalizeAddress(String addr) {
    var clean = addr.startsWith('0x') ? addr.substring(2) : addr;

    // Validate hex characters
    if (!RegExp(r'^[0-9a-fA-F]+$').hasMatch(clean)) {
      throw ArgumentError('Invalid hexadecimal characters in address: $clean');
    }

    // Canonicalize to the 32-byte form expected by RPC.
    if (clean.length > 64) {
      throw ArgumentError(
        'Address must be exactly 64 hex characters (32 bytes). '
        'Got ${clean.length} characters. '
        'Example: 0x${'1'.padLeft(64, '0')}',
      );
    }

    return '0x${clean.padLeft(64, '0').toLowerCase()}';
  }
}
