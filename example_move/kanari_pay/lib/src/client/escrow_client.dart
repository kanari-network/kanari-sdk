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
    int gasLimit = 100000,
    int gasPrice = 10,
  }) {
    return _operations.confirmDelivery(
      wallet: wallet,
      dealObjectId: dealObjectId,
      coinType: coinType,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  Future<TransactionResult> releaseFunds({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
    int gasLimit = 100000,
    int gasPrice = 10,
  }) {
    return _operations.releaseFunds(
      wallet: wallet,
      dealObjectId: dealObjectId,
      coinType: coinType,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  Future<TransactionResult> raiseDispute({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
    required String reason,
    int gasLimit = 100000,
    int gasPrice = 10,
  }) {
    return _operations.raiseDispute(
      wallet: wallet,
      dealObjectId: dealObjectId,
      coinType: coinType,
      reason: reason,
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

  // ==================== BACKWARD COMPATIBILITY ====================
  // These methods are kept for backward compatibility with escrow_screen.dart

  /// Confirm delivery (backward compatibility wrapper)
  Future<TransactionResult> confirmDeliveryByObjectId({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
    String? proofObjectId, // Ignored for backward compatibility
    int gasLimit = 100000,
    int gasPrice = 10,
  }) {
    return confirmDelivery(
      wallet: wallet,
      dealObjectId: dealObjectId,
      coinType: coinType,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  /// Release funds (backward compatibility wrapper)
  Future<TransactionResult> releaseFundsByObjectId({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
    String? proofObjectId, // Ignored for backward compatibility
    int gasLimit = 100000,
    int gasPrice = 10,
  }) {
    return releaseFunds(
      wallet: wallet,
      dealObjectId: dealObjectId,
      coinType: coinType,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  /// Raise dispute (backward compatibility wrapper)
  Future<TransactionResult> raiseDisputeByObjectId({
    required KanariWallet wallet,
    required String dealObjectId,
    required String coinType,
    required String reason,
    String? proofObjectId, // Ignored for backward compatibility
    int gasLimit = 100000,
    int gasPrice = 10,
  }) {
    return raiseDispute(
      wallet: wallet,
      dealObjectId: dealObjectId,
      coinType: coinType,
      reason: reason,
      gasLimit: gasLimit,
      gasPrice: gasPrice,
    );
  }

  /// Get deal state (backward compatibility wrapper)
  Future<int> getDealState({
    required KanariWallet wallet,
    required String buyerAddress,
  }) async {
    // Get all deals for buyer
    final deals = await getAllDeals(wallet: wallet, buyerAddress: buyerAddress);

    if (deals.isEmpty) {
      return 0; // STATE_NONE
    }

    // Get state from first deal
    final firstDeal = deals.first;
    final dealObjectId = firstDeal['object_id'] as String;
    final coinType = firstDeal['coin_type'] as String;

    return getDealStateByObjectId(
      wallet: wallet,
      dealObjectId: dealObjectId,
      coinType: coinType,
    );
  }

  /// Get deal details (backward compatibility wrapper)
  Future<Map<String, dynamic>> getDealDetails({
    required KanariWallet wallet,
    required String buyerAddress,
  }) async {
    // Get all deals for buyer
    final deals = await getAllDeals(wallet: wallet, buyerAddress: buyerAddress);

    if (deals.isEmpty) {
      return {};
    }

    // Get details from first deal
    final firstDeal = deals.first;
    final dealObjectId = firstDeal['object_id'] as String;
    final coinType = firstDeal['coin_type'] as String;

    return getDealDetailsByObjectId(
      wallet: wallet,
      dealObjectId: dealObjectId,
      coinType: coinType,
    );
  }

  /// Get spendable coin types (backward compatibility wrapper)
  Future<List<String>> getSpendableCoinTypes(String address) async {
    // For now, return empty list
    // TODO: Implement proper coin type detection based on user's owned objects
    return [];
  }
}
