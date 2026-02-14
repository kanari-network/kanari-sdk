import 'package:flutter_test/flutter_test.dart';
import 'package:kanari_kit/src/utils/bcs_writer.dart';

void main() {
  test('BcsWriter Serialization for Transaction::Transfer', () {
    final writer = BcsWriter();

    // Variant 2: Transfer (ULEB128)
    writer.writeULEB128(2);
    writer.writeString('address1');
    writer.writeString('address2');
    writer.writeU64(1000);
    writer.writeU64(2000);
    writer.writeU64(1);
    writer.writeU64(0);

    final bytes = writer.toBytes();
    print(
      'Encoded bytes: ${bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join(' ')}',
    );

    // Expected:
    // 02 (Variant 2 as ULEB128)
    // 08 (length 8) + address1 (61 64 64 72 65 73 73 31)
    // 08 (length 8) + address2 (61 64 64 72 65 73 73 32)
    // e8 03 00 00 00 00 00 00 (1000 as u64 LE)
    // d0 07 00 00 00 00 00 00 (2000 as u64 LE)
    // 01 00 00 00 00 00 00 00 (1 as u64 LE)
    // 00 00 00 00 00 00 00 00 (0 as u64 LE)

    final expectedHex =
        '02086164647265737331086164647265737332e803000000000000d00700000000000001000000000000000000000000000000';
    final actualHex = bytes
        .map((b) => b.toRadixString(16).padLeft(2, '0'))
        .join('');

    expect(actualHex, expectedHex);
  });
}
