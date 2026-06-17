import 'package:flutter_test/flutter_test.dart';
import 'package:bcs/bcs.dart';

void main() {
  group('BCS Package Examples', () {
    test('Vectors', () {
      final intList = Bcs.vector(Bcs.u8()).serialize([1, 2, 3, 4, 5]).toBytes();
      final stringList = Bcs.vector(
        Bcs.string(),
      ).serialize(['a', 'b', 'c']).toBytes();

      final parsedIntList = Bcs.vector(Bcs.u8()).parse(intList);
      final parsedStringList = Bcs.vector(Bcs.string()).parse(stringList);

      expect(parsedIntList, [1, 2, 3, 4, 5]);
      expect(parsedStringList, ['a', 'b', 'c']);
    });

    test('Arrays', () {
      final intArray = Bcs.fixedArray(
        4,
        Bcs.u8(),
      ).serialize([1, 2, 3, 4]).toBytes();
      final stringArray = Bcs.fixedArray(
        3,
        Bcs.string(),
      ).serialize(['a', 'b', 'c']).toBytes();

      final parsedIntArray = Bcs.fixedArray(4, Bcs.u8()).parse(intArray);
      final parsedStringArray = Bcs.fixedArray(
        3,
        Bcs.string(),
      ).parse(stringArray);

      expect(parsedIntArray, [1, 2, 3, 4]);
      expect(parsedStringArray, ['a', 'b', 'c']);
    });

    test('Option', () {
      final option = Bcs.option(Bcs.string()).serialize('some value').toBytes();
      final nullOption = Bcs.option(Bcs.string()).serialize(null).toBytes();

      final parsedOption = Bcs.option(Bcs.string()).parse(option);
      final parsedNullOption = Bcs.option(Bcs.string()).parse(nullOption);

      expect(parsedOption, 'some value');
      expect(parsedNullOption, null);
    });

    test('Enum', () {
      final myEnum = Bcs.enumeration('MyEnum', {
        "NoType": null,
        "Int": Bcs.u8(),
        "String": Bcs.string(),
        "Array": Bcs.fixedArray(3, Bcs.u8()),
      });

      final noTypeEnum = myEnum.serialize({"NoType": null}).toBytes();
      final intEnum = myEnum.serialize({"Int": 100}).toBytes();
      final stringEnum = myEnum.serialize({"String": 'string'}).toBytes();
      final arrayEnum = myEnum.serialize({
        "Array": [1, 2, 3],
      }).toBytes();

      final parsedNoTypeEnum = myEnum.parse(noTypeEnum);
      final parsedIntEnum = myEnum.parse(intEnum);
      final parsedStringEnum = myEnum.parse(stringEnum);
      final parsedArrayEnum = myEnum.parse(arrayEnum);

      expect(parsedNoTypeEnum, {"NoType": true, "\$kind": "NoType"});
      expect(parsedIntEnum, {"Int": 100, "\$kind": "Int"});
      expect(parsedStringEnum, {"String": 'string', "\$kind": "String"});
      expect(parsedArrayEnum, {
        "Array": [1, 2, 3],
        "\$kind": "Array",
      });
    });

    test('Struct', () {
      final myStruct = Bcs.struct('MyStruct', {
        "id": Bcs.u8(),
        "name": Bcs.string(),
      });

      final struct = myStruct.serialize({"id": 1, "name": 'name'}).toBytes();
      final parsedStruct = myStruct.parse(struct);

      expect(parsedStruct, {"id": 1, "name": 'name'});
    });

    test('Tuple', () {
      final tuple = Bcs.tuple([
        Bcs.u8(),
        Bcs.string(),
      ]).serialize([1, 'name']).toBytes();
      final parsedTuple = Bcs.tuple([Bcs.u8(), Bcs.string()]).parse(tuple);

      expect(parsedTuple, [1, 'name']);
    });

    test('Map', () {
      final map = Bcs.map(
        Bcs.u8(),
        Bcs.string(),
      ).serialize({1: 'one', 2: 'two'}).toBytes();

      final parsedMap = Bcs.map(Bcs.u8(), Bcs.string()).parse(map);

      expect(parsedMap, {1: 'one', 2: 'two'});
    });
  });
}
