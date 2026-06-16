// NEW FILE: modules/escrow/models.dart
// Escrow data models.

/// Escrow object references
class EscrowObjectRefs {
  final String dealObjectId;
  final String proofObjectId;
  final String coinType;

  const EscrowObjectRefs({
    required this.dealObjectId,
    required this.proofObjectId,
    required this.coinType,
  });

  /// Create from map
  factory EscrowObjectRefs.fromMap(Map<String, dynamic> map) {
    return EscrowObjectRefs(
      dealObjectId: map['dealObjectId'] as String,
      proofObjectId: map['proofObjectId'] as String,
      coinType: map['coinType'] as String,
    );
  }

  /// Convert to map
  Map<String, dynamic> toMap() {
    return {
      'dealObjectId': dealObjectId,
      'proofObjectId': proofObjectId,
      'coinType': coinType,
    };
  }
}

/// Deal details
class DealDetails {
  final String dealId;
  final String buyer;
  final String seller;
  final int amount;
  final String coinType;
  final int state;

  const DealDetails({
    required this.dealId,
    required this.buyer,
    required this.seller,
    required this.amount,
    required this.coinType,
    required this.state,
  });

  /// Create from map
  factory DealDetails.fromMap(Map<String, dynamic> map) {
    return DealDetails(
      dealId: map['deal_id'] as String,
      buyer: map['buyer'] as String,
      seller: map['seller'] as String,
      amount: map['amount'] as int,
      coinType: map['coin_type'] as String,
      state: map['state'] as int? ?? 0,
    );
  }

  /// Convert to map
  Map<String, dynamic> toMap() {
    return {
      'deal_id': dealId,
      'buyer': buyer,
      'seller': seller,
      'amount': amount,
      'coin_type': coinType,
      'state': state,
    };
  }

  @override
  String toString() {
    return 'DealDetails(dealId: $dealId, buyer: $buyer, seller: $seller, amount: $amount, state: $state)';
  }
}
