import 'package:json_annotation/json_annotation.dart';
import 'package:equatable/equatable.dart';

part 'rpc_response.g.dart';

@JsonSerializable(genericArgumentFactories: true)
class RpcResponse<T> extends Equatable {
  final String jsonrpc;
  final T? result;
  final RpcError? error;
  final dynamic id;

  const RpcResponse({
    required this.jsonrpc,
    this.result,
    this.error,
    this.id,
  });

  factory RpcResponse.fromJson(
    Map<String, dynamic> json,
    T Function(Object? json) fromJsonT,
  ) =>
      _$RpcResponseFromJson(json, fromJsonT);

  Map<String, dynamic> toJson(Object? Function(T value) toJsonT) =>
      _$RpcResponseToJson(this, toJsonT);

  @override
  List<Object?> get props => [jsonrpc, result, error, id];
}

@JsonSerializable()
class RpcError extends Equatable {
  final int code;
  final String message;
  final dynamic data;

  const RpcError({
    required this.code,
    required this.message,
    this.data,
  });

  factory RpcError.fromJson(Map<String, dynamic> json) =>
      _$RpcErrorFromJson(json);

  Map<String, dynamic> toJson() => _$RpcErrorToJson(this);

  @override
  List<Object?> get props => [code, message, data];
}
