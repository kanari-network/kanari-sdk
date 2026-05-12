import 'package:json_annotation/json_annotation.dart';
import 'package:equatable/equatable.dart';

part 'module.g.dart';

@JsonSerializable()
class ModuleInfo extends Equatable {
  final String address;
  final String name;
  @JsonKey(name: 'bytecode_hash')
  final String bytecodeHash;
  final int size;
  final List<String> dependencies;

  const ModuleInfo({
    required this.address,
    required this.name,
    required this.bytecodeHash,
    required this.size,
    required this.dependencies,
  });

  factory ModuleInfo.fromJson(Map<String, dynamic> json) =>
      _$ModuleInfoFromJson(json);

  Map<String, dynamic> toJson() => _$ModuleInfoToJson(this);

  @override
  List<Object?> get props => [address, name, bytecodeHash, size, dependencies];
}

@JsonSerializable()
class VerifyModuleResult extends Equatable {
  final bool valid;
  final String? address;
  final String? name;
  final String? error;

  const VerifyModuleResult({
    required this.valid,
    this.address,
    this.name,
    this.error,
  });

  factory VerifyModuleResult.fromJson(Map<String, dynamic> json) =>
      _$VerifyModuleResultFromJson(json);

  Map<String, dynamic> toJson() => _$VerifyModuleResultToJson(this);

  @override
  List<Object?> get props => [valid, address, name, error];
}
