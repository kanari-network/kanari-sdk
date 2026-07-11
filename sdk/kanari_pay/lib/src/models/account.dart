import 'package:equatable/equatable.dart';
import '../core/token_metadata.dart';

int _jsonInt(dynamic value, {int fallback = 0}) {
  if (value is int) return value;
  if (value is num) return value.toInt();
  if (value is String) return int.tryParse(value) ?? fallback;
  return fallback;
}

String _jsonString(dynamic value, {String fallback = ''}) {
  final stringValue = value?.toString() ?? '';
  return stringValue.isEmpty ? fallback : stringValue;
}

class AccountInfo extends Equatable {
  final String address;
  final int nonce;
  final List<String> modules;
  final Map<String, int> tokenBalances;
  final List<ObjectInfo>? ownedObjects;

  const AccountInfo({
    required this.address,
    required this.nonce,
    required this.modules,
    required this.tokenBalances,
    this.ownedObjects,
  });

  factory AccountInfo.fromJson(Map<String, dynamic> json) {
    final tokenBalances = <String, int>{};
    final rawTokenBalances = json['balances'] ?? json['token_balances'];
    if (rawTokenBalances is Map) {
      for (final entry in rawTokenBalances.entries) {
        tokenBalances[entry.key.toString()] = _jsonInt(entry.value);
      }
    }

    return AccountInfo(
      address: _jsonString(json['owner'] ?? json['address']),
      nonce: _jsonInt(json['nonce']),
      modules: (json['modules'] as List<dynamic>? ?? const [])
          .map((item) => item.toString())
          .toList(),
      tokenBalances: tokenBalances,
      ownedObjects: (json['owned_objects'] as List<dynamic>?)
          ?.map((item) => ObjectInfo.fromJson(item as Map<String, dynamic>))
          .toList(),
    );
  }

  Map<String, dynamic> toJson() => {
    'owner': address,
    'nonce': nonce,
    'modules': modules,
    'balances': tokenBalances,
    'owned_objects': ownedObjects?.map((item) => item.toJson()).toList(),
  };

  @override
  List<Object?> get props => [
    address,
    nonce,
    modules,
    tokenBalances,
    ownedObjects,
  ];
}

class TokenBalance extends Equatable {
  final String tokenType;
  final int amount;
  final int decimals;
  final String symbol;
  final String? iconUrl;
  final String? name;
  final String? description;

  const TokenBalance({
    required this.tokenType,
    required this.amount,
    required this.decimals,
    required this.symbol,
    this.iconUrl,
    this.name,
    this.description,
  });

  factory TokenBalance.fromJson(Map<String, dynamic> json) {
    final tokenType = _jsonString(json['token_type']);
    final isKanari = isKanariType(tokenType);

    return TokenBalance(
      tokenType: tokenType,
      amount: _jsonInt(json['amount'] ?? json['balance']),
      decimals: _jsonInt(
        json['decimals'],
        fallback: isKanari ? kanariDecimals : 9,
      ),
      symbol: _normalizeSymbol(json['symbol'], isKanari: isKanari),
      iconUrl: json['icon_url'] as String?,
      name: _normalizeName(json['name'], isKanari: isKanari),
      description: json['description'] as String?,
    );
  }

  Map<String, dynamic> toJson() => {
    'token_type': tokenType,
    'amount': amount,
    'decimals': decimals,
    'symbol': symbol,
    'icon_url': iconUrl,
    'name': name,
    'description': description,
  };

  @override
  List<Object?> get props => [
    tokenType,
    amount,
    decimals,
    symbol,
    iconUrl,
    name,
    description,
  ];

  static String _normalizeSymbol(dynamic raw, {required bool isKanari}) {
    final symbol = _jsonString(raw);
    if (symbol.isNotEmpty) {
      return symbol;
    }
    return isKanari ? kanariSymbol : '';
  }

  static String? _normalizeName(dynamic raw, {required bool isKanari}) {
    final name = raw?.toString();
    if (name != null && name.isNotEmpty) {
      return name;
    }
    return isKanari ? kanariName : null;
  }
}

class TokenInfo extends Equatable {
  final String tokenType;
  final int totalSupply;
  final int walletVisibleSupply;
  final int circulatingSupply;
  final int objectLockedSupply;
  final int accountedSupply;
  final int untrackedSupply;
  final int decimals;
  final String symbol;
  final String? iconUrl;
  final String? name;
  final String? description;

  const TokenInfo({
    required this.tokenType,
    required this.totalSupply,
    required this.walletVisibleSupply,
    required this.circulatingSupply,
    required this.objectLockedSupply,
    required this.accountedSupply,
    required this.untrackedSupply,
    required this.decimals,
    required this.symbol,
    this.iconUrl,
    this.name,
    this.description,
  });

  factory TokenInfo.fromJson(Map<String, dynamic> json) {
    final tokenType = _jsonString(json['token_type']);
    final isKanari = isKanariType(tokenType);
    final totalSupply = _jsonInt(json['total_supply']);
    final walletVisibleSupply = _jsonInt(
      json['wallet_visible_supply'],
      fallback: totalSupply,
    );

    return TokenInfo(
      tokenType: tokenType,
      totalSupply: totalSupply,
      walletVisibleSupply: walletVisibleSupply,
      circulatingSupply: _jsonInt(
        json['circulating_supply'],
        fallback: walletVisibleSupply,
      ),
      objectLockedSupply: _jsonInt(json['object_locked_supply']),
      accountedSupply: _jsonInt(
        json['accounted_supply'],
        fallback: totalSupply,
      ),
      untrackedSupply: _jsonInt(json['untracked_supply']),
      decimals: _jsonInt(
        json['decimals'],
        fallback: isKanari ? kanariDecimals : 9,
      ),
      symbol: TokenBalance._normalizeSymbol(json['symbol'], isKanari: isKanari),
      iconUrl: json['icon_url'] as String?,
      name: TokenBalance._normalizeName(json['name'], isKanari: isKanari),
      description: json['description'] as String?,
    );
  }

  Map<String, dynamic> toJson() => {
    'token_type': tokenType,
    'total_supply': totalSupply,
    'wallet_visible_supply': walletVisibleSupply,
    'circulating_supply': circulatingSupply,
    'object_locked_supply': objectLockedSupply,
    'accounted_supply': accountedSupply,
    'untracked_supply': untrackedSupply,
    'decimals': decimals,
    'symbol': symbol,
    'icon_url': iconUrl,
    'name': name,
    'description': description,
  };

  @override
  List<Object?> get props => [
    tokenType,
    totalSupply,
    walletVisibleSupply,
    circulatingSupply,
    objectLockedSupply,
    accountedSupply,
    untrackedSupply,
    decimals,
    symbol,
    iconUrl,
    name,
    description,
  ];
}

class ObjectInfo extends Equatable {
  final String id;
  final String owner;
  final String type;
  final List<int> data;
  final int version;
  final String? digest;

  const ObjectInfo({
    required this.id,
    required this.owner,
    required this.type,
    required this.data,
    required this.version,
    this.digest,
  });

  factory ObjectInfo.fromJson(Map<String, dynamic> json) {
    return ObjectInfo(
      id: _jsonString(json['id']),
      owner: _jsonString(json['owner']),
      type: _jsonString(json['type_'] ?? json['type']),
      data: (json['data'] as List<dynamic>? ?? const []).map(_jsonInt).toList(),
      version: _jsonInt(json['version']),
      digest: json['digest']?.toString(),
    );
  }

  Map<String, dynamic> toJson() => {
    'id': id,
    'owner': owner,
    'type_': type,
    'data': data,
    'version': version,
    'digest': digest,
  };

  @override
  List<Object?> get props => [id, owner, type, data, version, digest];
}
