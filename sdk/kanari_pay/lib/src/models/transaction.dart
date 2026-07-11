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
  final TransactionEffectsInfo? effects;
  final String? errorMessage;
  final String? action;

  const TransactionResult({
    required this.hash,
    required this.status,
    this.gasUsed,
    this.effects,
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
      effects: json['effects'] is Map<String, dynamic>
          ? TransactionEffectsInfo.fromJson(
              json['effects'] as Map<String, dynamic>,
            )
          : null,
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
    'effects': effects?.toJson(),
    'error_message': errorMessage,
    'action': action,
  };

  @override
  List<Object?> get props => [
    hash,
    status,
    gasUsed,
    effects,
    errorMessage,
    action,
  ];
}

class TransactionDetails extends Equatable {
  final String hash;
  final String status;
  final int? blockHeight;
  final int? checkpointHeight;
  final int? gasUsed;
  final String txType;
  final String sender;
  final String? senderAddress;
  final int nonce;
  final int gasLimit;
  final int gasPrice;
  final String? module;
  final String? function;
  final List<String>? moduleFunctions;
  final TransactionEffectsInfo? effects;

  const TransactionDetails({
    required this.hash,
    required this.status,
    this.blockHeight,
    this.checkpointHeight,
    this.gasUsed,
    required this.txType,
    required this.sender,
    this.senderAddress,
    required this.nonce,
    required this.gasLimit,
    required this.gasPrice,
    this.module,
    this.function,
    this.moduleFunctions,
    this.effects,
  });

  factory TransactionDetails.fromJson(Map<String, dynamic> json) {
    return TransactionDetails(
      hash: json['hash']?.toString() ?? '',
      status: json['status']?.toString() ?? '',
      blockHeight:
          json['checkpoint_height'] == null && json['block_height'] == null
          ? null
          : _jsonInt(json['checkpoint_height'] ?? json['block_height']),
      checkpointHeight:
          json['checkpoint_height'] == null && json['block_height'] == null
          ? null
          : _jsonInt(json['checkpoint_height'] ?? json['block_height']),
      gasUsed: json['gas_used'] == null ? null : _jsonInt(json['gas_used']),
      txType: json['tx_type']?.toString() ?? '',
      sender: json['sender']?.toString() ?? '',
      senderAddress: json['sender_address']?.toString(),
      nonce: _jsonInt(json['nonce']),
      gasLimit: _jsonInt(json['gas_limit']),
      gasPrice: _jsonInt(json['gas_price']),
      module: json['module']?.toString(),
      function: json['function']?.toString(),
      moduleFunctions: (json['module_functions'] as List<dynamic>?)
          ?.map((item) => item.toString())
          .toList(),
      effects: json['effects'] is Map<String, dynamic>
          ? TransactionEffectsInfo.fromJson(
              json['effects'] as Map<String, dynamic>,
            )
          : null,
    );
  }

  Map<String, dynamic> toJson() => {
    'hash': hash,
    'status': status,
    'block_height': blockHeight,
    'checkpoint_height': checkpointHeight ?? blockHeight,
    'gas_used': gasUsed,
    'tx_type': txType,
    'sender': sender,
    'sender_address': senderAddress,
    'nonce': nonce,
    'gas_limit': gasLimit,
    'gas_price': gasPrice,
    'module': module,
    'function': function,
    'module_functions': moduleFunctions,
    'effects': effects?.toJson(),
  };

  @override
  List<Object?> get props => [
    hash,
    status,
    blockHeight,
    checkpointHeight,
    gasUsed,
    txType,
    sender,
    senderAddress,
    nonce,
    gasLimit,
    gasPrice,
    module,
    function,
    moduleFunctions,
    effects,
  ];
}

class ObjectRefInfo extends Equatable {
  final String objectId;
  final int? version;
  final String? digest;

  const ObjectRefInfo({
    required this.objectId,
    this.version,
    this.digest,
  });

  factory ObjectRefInfo.fromJson(Map<String, dynamic> json) {
    return ObjectRefInfo(
      objectId: json['object_id']?.toString() ?? json['id']?.toString() ?? '',
      version: json['version'] == null ? null : _jsonInt(json['version']),
      digest: json['digest']?.toString(),
    );
  }

  Map<String, dynamic> toJson() => {
    'object_id': objectId,
    'version': version,
    'digest': digest,
  };

  @override
  List<Object?> get props => [objectId, version, digest];
}

class ObjectChangeInfo extends Equatable {
  final String changeType;
  final ObjectRefInfo objectRef;
  final String? objectType;
  final String? owner;

  const ObjectChangeInfo({
    required this.changeType,
    required this.objectRef,
    this.objectType,
    this.owner,
  });

  factory ObjectChangeInfo.fromJson(Map<String, dynamic> json) {
    return ObjectChangeInfo(
      changeType: json['change_type']?.toString() ?? '',
      objectRef: ObjectRefInfo.fromJson(
        json['object_ref'] as Map<String, dynamic>? ?? <String, dynamic>{},
      ),
      objectType: json['type_']?.toString() ?? json['type']?.toString(),
      owner: _ownerToString(json['owner']),
    );
  }

  Map<String, dynamic> toJson() => {
    'change_type': changeType,
    'object_ref': objectRef.toJson(),
    'type_': objectType,
    'owner': owner,
  };

  static String? _ownerToString(dynamic value) {
    if (value == null) return null;
    if (value is String) return value;
    if (value is Map<String, dynamic>) {
      final addressOwner = value['AddressOwner'] ?? value['address_owner'];
      if (addressOwner != null) return addressOwner.toString();
    }
    return value.toString();
  }

  @override
  List<Object?> get props => [changeType, objectRef, objectType, owner];
}

class TransactionEffectsInfo extends Equatable {
  final String status;
  final int gasUsed;
  final List<ObjectChangeInfo> objectChanges;

  const TransactionEffectsInfo({
    required this.status,
    required this.gasUsed,
    required this.objectChanges,
  });

  factory TransactionEffectsInfo.fromJson(Map<String, dynamic> json) {
    final changes = <ObjectChangeInfo>[];
    for (final key in const [
      'object_changes',
      'created',
      'mutated',
      'transferred',
    ]) {
      final raw = json[key];
      if (raw is List) {
        changes.addAll(
          raw
              .whereType<Map<String, dynamic>>()
              .map(ObjectChangeInfo.fromJson),
        );
      }
    }

    final deduped = <String, ObjectChangeInfo>{};
    for (final change in changes) {
      final key =
          '${change.objectRef.objectId}|${change.objectType}|${change.changeType}';
      deduped[key] = change;
    }

    return TransactionEffectsInfo(
      status: json['status']?.toString() ?? '',
      gasUsed: _jsonInt(json['gas_used']),
      objectChanges: deduped.values.toList(),
    );
  }

  Map<String, dynamic> toJson() => {
    'status': status,
    'gas_used': gasUsed,
    'object_changes': objectChanges.map((item) => item.toJson()).toList(),
  };

  @override
  List<Object?> get props => [status, gasUsed, objectChanges];
}
