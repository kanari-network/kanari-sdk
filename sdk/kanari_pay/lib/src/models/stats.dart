import 'package:json_annotation/json_annotation.dart';
import 'package:equatable/equatable.dart';

part 'stats.g.dart';

@JsonSerializable()
class BlockchainStats extends Equatable {
  final int height;
  @JsonKey(name: 'total_blocks')
  final int totalBlocks;
  @JsonKey(name: 'total_transactions')
  final int totalTransactions;
  @JsonKey(name: 'pending_transactions')
  final int pendingTransactions;
  @JsonKey(name: 'total_accounts')
  final int totalAccounts;
  @JsonKey(name: 'total_supply')
  final int totalSupply;

  int get totalCheckpoints => totalBlocks;

  const BlockchainStats({
    required this.height,
    required this.totalBlocks,
    required this.totalTransactions,
    required this.pendingTransactions,
    required this.totalAccounts,
    required this.totalSupply,
  });

  factory BlockchainStats.fromJson(Map<String, dynamic> json) =>
      _$BlockchainStatsFromJson(json);

  Map<String, dynamic> toJson() => _$BlockchainStatsToJson(this);

  @override
  List<Object?> get props => [
    height,
    totalBlocks,
    totalTransactions,
    pendingTransactions,
    totalAccounts,
    totalSupply,
  ];
}
