// kanari_client.dart - Refactored version following escrow_client.dart pattern
import 'package:http/http.dart' as http;

import '../models/environment.dart';
import '../models/account.dart';
import '../models/block.dart';
import '../models/stats.dart';
import '../models/transaction.dart';
import '../models/module.dart';
import '../models/health.dart';
import '../kanari_wallet.dart';
import '../modules/modules.dart';

/// KanariClient - Facade that delegates to specialized modules
class KanariClient {
  final String url;
  final http.Client _client;

  late final QueriesModule _queries;
  late final TransactionOperations _transactions;

  KanariClient(this.url, {http.Client? client})
    : _client = client ?? http.Client() {
    _queries = QueriesModule(url, _client);
    _transactions = TransactionOperations(url, _queries, _client);
  }

  factory KanariClient.fromEnvironment(
    KanariEnvironment environment, {
    http.Client? client,
  }) {
    return KanariClient(environment.rpcUrl, client: client);
  }

  // ==================== QUERIES (Read Operations) ====================

  /// Get account information
  Future<AccountInfo> getAccount(String address) {
    return _queries.getAccount(address);
  }

  /// Get token balance
  Future<TokenBalance> getTokenBalance(String address, String tokenType) {
    return _queries.getTokenBalance(address, tokenType);
  }

  /// Get all token balances
  Future<List<TokenBalance>> getAllBalances(String address) {
    return _queries.getAllBalances(address);
  }

  /// List registered tokens and DeFi-aware supply accounting.
  Future<List<TokenInfo>> listTokens() {
    return _queries.listTokens();
  }

  /// Get block by height
  Future<BlockInfo> getBlock(int height) {
    return _queries.getBlock(height);
  }

  /// Get current block height
  Future<int> getBlockHeight() {
    return _queries.getBlockHeight();
  }

  /// Get transaction details
  Future<TransactionDetails> getTransaction(String hash) {
    return _queries.getTransaction(hash);
  }

  /// Get blockchain statistics
  Future<BlockchainStats> getStats() {
    return _queries.getStats();
  }

  /// Get health status
  Future<HealthStatus> getHealth() {
    return _queries.getHealth();
  }

  /// Get module information
  Future<ModuleInfo> getModule(String address, String name) {
    return _queries.getModule(address, name);
  }

  /// List all modules
  Future<List<ModuleInfo>> listModules() {
    return _queries.listModules();
  }

  /// Verify module bytecode
  Future<VerifyModuleResult> verifyModule(List<int> moduleBytes) {
    return _queries.verifyModule(moduleBytes);
  }

  // ==================== TRANSACTIONS (Write Operations) ====================

  /// Publish a Move module to the blockchain
  Future<TransactionResult> publishModule({
    required KanariWallet wallet,
    required List<int> moduleBytes,
    required String moduleName,
    int gasLimit = TransactionConstants.defaultGasLimit,
    int gasPrice = TransactionConstants.defaultGasPrice,
    bool? executeImmediate,
  }) {
    return _transactions.publishModule(
      wallet: wallet,
      moduleBytes: moduleBytes,
      moduleName: moduleName,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
      executeImmediate: executeImmediate,
    );
  }

  /// Transfer KANARI tokens from one account to another
  Future<TransactionResult> transfer({
    required KanariWallet wallet,
    required String recipient,
    required int amount,
    int gasLimit = TransactionConstants.defaultGasLimit,
    int gasPrice = TransactionConstants.defaultGasPrice,
  }) {
    return _transactions.transfer(
      wallet: wallet,
      recipient: recipient,
      amount: amount,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
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
  }) {
    return _transactions.executeFunction(
      wallet: wallet,
      package: package,
      module: module,
      function: function,
      typeArgs: typeArgs,
      args: args,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
      executeImmediate: executeImmediate,
    );
  }

  /// Burn KANARI tokens (restricted to system/admin)
  Future<TransactionResult> burn({
    required KanariWallet wallet,
    required int amount,
    int gasLimit = TransactionConstants.defaultGasLimit,
    int gasPrice = TransactionConstants.defaultGasPrice,
  }) {
    return _transactions.burn(
      wallet: wallet,
      amount: amount,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
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
  }) {
    return _transactions.transferToken(
      wallet: wallet,
      recipient: recipient,
      tokenType: tokenType,
      amount: amount,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }
}
