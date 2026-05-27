import 'package:json_annotation/json_annotation.dart';
import 'package:equatable/equatable.dart';

part 'block.g.dart';

@JsonSerializable()
class BlockInfo extends Equatable {
  final int height;
  final int timestamp;
  final String hash;
  @JsonKey(name: 'prev_hash')
  final String prevHash;
  @JsonKey(name: 'tx_count')
  final int txCount;
  @JsonKey(name: 'state_root')
  final String stateRoot;
  final List<RpcEvent> events;

  const BlockInfo({
    required this.height,
    required this.timestamp,
    required this.hash,
    required this.prevHash,
    required this.txCount,
    required this.stateRoot,
    required this.events,
  });

  factory BlockInfo.fromJson(Map<String, dynamic> json) =>
      _$BlockInfoFromJson(json);

  Map<String, dynamic> toJson() => _$BlockInfoToJson(this);

  @override
  List<Object?> get props => [
    height,
    timestamp,
    hash,
    prevHash,
    txCount,
    stateRoot,
    events,
  ];
}

@JsonSerializable()
class RpcEvent extends Equatable {
  final List<int> key;
  @JsonKey(name: 'sequence_number')
  final int sequenceNumber;
  @JsonKey(name: 'type_tag')
  final String typeTag;
  @JsonKey(name: 'event_data')
  final List<int> eventData;

  const RpcEvent({
    required this.key,
    required this.sequenceNumber,
    required this.typeTag,
    required this.eventData,
  });

  factory RpcEvent.fromJson(Map<String, dynamic> json) =>
      _$RpcEventFromJson(json);

  Map<String, dynamic> toJson() => _$RpcEventToJson(this);

  @override
  List<Object?> get props => [key, sequenceNumber, typeTag, eventData];
}
