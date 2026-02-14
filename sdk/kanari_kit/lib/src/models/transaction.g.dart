// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'transaction.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

TransactionResult _$TransactionResultFromJson(Map<String, dynamic> json) =>
    TransactionResult(
      hash: json['hash'] as String,
      status: json['status'] as String,
      gasUsed: (json['gas_used'] as num?)?.toInt(),
      errorMessage: json['error_message'] as String?,
      action: json['action'] as String?,
    );

Map<String, dynamic> _$TransactionResultToJson(TransactionResult instance) =>
    <String, dynamic>{
      'hash': instance.hash,
      'status': instance.status,
      'gas_used': instance.gasUsed,
      'error_message': instance.errorMessage,
      'action': instance.action,
    };

TransactionDetails _$TransactionDetailsFromJson(Map<String, dynamic> json) =>
    TransactionDetails(
      hash: json['hash'] as String,
      status: json['status'] as String,
      blockHeight: (json['block_height'] as num?)?.toInt(),
      gasUsed: (json['gas_used'] as num?)?.toInt(),
      txType: json['tx_type'] as String,
      sender: json['sender'] as String,
      sequenceNumber: (json['sequence_number'] as num?)?.toInt() ?? 0,
      gasLimit: (json['gas_limit'] as num?)?.toInt() ?? 0,
      gasPrice: (json['gas_price'] as num?)?.toInt() ?? 0,
      module: json['module'] as String?,
      function: json['function'] as String?,
      moduleFunctions: (json['module_functions'] as List<dynamic>?)
          ?.map((e) => e as String)
          .toList(),
    );

Map<String, dynamic> _$TransactionDetailsToJson(TransactionDetails instance) =>
    <String, dynamic>{
      'hash': instance.hash,
      'status': instance.status,
      'block_height': instance.blockHeight,
      'gas_used': instance.gasUsed,
      'tx_type': instance.txType,
      'sender': instance.sender,
      'sequence_number': instance.sequenceNumber,
      'gas_limit': instance.gasLimit,
      'gas_price': instance.gasPrice,
      'module': instance.module,
      'function': instance.function,
      'module_functions': instance.moduleFunctions,
    };
