// core/bcs_serializers.dart
/// BCS serialization utilities for Kanari SDK

import 'dart:typed_data';

class BcsSerializers {
  const BcsSerializers._();

  /// Convert hex string to bytes
  static List<int> hexToBytes(String hexStr) {
    // Remove 0x prefix if present
    final clean = hexStr.startsWith('0x') ? hexStr.substring(2) : hexStr;
    
    // Validate hex string
    if (clean.isEmpty) {
      throw ArgumentError('Empty hex string');
    }
    
    if (!RegExp(r'^[0-9a-fA-F]+$').hasMatch(clean)) {
      throw ArgumentError('Invalid hex string: $hexStr');
    }
    
    // Ensure even length by padding with leading zero if needed
    final padded = clean.length.isOdd ? '0$clean' : clean;
    
    List<int> bytes = [];
    for (int i = 0; i < padded.length; i += 2) {
      // Ensure we don't go out of bounds
      if (i + 2 > padded.length) {
        throw ArgumentError(
          'Invalid hex string length. Expected even number of characters, got ${clean.length}. '
          'Original: $hexStr, Cleaned: $clean, Padded: $padded'
        );
      }
      bytes.add(int.parse(padded.substring(i, i + 2), radix: 16));
    }
    return bytes;
  }

  /// Encode u64 to little-endian bytes (BCS format)
  static List<int> encodeU64(int value) {
    if (value < 0 || value > 0xFFFFFFFFFFFFFFFF) {
      throw ArgumentError('Value out of u64 range: $value');
    }
    final data = ByteData(8);
    data.setUint64(0, value, Endian.little);
    return data.buffer.asUint8List();
  }

  /// Encode string to BCS format (ULEB128 length prefix + UTF-8 bytes)
  static List<int> encodeString(String value) {
    final utf8Bytes = value.codeUnits;
    final lengthBytes = encodeULEB128(utf8Bytes.length);
    return [...lengthBytes, ...utf8Bytes];
  }

  /// Encode integer as ULEB128
  static List<int> encodeULEB128(int value) {
    if (value < 0) {
      throw ArgumentError('ULEB128 value must be non-negative: $value');
    }
    final bytes = <int>[];
    do {
      int byte = value & 0x7F;
      value >>= 7;
      if (value != 0) {
        byte |= 0x80;
      }
      bytes.add(byte);
    } while (value != 0);
    return bytes;
  }

  /// Normalize address to 0x followed by 64 hex characters
  static String normalizeAddress(String addr) {
    var clean = addr.startsWith('0x') ? addr.substring(2) : addr;

    // Validate hex characters
    if (!RegExp(r'^[0-9a-fA-F]+$').hasMatch(clean)) {
      throw ArgumentError('Invalid hexadecimal characters in address: $clean');
    }

    // CRITICAL: Address MUST be exactly 64 hex characters (32 bytes)
    if (clean.length != 64) {
      throw ArgumentError(
        'Address must be exactly 64 hex characters (32 bytes). '
        'Got ${clean.length} characters. '
        'Example: 0x${'1'.padLeft(64, '0')}',
      );
    }

    return '0x${clean.toLowerCase()}';
  }

  /// Normalize object ID
  static String normalizeObjectId(String objectId) {
    var clean = objectId.startsWith('0x') ? objectId.substring(2) : objectId;
    if (clean.isEmpty || !RegExp(r'^[0-9a-fA-F]+$').hasMatch(clean)) {
      throw ArgumentError('Invalid object ID format: $objectId');
    }
    clean = clean.padLeft(64, '0').toLowerCase();
    if (clean.length != 64) {
      throw ArgumentError(
        'Object ID must be 32 bytes (64 hex chars) after normalization. '
        'Got ${clean.length} characters for $objectId.',
      );
    }
    return '0x$clean';
  }

  /// Extract coin type from object type
  static String? extractCoinTypeFromObjectType(String objectType) {
    final start = objectType.indexOf('<');
    final end = objectType.lastIndexOf('>');
    if (start != -1 && end != -1) {
      final outer = objectType.substring(0, start);
      if (outer.endsWith('::coin::Coin') ||
          outer.endsWith('::coin::coin::Coin')) {
        return objectType.substring(start + 1, end);
      }
    }
    return null;
  }
}
