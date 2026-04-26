// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'account.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

AccountInfo _$AccountInfoFromJson(Map<String, dynamic> json) => AccountInfo(
  address: json['address'] as String,
  balance: (json['balance'] as num).toInt(),
  sequenceNumber: (json['sequence_number'] as num).toInt(),
  modules: (json['modules'] as List<dynamic>).map((e) => e as String).toList(),
  tokenBalances: Map<String, int>.from(json['token_balances'] as Map),
  ownedObjects: (json['owned_objects'] as List<dynamic>?)
      ?.map((e) => ObjectInfo.fromJson(e as Map<String, dynamic>))
      .toList(),
);

Map<String, dynamic> _$AccountInfoToJson(AccountInfo instance) =>
    <String, dynamic>{
      'address': instance.address,
      'balance': instance.balance,
      'sequence_number': instance.sequenceNumber,
      'modules': instance.modules,
      'token_balances': instance.tokenBalances,
      'owned_objects': instance.ownedObjects,
    };

TokenBalance _$TokenBalanceFromJson(Map<String, dynamic> json) => TokenBalance(
  tokenType: json['token_type'] as String,
  amount: (json['balance'] as num).toInt(),
  decimals: (json['decimals'] as num).toInt(),
  symbol: json['symbol'] as String,
);

Map<String, dynamic> _$TokenBalanceToJson(TokenBalance instance) =>
    <String, dynamic>{
      'token_type': instance.tokenType,
      'balance': instance.amount,
      'decimals': instance.decimals,
      'symbol': instance.symbol,
    };

ObjectInfo _$ObjectInfoFromJson(Map<String, dynamic> json) => ObjectInfo(
  id: json['id'] as String,
  owner: json['owner'] as String,
  type: json['type_'] as String,
  data: (json['data'] as List<dynamic>).map((e) => (e as num).toInt()).toList(),
  version: (json['version'] as num).toInt(),
);

Map<String, dynamic> _$ObjectInfoToJson(ObjectInfo instance) =>
    <String, dynamic>{
      'id': instance.id,
      'owner': instance.owner,
      'type_': instance.type,
      'data': instance.data,
      'version': instance.version,
    };
