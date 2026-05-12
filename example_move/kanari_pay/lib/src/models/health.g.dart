// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'health.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

HealthStatus _$HealthStatusFromJson(Map<String, dynamic> json) => HealthStatus(
  status: json['status'] as String,
  version: json['version'] as String,
  uptimeSeconds: (json['uptime_seconds'] as num).toInt(),
  syncStatus: json['sync_status'] as String,
);

Map<String, dynamic> _$HealthStatusToJson(HealthStatus instance) =>
    <String, dynamic>{
      'status': instance.status,
      'version': instance.version,
      'uptime_seconds': instance.uptimeSeconds,
      'sync_status': instance.syncStatus,
    };
