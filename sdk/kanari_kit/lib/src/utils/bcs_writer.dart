import 'dart:convert';
import 'dart:typed_data';

class BcsWriter {
  final List<int> _bytes = [];

  Uint8List toBytes() => Uint8List.fromList(_bytes);

  void writeU8(int value) {
    _bytes.add(value & 0xFF);
  }

  void writeU32(int value) {
    final data = ByteData(4);
    data.setUint32(0, value, Endian.little);
    _bytes.addAll(data.buffer.asUint8List());
  }

  void writeU64(int value) {
    final data = ByteData(8);
    data.setUint64(0, value, Endian.little);
    _bytes.addAll(data.buffer.asUint8List());
  }

  void writeULEB128(int value) {
    while (value >= 0x80) {
      _bytes.add((value & 0x7F) | 0x80);
      value >>= 7;
    }
    _bytes.add(value & 0x7F);
  }

  void writeString(String value) {
    final encoded = utf8.encode(value);
    writeULEB128(encoded.length);
    _bytes.addAll(encoded);
  }

  void writeFixedBytes(List<int> bytes) {
    _bytes.addAll(bytes);
  }

  void writeVectorU8(List<int> bytes) {
    writeULEB128(bytes.length);
    _bytes.addAll(bytes);
  }
}
