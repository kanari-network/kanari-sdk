import 'package:json_annotation/json_annotation.dart';
import 'package:equatable/equatable.dart';

part 'account.g.dart';

@JsonSerializable()
class AccountInfo extends Equatable {
  final String address;
  final int balance;
  @JsonKey(name: 'sequence_number')
  final int sequenceNumber;
  final List<String> modules;
  @JsonKey(name: 'token_balances')
  final Map<String, int> tokenBalances;
  @JsonKey(name: 'owned_objects')
  final List<ObjectInfo>? ownedObjects;

  const AccountInfo({
    required this.address,
    required this.balance,
    required this.sequenceNumber,
    required this.modules,
    required this.tokenBalances,
    this.ownedObjects,
  });

  factory AccountInfo.fromJson(Map<String, dynamic> json) =>
      _$AccountInfoFromJson(json);

  Map<String, dynamic> toJson() => _$AccountInfoToJson(this);

  @override
  List<Object?> get props => [
    address,
    balance,
    sequenceNumber,
    modules,
    tokenBalances,
    ownedObjects,
  ];
}

// TokenBalance model for fungible tokens
@JsonSerializable()
class TokenBalance extends Equatable {
  @JsonKey(name: 'token_type')
  final String tokenType;

  @JsonKey(name: 'balance') // Balance amount field
  final int amount;

  final int decimals;
  final String symbol;
  
  @JsonKey(name: 'icon_url')
  final String? iconUrl;

  const TokenBalance({
    required this.tokenType,
    required this.amount,
    required this.decimals,
    required this.symbol,
    this.iconUrl,
  });

  factory TokenBalance.fromJson(Map<String, dynamic> json) =>
      _$TokenBalanceFromJson(json);

  Map<String, dynamic> toJson() => _$TokenBalanceToJson(this);

  @override
  List<Object?> get props => [tokenType, amount, decimals, symbol, iconUrl];
}

@JsonSerializable()
class ObjectInfo extends Equatable {
  final String id;
  final String owner;
  @JsonKey(name: 'type_')
  final String type;
  final List<int> data;
  final int version;

  const ObjectInfo({
    required this.id,
    required this.owner,
    required this.type,
    required this.data,
    required this.version,
  });

  factory ObjectInfo.fromJson(Map<String, dynamic> json) =>
      _$ObjectInfoFromJson(json);

  Map<String, dynamic> toJson() => _$ObjectInfoToJson(this);

  @override
  List<Object?> get props => [id, owner, type, data, version];
}
