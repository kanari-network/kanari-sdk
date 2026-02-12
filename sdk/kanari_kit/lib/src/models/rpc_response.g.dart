// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'rpc_response.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

RpcResponse<T> _$RpcResponseFromJson<T>(
  Map<String, dynamic> json,
  T Function(Object? json) fromJsonT,
) => RpcResponse<T>(
  jsonrpc: json['jsonrpc'] as String,
  result: _$nullableGenericFromJson(json['result'], fromJsonT),
  error: json['error'] == null
      ? null
      : RpcError.fromJson(json['error'] as Map<String, dynamic>),
  id: json['id'],
);

Map<String, dynamic> _$RpcResponseToJson<T>(
  RpcResponse<T> instance,
  Object? Function(T value) toJsonT,
) => <String, dynamic>{
  'jsonrpc': instance.jsonrpc,
  'result': _$nullableGenericToJson(instance.result, toJsonT),
  'error': instance.error,
  'id': instance.id,
};

T? _$nullableGenericFromJson<T>(
  Object? input,
  T Function(Object? json) fromJson,
) => input == null ? null : fromJson(input);

Object? _$nullableGenericToJson<T>(
  T? input,
  Object? Function(T value) toJson,
) => input == null ? null : toJson(input);

RpcError _$RpcErrorFromJson(Map<String, dynamic> json) => RpcError(
  code: (json['code'] as num).toInt(),
  message: json['message'] as String,
  data: json['data'],
);

Map<String, dynamic> _$RpcErrorToJson(RpcError instance) => <String, dynamic>{
  'code': instance.code,
  'message': instance.message,
  'data': instance.data,
};
