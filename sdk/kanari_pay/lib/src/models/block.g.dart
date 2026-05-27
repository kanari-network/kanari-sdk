// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'block.dart';

// **************************************************************************
// JsonSerializableGenerator
// **************************************************************************

BlockInfo _$BlockInfoFromJson(Map<String, dynamic> json) => BlockInfo(
  height: (json['height'] as num).toInt(),
  timestamp: (json['timestamp'] as num).toInt(),
  hash: json['hash'] as String,
  prevHash: json['prev_hash'] as String,
  txCount: (json['tx_count'] as num).toInt(),
  stateRoot: json['state_root'] as String,
  events: (json['events'] as List<dynamic>)
      .map((e) => RpcEvent.fromJson(e as Map<String, dynamic>))
      .toList(),
);

Map<String, dynamic> _$BlockInfoToJson(BlockInfo instance) => <String, dynamic>{
  'height': instance.height,
  'timestamp': instance.timestamp,
  'hash': instance.hash,
  'prev_hash': instance.prevHash,
  'tx_count': instance.txCount,
  'state_root': instance.stateRoot,
  'events': instance.events,
};

RpcEvent _$RpcEventFromJson(Map<String, dynamic> json) => RpcEvent(
  key: (json['key'] as List<dynamic>).map((e) => (e as num).toInt()).toList(),
  sequenceNumber: (json['sequence_number'] as num).toInt(),
  typeTag: json['type_tag'] as String,
  eventData: (json['event_data'] as List<dynamic>)
      .map((e) => (e as num).toInt())
      .toList(),
);

Map<String, dynamic> _$RpcEventToJson(RpcEvent instance) => <String, dynamic>{
  'key': instance.key,
  'sequence_number': instance.sequenceNumber,
  'type_tag': instance.typeTag,
  'event_data': instance.eventData,
};
