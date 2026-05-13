// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'stats.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

BlockchainStats _$BlockchainStatsFromJson(Map<String, dynamic> json) =>
    BlockchainStats(
      height: (json['height'] as num).toInt(),
      totalBlocks: (json['total_blocks'] as num).toInt(),
      totalTransactions: (json['total_transactions'] as num).toInt(),
      pendingTransactions: (json['pending_transactions'] as num).toInt(),
      totalAccounts: (json['total_accounts'] as num).toInt(),
      totalSupply: (json['total_supply'] as num).toInt(),
    );

Map<String, dynamic> _$BlockchainStatsToJson(BlockchainStats instance) =>
    <String, dynamic>{
      'height': instance.height,
      'total_blocks': instance.totalBlocks,
      'total_transactions': instance.totalTransactions,
      'pending_transactions': instance.pendingTransactions,
      'total_accounts': instance.totalAccounts,
      'total_supply': instance.totalSupply,
    };
