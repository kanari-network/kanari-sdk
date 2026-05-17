import 'dart:math' as math;

import '../models/account.dart';
import 'token_metadata.dart';

// Re-export all symbols from token_metadata for convenience
export 'token_metadata.dart';

bool isKanariToken(TokenBalance token) {
  return isKanariLike(
    tokenType: token.tokenType,
    symbol: token.symbol,
    name: token.name,
  );
}

TokenBalance buildKanariTokenBalance(int amount) {
  return TokenBalance(
    tokenType: kanariTokenType,
    symbol: kanariSymbol,
    amount: amount,
    decimals: kanariDecimals,
    name: 'Kanari Network Coin',
  );
}

double displayAmountFromBaseUnits(int amount, int decimals) {
  return amount / math.pow(10, decimals);
}

String formatDisplayAmount(int amount, int decimals, {int fractionDigits = 4}) {
  return displayAmountFromBaseUnits(
    amount,
    decimals,
  ).toStringAsFixed(fractionDigits);
}

int baseUnitsFromDisplayAmount(double amount, int decimals) {
  return (amount * math.pow(10, decimals)).round();
}

int baseUnitsFromDisplayString(
  String amount,
  int decimals, {
  int fallback = 0,
}) {
  final parsed = double.tryParse(amount.trim());
  if (parsed == null) return fallback;
  return baseUnitsFromDisplayAmount(parsed, decimals);
}
