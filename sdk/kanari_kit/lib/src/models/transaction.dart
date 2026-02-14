import 'package:json_annotation/json_annotation.dart';
import 'package:equatable/equatable.dart';

part 'transaction.g.dart';

@JsonSerializable()
class TransactionResult extends Equatable {
  final String hash;
  final String status;
  @JsonKey(name: 'gas_used')
  final int? gasUsed;
  @JsonKey(name: 'error_message')
  final String? errorMessage;
  final String? action; // Optional if coming from publish/upgrade

  const TransactionResult({
    required this.hash,
    required this.status,
    this.gasUsed,
    this.errorMessage,
    this.action,
  });

  factory TransactionResult.fromJson(Map<String, dynamic> json) =>
      _$TransactionResultFromJson(json);

  Map<String, dynamic> toJson() => _$TransactionResultToJson(this);

  @override
  List<Object?> get props => [hash, status, gasUsed, errorMessage, action];
}

@JsonSerializable()
class TransactionDetails extends Equatable {
  final String hash;
  final String status;
  @JsonKey(name: 'block_height')
  final int? blockHeight;
  @JsonKey(name: 'gas_used')
  final int? gasUsed;
  @JsonKey(name: 'tx_type')
  final String txType;
  final String sender;
  @JsonKey(name: 'sequence_number')
  final int sequenceNumber;
  @JsonKey(name: 'gas_limit')
  final int gasLimit;
  @JsonKey(name: 'gas_price')
  final int gasPrice;
  final String? module;
  final String? function;
  @JsonKey(name: 'module_functions')
  final List<String>? moduleFunctions;

  const TransactionDetails({
    required this.hash,
    required this.status,
    this.blockHeight,
    this.gasUsed,
    required this.txType,
    required this.sender,
    required this.sequenceNumber,
    required this.gasLimit,
    required this.gasPrice,
    this.module,
    this.function,
    this.moduleFunctions,
  });

  factory TransactionDetails.fromJson(Map<String, dynamic> json) =>
      _$TransactionDetailsFromJson(json);

  Map<String, dynamic> toJson() => _$TransactionDetailsToJson(this);

  @override
  List<Object?> get props => [
    hash,
    status,
    blockHeight,
    gasUsed,
    txType,
    sender,
    sequenceNumber,
    gasLimit,
    gasPrice,
    module,
    function,
    moduleFunctions,
  ];
}
