import 'package:json_annotation/json_annotation.dart';
import 'package:equatable/equatable.dart';

part 'health.g.dart';

@JsonSerializable()
class HealthStatus extends Equatable {
  final String status;
  final String version;
  @JsonKey(name: 'uptime_seconds')
  final int uptimeSeconds;
  @JsonKey(name: 'sync_status')
  final String syncStatus;

  const HealthStatus({
    required this.status,
    required this.version,
    required this.uptimeSeconds,
    required this.syncStatus,
  });

  factory HealthStatus.fromJson(Map<String, dynamic> json) =>
      _$HealthStatusFromJson(json);

  Map<String, dynamic> toJson() => _$HealthStatusToJson(this);

  @override
  List<Object?> get props => [status, version, uptimeSeconds, syncStatus];
}
