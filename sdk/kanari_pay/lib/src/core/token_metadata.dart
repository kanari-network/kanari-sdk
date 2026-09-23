const String kanariTokenType = '0x2::kanari::KANARI';
const int kanariDecimals = 9;
const String kanariSymbol = 'KANARI';
const String kanariName = 'Kanari Network Coin';

bool isKanariType(String tokenType) => tokenType.contains('::kanari::KANARI');

bool isKanariLike({
  required String tokenType,
  String symbol = '',
  String? name,
}) {
  return tokenType == kanariTokenType ||
      isKanariType(tokenType) ||
      symbol.toUpperCase() == kanariSymbol ||
      name?.toUpperCase() == kanariSymbol;
}

String tokenInitials(String symbol) {
  final normalized = symbol.trim().toUpperCase();
  if (normalized.isEmpty) return '?';
  return normalized.length > 2 ? normalized.substring(0, 2) : normalized;
}
