import 'package:equatable/equatable.dart';

int _jsonInt(dynamic value, {int? fallback}) {
  if (value is int) return value;
  if (value is num) return value.toInt();
  if (value is String) return int.tryParse(value) ?? (fallback ?? 0);
  return fallback ?? 0;
}

class TransactionResult extends Equatable {
  final String hash;
  final String status;
  final int? gasUsed;
  final String? errorMessage;
  final String? action;

  const TransactionResult({
    required this.hash,
    required this.status,
    this.gasUsed,
    this.errorMessage,
    this.action,
  });

  factory TransactionResult.fromJson(Map<String, dynamic> json) {
    final changeset = json['changeset'];
    final nestedError = changeset is Map<String, dynamic>
        ? changeset['error_message']?.toString() ?? ''
        : '';

    return TransactionResult(
      hash: json['hash']?.toString() ?? '',
      status: json['status']?.toString() ?? '',
      gasUsed: json['gas_used'] == null ? null : _jsonInt(json['gas_used']),
      errorMessage: (json['error_message']?.toString().isNotEmpty == true)
          ? json['error_message'].toString()
          : (nestedError.isEmpty ? null : nestedError),
      action: json['action']?.toString(),
    );
  }

  Map<String, dynamic> toJson() => {
    'hash': hash,
    'status': status,
    'gas_used': gasUsed,
    'error_message': errorMessage,
    'action': action,
  };

  @override
  List<Object?> get props => [hash, status, gasUsed, errorMessage, action];
}

class TransactionDetails extends Equatable {
  final String hash;
  final String status;
  final int? blockHeight;
  final int? gasUsed;
  final String txType;
  final String sender;
  final int sequenceNumber;
  final int gasLimit;
  final int gasPrice;
  final String? module;
  final String? function;
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

  factory TransactionDetails.fromJson(Map<String, dynamic> json) {
    return TransactionDetails(
      hash: json['hash']?.toString() ?? '',
      status: json['status']?.toString() ?? '',
      blockHeight: json['block_height'] == null
          ? null
          : _jsonInt(json['block_height']),
      gasUsed: json['gas_used'] == null ? null : _jsonInt(json['gas_used']),
      txType: json['tx_type']?.toString() ?? '',
      sender: json['sender']?.toString() ?? '',
      sequenceNumber: _jsonInt(json['sequence_number']),
      gasLimit: _jsonInt(json['gas_limit']),
      gasPrice: _jsonInt(json['gas_price']),
      module: json['module']?.toString(),
      function: json['function']?.toString(),
      moduleFunctions: (json['module_functions'] as List<dynamic>?)
          ?.map((item) => item.toString())
          .toList(),
    );
  }

  Map<String, dynamic> toJson() => {
    'hash': hash,
    'status': status,
    'block_height': blockHeight,
    'gas_used': gasUsed,
    'tx_type': txType,
    'sender': sender,
    'sequence_number': sequenceNumber,
    'gas_limit': gasLimit,
    'gas_price': gasPrice,
    'module': module,
    'function': function,
    'module_functions': moduleFunctions,
  };

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
