// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'module.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

ModuleInfo _$ModuleInfoFromJson(Map<String, dynamic> json) => ModuleInfo(
  address: json['address'] as String,
  name: json['name'] as String,
  bytecodeHash: json['bytecode_hash'] as String,
  size: (json['size'] as num).toInt(),
  dependencies: (json['dependencies'] as List<dynamic>)
      .map((e) => e as String)
      .toList(),
);

Map<String, dynamic> _$ModuleInfoToJson(ModuleInfo instance) =>
    <String, dynamic>{
      'address': instance.address,
      'name': instance.name,
      'bytecode_hash': instance.bytecodeHash,
      'size': instance.size,
      'dependencies': instance.dependencies,
    };

VerifyModuleResult _$VerifyModuleResultFromJson(Map<String, dynamic> json) =>
    VerifyModuleResult(
      valid: json['valid'] as bool,
      address: json['address'] as String?,
      name: json['name'] as String?,
      error: json['error'] as String?,
    );

Map<String, dynamic> _$VerifyModuleResultToJson(VerifyModuleResult instance) =>
    <String, dynamic>{
      'valid': instance.valid,
      'address': instance.address,
      'name': instance.name,
      'error': instance.error,
    };
