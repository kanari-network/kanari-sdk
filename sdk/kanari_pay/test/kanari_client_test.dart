import 'package:flutter_test/flutter_test.dart';
import 'package:kanari_pay/kanari_pay.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:kanari_crypto/kanari_crypto.dart';
import 'dart:convert';
import 'dart:typed_data';

Map<String, dynamic> ownerResponse({
  required int sequenceNumber,
  Map<String, int> balances = const {},
  List<Map<String, dynamic>> ownedObjects = const [],
}) {
  return {
    'jsonrpc': '2.0',
    'result': {
      'owner': '0x123',
      'sequence_number': sequenceNumber,
      'modules': [],
      'balances': balances,
      'owned_objects': ownedObjects,
    },
    'id': 1,
  };
}

void main() {
  group('KanariClient', () {
    test('getHealth returns health status', () async {
      final mockClient = MockClient((request) async {
        return http.Response(
          jsonEncode({
            'jsonrpc': '2.0',
            'result': {
              'status': 'ok',
              'version': '0.1.0',
              'uptime_seconds': 100,
              'sync_status': 'synced',
            },
            'id': 1,
          }),
          200,
        );
      });

      final client = KanariClient(
        'http://localhost:19001/rpc',
        client: mockClient,
      );
      final health = await client.getHealth();

      expect(health.status, 'ok');
      expect(health.version, '0.1.0');
      expect(health.uptimeSeconds, 100);
      expect(health.syncStatus, 'synced');
    });

    test('fromEnvironment uses correct URL', () {
      final clientDev = KanariClient.fromEnvironment(KanariEnvironment.dev);
      expect(clientDev.url, 'http://192.168.1.103:19001');

      final clientLocal = KanariClient.fromEnvironment(KanariEnvironment.local);
      expect(clientLocal.url, 'http://127.0.0.1:6767/rpc');
    });

    test('transfer signs and submits correctly', () async {
      final mockWallet = KanariWallet(
        KeyPairData(
          privateKey: 'priv',
          publicKey: 'pub',
          address: '0x123',
          taggedAddress: 'Ed25519:0x123',
          rawPublicKey: Uint8List(32),
          curveType: 'Ed25519',
        ),
      );

      Map<String, dynamic>? capturedParams;

      final mockClient = MockClient((request) async {
        final body = jsonDecode(request.body);
        final method = body['method'];

        if (method == 'kanari_getOwner') {
          return http.Response(
            jsonEncode(
              ownerResponse(
                sequenceNumber: 5,
                balances: {'0x2::kanari::KANARI': 7000},
                ownedObjects: [
                  {
                    'id':
                        '0x0000000000000000000000000000000000000000000000000000000000000aaa',
                    'owner': '0x123',
                    'type_': '0x2::coin::Coin<0x2::kanari::KANARI>',
                    'data': [
                      ...List<int>.filled(32, 0),
                      136,
                      19,
                      0,
                      0,
                      0,
                      0,
                      0,
                      0,
                    ],
                    'version': 1,
                  },
                  {
                    'id':
                        '0x0000000000000000000000000000000000000000000000000000000000000abc',
                    'owner': '0x123',
                    'type_': '0x2::coin::Coin<0x2::kanari::KANARI>',
                    'data': [
                      ...List<int>.filled(32, 0),
                      208,
                      7,
                      0,
                      0,
                      0,
                      0,
                      0,
                      0,
                    ],
                    'version': 1,
                  },
                ],
              ),
            ),
            200,
          );
        }
        if (method == 'kanari_callFunction') {
          capturedParams = body['params'] as Map<String, dynamic>;
          return http.Response(
            jsonEncode({
              'jsonrpc': '2.0',
              'result': {
                'hash': '0xtxhash',
                'status': 'success',
                'gas_used': 100,
              },
              'id': 2,
            }),
            200,
          );
        }
        return http.Response('', 404);
      });

      final client = KanariClient('http://localhost/rpc', client: mockClient);
      final result = await client.transfer(
        wallet: mockWallet,
        recipient: '0x456',
        amount: 1000,
        gasLimit: 1,
        gasPrice: 1,
      );

      expect(result.hash, '0xtxhash');
      expect(result.status, 'success');

      // Verify KANARI transfer uses the Coin<KANARI> entry function.
      expect(capturedParams, isNotNull);
      final txData = capturedParams!;
      expect(txData['sender'], 'Ed25519:0x123');
      expect(txData['package'], '0x2');
      expect(txData['module'], 'kanari');
      expect(txData['function'], 'transfer');
      expect(txData['sequence_number'], 5);
      expect(txData['args'], hasLength(3));
      expect(
        (txData['args'] as List)[0],
        [
          ...List<int>.filled(30, 0),
          10,
          0xbc,
        ],
      );
      expect(txData['signature'], isA<List>());
    });

    test('getOwnedObjects returns object-centric owner state', () async {
      final mockClient = MockClient((request) async {
        final body = jsonDecode(request.body);
        final method = body['method'];

        if (method == 'kanari_getOwnedObjects') {
          return http.Response(
            jsonEncode({
              'jsonrpc': '2.0',
              'result': {
                'objects': [
                  {
                    'id':
                        '0x0000000000000000000000000000000000000000000000000000000000000abc',
                    'owner': '0x123',
                    'type_': '0x2::coin::Coin<0x2::kanari::KANARI>',
                    'data': [...List<int>.filled(40, 0)],
                    'version': 7,
                  },
                ],
              },
              'id': 1,
            }),
            200,
          );
        }

        return http.Response('', 404);
      });

      final client = KanariClient('http://localhost/rpc', client: mockClient);
      final objects = await client.getOwnedObjects(
        '0x123',
        objectType: '0x2::coin::Coin<0x2::kanari::KANARI>',
      );

      expect(objects, hasLength(1));
      expect(
        objects.first.id,
        '0x0000000000000000000000000000000000000000000000000000000000000abc',
      );
      expect(objects.first.version, 7);
    });

    test('getObject returns one on-chain object', () async {
      final mockClient = MockClient((request) async {
        final body = jsonDecode(request.body);
        final method = body['method'];

        if (method == 'kanari_getObject') {
          return http.Response(
            jsonEncode({
              'jsonrpc': '2.0',
              'result': {
                'id':
                    '0x0000000000000000000000000000000000000000000000000000000000000abc',
                'owner': '0x123',
                'type_': '0x2::coin::Coin<0x2::kanari::KANARI>',
                'data': [...List<int>.filled(40, 0)],
                'version': 9,
              },
              'id': 1,
            }),
            200,
          );
        }

        return http.Response('', 404);
      });

      final client = KanariClient('http://localhost/rpc', client: mockClient);
      final object = await client.getObject(
        '0x0000000000000000000000000000000000000000000000000000000000000abc',
      );

      expect(object.owner, '0x123');
      expect(object.version, 9);
    });

    test('publishModule signs and submits correctly', () async {
      final mockWallet = KanariWallet(
        KeyPairData(
          privateKey: 'priv',
          publicKey: 'pub',
          address: '0x123',
          taggedAddress: 'Ed25519:0x123',
          rawPublicKey: Uint8List(32),
          curveType: 'Ed25519',
        ),
      );

      Map<String, dynamic>? capturedParams;

      final mockClient = MockClient((request) async {
        final body = jsonDecode(request.body);
        final method = body['method'];

        if (method == 'kanari_getOwner') {
          return http.Response(
            jsonEncode(ownerResponse(sequenceNumber: 10)),
            200,
          );
        }
        if (method == 'kanari_publishModule') {
          capturedParams = body['params'] as Map<String, dynamic>;
          return http.Response(
            jsonEncode({
              'jsonrpc': '2.0',
              'result': {
                'hash': '0xpubhash',
                'status': 'success',
                'gas_used': 500,
              },
              'id': 2,
            }),
            200,
          );
        }
        return http.Response('', 404);
      });

      final client = KanariClient('http://localhost/rpc', client: mockClient);
      final result = await client.publishModule(
        wallet: mockWallet,
        moduleBytes: [1, 2, 3],
        moduleName: 'TestModule',
      );

      expect(result.hash, '0xpubhash');
      expect(result.status, 'success');

      // Verify params
      expect(capturedParams, isNotNull);
      expect(capturedParams!['sender'], 'Ed25519:0x123');
      expect(capturedParams!['module_bytes'], [1, 2, 3]);
      expect(capturedParams!['module_name'], 'TestModule');
      expect(capturedParams!['sequence_number'], 10);
      expect(capturedParams!['signature'], isA<List>());
    });

    test('executeFunction signs and submits correctly', () async {
      final mockWallet = KanariWallet(
        KeyPairData(
          privateKey: 'priv',
          publicKey: 'pub',
          address: '0x123',
          taggedAddress: 'Ed25519:0x123',
          rawPublicKey: Uint8List(32),
          curveType: 'Ed25519',
        ),
      );

      Map<String, dynamic>? capturedParams;

      final mockClient = MockClient((request) async {
        final body = jsonDecode(request.body);
        final method = body['method'];

        if (method == 'kanari_getOwner') {
          return http.Response(
            jsonEncode(ownerResponse(sequenceNumber: 15)),
            200,
          );
        }
        if (method == 'kanari_callFunction') {
          capturedParams = body['params'] as Map<String, dynamic>;
          return http.Response(
            jsonEncode({
              'jsonrpc': '2.0',
              'result': {
                'hash': '0xcallhash',
                'status': 'success',
                'gas_used': 200,
              },
              'id': 2,
            }),
            200,
          );
        }
        return http.Response('', 404);
      });

      final client = KanariClient('http://localhost/rpc', client: mockClient);
      final result = await client.executeFunction(
        wallet: mockWallet,
        package: '0x1',
        module: 'test',
        function: 'run',
        args: [
          [1],
          [2],
        ],
      );

      expect(result.hash, '0xcallhash');
      expect(result.status, 'success');

      // Verify params
      expect(capturedParams, isNotNull);
      expect(capturedParams!['sender'], 'Ed25519:0x123');
      expect(capturedParams!['package'], '0x1');
      expect(capturedParams!['module'], 'test');
      expect(capturedParams!['function'], 'run');
      expect(capturedParams!['args'], [
        [1],
        [2],
      ]);
      expect(capturedParams!['sequence_number'], 15);
      expect(capturedParams!['signature'], isA<List>());
    });

    test('burn signs and submits correctly', () async {
      final mockWallet = KanariWallet(
        KeyPairData(
          privateKey: 'priv',
          publicKey: 'pub',
          address: '0x123',
          taggedAddress: 'Ed25519:0x123',
          rawPublicKey: Uint8List(32),
          curveType: 'Ed25519',
        ),
      );

      Map<String, dynamic>? capturedParams;

      final mockClient = MockClient((request) async {
        final body = jsonDecode(request.body);
        final method = body['method'];

        if (method == 'kanari_getOwner') {
          return http.Response(
            jsonEncode(ownerResponse(sequenceNumber: 20)),
            200,
          );
        }
        if (method == 'kanari_callFunction') {
          capturedParams = body['params'] as Map<String, dynamic>;
          return http.Response(
            jsonEncode({
              'jsonrpc': '2.0',
              'result': {
                'hash': '0xburnhash',
                'status': 'success',
                'gas_used': 50,
              },
              'id': 2,
            }),
            200,
          );
        }
        return http.Response('', 404);
      });

      final client = KanariClient('http://localhost/rpc', client: mockClient);
      final result = await client.burn(wallet: mockWallet, amount: 500);

      expect(result.hash, '0xburnhash');
      expect(result.status, 'success');

      // Verify params
      expect(capturedParams, isNotNull);
      expect(capturedParams!['sender'], 'Ed25519:0x123');
      expect(capturedParams!['package'], '0x2');
      expect(capturedParams!['module'], 'kanari');
      expect(capturedParams!['function'], 'burn_amount');
      expect(capturedParams!['args'], [
        [244, 1, 0, 0, 0, 0, 0, 0],
      ]);
      expect(capturedParams!['sequence_number'], 20);
      expect(capturedParams!['signature'], isA<List>());
    });

    test('transfer consolidates fragmented native coins before sending', () async {
      final mockWallet = KanariWallet(
        KeyPairData(
          privateKey: 'priv',
          publicKey: 'pub',
          address: '0x123',
          taggedAddress: 'Ed25519:0x123',
          rawPublicKey: Uint8List(32),
          curveType: 'Ed25519',
        ),
      );

      final submittedCalls = <Map<String, dynamic>>[];

      final mockClient = MockClient((request) async {
        final body = jsonDecode(request.body);
        final method = body['method'];

        if (method == 'kanari_getOwner') {
          return http.Response(
            jsonEncode(
              ownerResponse(
                sequenceNumber: 9,
                balances: {'0x2::kanari::KANARI': 4500},
                ownedObjects: [
                  {
                    'id':
                        '0x00000000000000000000000000000000000000000000000000000000000000aa',
                    'owner': '0x123',
                    'type_': '0x2::coin::Coin<0x2::kanari::KANARI>',
                    'data': [
                      ...List<int>.filled(32, 0),
                      208,
                      7,
                      0,
                      0,
                      0,
                      0,
                      0,
                      0,
                    ],
                    'version': 1,
                  },
                  {
                    'id':
                        '0x00000000000000000000000000000000000000000000000000000000000000bb',
                    'owner': '0x123',
                    'type_': '0x2::coin::Coin<0x2::kanari::KANARI>',
                    'data': [
                      ...List<int>.filled(32, 0),
                      196,
                      9,
                      0,
                      0,
                      0,
                      0,
                      0,
                      0,
                    ],
                    'version': 1,
                  },
                ],
              ),
            ),
            200,
          );
        }

        if (method == 'kanari_callFunction') {
          submittedCalls.add(body['params'] as Map<String, dynamic>);
          return http.Response(
            jsonEncode({
              'jsonrpc': '2.0',
              'result': {
                'hash': '0x${submittedCalls.length}',
                'status': 'success',
                'gas_used': 100,
              },
              'id': 2,
            }),
            200,
          );
        }

        return http.Response('', 404);
      });

      final client = KanariClient('http://localhost/rpc', client: mockClient);
      final result = await client.transfer(
        wallet: mockWallet,
        recipient: '0x456',
        amount: 3000,
        gasLimit: 1000,
        gasPrice: 1,
      );

      expect(result.hash, '0x2');
      expect(submittedCalls, hasLength(2));
      expect(submittedCalls[0]['module'], 'coin');
      expect(submittedCalls[0]['function'], 'join_entry');
      expect(submittedCalls[0]['sequence_number'], 9);
      expect(submittedCalls[1]['module'], 'kanari');
      expect(submittedCalls[1]['function'], 'transfer');
      expect(submittedCalls[1]['sequence_number'], 10);
    });
  });
}
