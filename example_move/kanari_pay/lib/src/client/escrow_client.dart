// new_escrow_client.dart
import 'kanari_client.dart';
import '../kanari_wallet.dart';
import '../models/transaction.dart';
import '../modules/escrow/escrow.dart';

/// EscrowClient - facade that uses internal modules
class EscrowClient {
  final KanariClient rpc;
  late final EscrowOperations _operations;
  late final EscrowQueries _queries;

  EscrowClient(this.rpc) {
    _operations = EscrowOperations(rpc);
    _queries = EscrowQueries(rpc);
  }

  // ==================== OPERATIONS ====================

  Future<TransactionResult> createDeal({
    required KanariWallet wallet,
    required String dealId,
    required String sellerAddress,
    required int amount,
    required String description,
    required String tokenType,
    int gasLimit = 100000,
    int gasPrice = 10,
  }) {
    return _operations.createDeal(
      wallet: wallet,
      dealId: dealId,
      sellerAddress: sellerAddress,
      amount: amount,
      description: description,
      tokenType: tokenType,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  Future<TransactionResult> confirmDelivery({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
    required String proofObjectId,
    int gasLimit = 100000,
    int gasPrice = 10,
  }) {
    return _operations.confirmDelivery(
      wallet: wallet,
      dealObjectId: dealObjectId,
      coinType: coinType,
      proofObjectId: proofObjectId,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  Future<TransactionResult> releaseFunds({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
    required String proofObjectId,
    int gasLimit = 100000,
    int gasPrice = 10,
  }) {
    return _operations.releaseFunds(
      wallet: wallet,
      dealObjectId: dealObjectId,
      coinType: coinType,
      proofObjectId: proofObjectId,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  Future<TransactionResult> raiseDispute({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
    required String reason,
    required String proofObjectId,
    int gasLimit = 100000,
    int gasPrice = 10,
  }) {
    return _operations.raiseDispute(
      wallet: wallet,
      dealObjectId: dealObjectId,
      coinType: coinType,
      reason: reason,
      proofObjectId: proofObjectId,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  // ==================== QUERIES ====================

  Future<int> getDealStateByObjectId({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
  }) {
    return _queries.getDealStateByObjectId(
      wallet: wallet,
      dealObjectId: dealObjectId,
      coinType: coinType,
    );
  }

  Future<List<Map<String, dynamic>>> getAllDeals({
    required KanariWallet wallet,
    required String buyerAddress,
  }) {
    return _queries.getAllDeals(wallet: wallet, buyerAddress: buyerAddress);
  }

  Future<Map<String, dynamic>> getDealDetailsByObjectId({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
  }) {
    return _queries.getDealDetailsByObjectId(
      wallet: wallet,
      dealObjectId: dealObjectId,
      coinType: coinType,
    );
  }

  // ==================== UTILS ====================

  /// Get state name
  String getStateName(int state) => EscrowConstants.getStateName(state);

  /// Check if state matches
  bool isState(int actual, int expected) =>
      EscrowConstants.isState(actual, expected);

  /// Get spendable coin types from user's owned objects
  Future<List<String>> getSpendableCoinTypes(String address) async {
    try {
      print('[ESCROW CLIENT] getSpendableCoinTypes for: $address');

      final account = await rpc.getAccount(address);
      print(
        '[ESCROW CLIENT] Total owned objects: ${account.ownedObjects?.length ?? 0}',
      );

      final coinTypes = <String>{};

      for (final obj in account.ownedObjects ?? const []) {
        final tokenType = _extractCoinTypeFromObjectType(obj.type);
        if (tokenType != null) {
          print('[ESCROW CLIENT] Found coin object:');
          print('[ESCROW CLIENT]   ID: ${obj.id}');
          print('[ESCROW CLIENT]   Type: ${obj.type}');
          print('[ESCROW CLIENT]   Token: $tokenType');
          coinTypes.add(tokenType);
        }
      }

      final sorted = coinTypes.toList()..sort();
      print('[ESCROW CLIENT] Spendable coin types: $sorted');
      return sorted;
    } catch (e) {
      print('[ESCROW CLIENT] Error getting spendable coin types: $e');
      return [];
    }
  }

  /// Extract coin type from object type string
  /// Example: "0x2::coin::Coin<0xPKG::usdc::USDC>" -> "0xPKG::usdc::USDC"
  String? _extractCoinTypeFromObjectType(String objectType) {
    final start = objectType.indexOf('<');
    final end = objectType.lastIndexOf('>');
    if (start != -1 && end != -1) {
      final outer = objectType.substring(0, start);
      if (outer.endsWith('::coin::Coin') ||
          outer.endsWith('::coin::coin::Coin')) {
        return objectType.substring(start + 1, end);
      }
    }
    return null;
  }
}
