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
import '../core/core.dart';
import '../modules/modules.dart';

/// KanariClient - Facade that delegates to specialized modules
class KanariClient {
  final String url;
  final http.Client _client;

  late final QueriesModule _queries;
  late final TransactionOperations _transactions;
  late final DexOperations _dexOps;
  late final DexQueries _dexQueries;

  KanariClient(this.url, {http.Client? client})
    : _client = client ?? http.Client() {
    _queries = QueriesModule(url, _client);
    _transactions = TransactionOperations(url, _queries, _client);
    _dexOps = DexOperations(url, _queries, _client);
    _dexQueries = DexQueries(url, _queries, _client);
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

  /// Get KANARI balance
  Future<int> getBalance(String address) {
    return _queries.getBalance(address);
  }

  /// Get token balance
  Future<TokenBalance> getTokenBalance(String address, String tokenType) {
    return _queries.getTokenBalance(address, tokenType);
  }

  /// Get all token balances
  Future<List<TokenBalance>> getAllBalances(String address) {
    return _queries.getAllBalances(address);
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

  // ==================== DEX OPERATIONS ====================

  /// Create a new liquidity pool
  Future<TransactionResult> createPool({
    required KanariWallet wallet,
    required String coinTypeA,
    required String coinTypeB,
    int feePercent = 30,
    int gasLimit = 100000,
    int gasPrice = 1000,
  }) {
    return _dexOps.createPool(
      wallet: wallet,
      coinTypeA: coinTypeA,
      coinTypeB: coinTypeB,
      feePercent: feePercent,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  /// Add liquidity to an existing pool
  Future<TransactionResult> addLiquidity({
    required KanariWallet wallet,
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
    required int amountA,
    required int amountB,
    int gasLimit = 200000,
    int gasPrice = 1000,
  }) {
    return _dexOps.addLiquidity(
      wallet: wallet,
      poolObjectId: poolObjectId,
      coinTypeA: coinTypeA,
      coinTypeB: coinTypeB,
      amountA: amountA,
      amountB: amountB,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  /// Remove liquidity from a pool
  Future<TransactionResult> removeLiquidity({
    required KanariWallet wallet,
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
    required int lpTokenAmount,
    int gasLimit = 200000,
    int gasPrice = 1000,
  }) {
    return _dexOps.removeLiquidity(
      wallet: wallet,
      poolObjectId: poolObjectId,
      coinTypeA: coinTypeA,
      coinTypeB: coinTypeB,
      lpTokenAmount: lpTokenAmount,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  /// Swap Coin A for Coin B
  Future<TransactionResult> swapAForB({
    required KanariWallet wallet,
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
    required int amountIn,
    int gasLimit = 150000,
    int gasPrice = 1000,
  }) {
    return _dexOps.swapAForB(
      wallet: wallet,
      poolObjectId: poolObjectId,
      coinTypeA: coinTypeA,
      coinTypeB: coinTypeB,
      amountIn: amountIn,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  /// Swap Coin B for Coin A
  Future<TransactionResult> swapBForA({
    required KanariWallet wallet,
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
    required int amountIn,
    int gasLimit = 150000,
    int gasPrice = 1000,
  }) {
    return _dexOps.swapBForA(
      wallet: wallet,
      poolObjectId: poolObjectId,
      coinTypeA: coinTypeA,
      coinTypeB: coinTypeB,
      amountIn: amountIn,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  // ==================== DEX QUERIES ====================

  /// Get pool information
  Future<Map<String, dynamic>> getPoolInfo({
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
  }) {
    return _dexQueries.getPoolInfo(
      poolObjectId: poolObjectId,
      coinTypeA: coinTypeA,
      coinTypeB: coinTypeB,
    );
  }

  /// Calculate swap output amount
  Future<int> calculateSwapOutput({
    required String poolObjectId,
    required String coinTypeIn,
    required String coinTypeOut,
    required int amountIn,
  }) {
    return _dexQueries.calculateSwapAForBOutput(
      poolObjectId: poolObjectId,
      coinTypeA: coinTypeIn,
      coinTypeB: coinTypeOut,
      amountIn: amountIn,
    );
  }

  /// Get user's LP token balance for a pool
  Future<int> getLpTokenBalance({
    required String userAddress,
    required String poolObjectId,
    required String coinTypeA,
    required String coinTypeB,
  }) {
    return _dexQueries.getLpTokenBalance(
      userAddress: userAddress,
      poolObjectId: poolObjectId,
      coinTypeA: coinTypeA,
      coinTypeB: coinTypeB,
    );
  }

  /// List all pools created by user
  Future<List<Map<String, dynamic>>> getUserPools(String userAddress) {
    return _dexQueries.getUserPools(userAddress);
  }

  // ==================== UTILS ====================

  /// Close the HTTP client
  void close() {
    _client.close();
  }

  // ==================== BACKWARD COMPATIBILITY ====================
  // These methods are kept for backward compatibility

  /// Normalize address (backward compatibility)
  String normalizeAddress(String addr) {
    return BcsSerializers.normalizeAddress(addr);
  }

  /// Hex to bytes (backward compatibility)
  List<int> hexToBytes(String hexStr) {
    return BcsSerializers.hexToBytes(hexStr);
  }

  /// Encode U64 to BCS format (backward compatibility)
  List<int> encodeU64Bcs(int value) {
    return BcsSerializers.encodeU64(value);
  }
}
