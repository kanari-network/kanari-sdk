import 'package:flutter_test/flutter_test.dart';
import 'package:kanari_kit/kanari_kit.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:kanari_crypto/kanari_crypto.dart';
import 'dart:convert';
import 'dart:typed_data';

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
              'sync_status': 'synced'
            },
            'id': 1
          }),
          200,
        );
      });

      final client = KanariClient('http://localhost:19001/rpc', client: mockClient);
      final health = await client.getHealth();

      expect(health.status, 'ok');
      expect(health.version, '0.1.0');
      expect(health.uptimeSeconds, 100);
      expect(health.syncStatus, 'synced');
    });

    test('getBalance returns correct amount', () async {
      final mockClient = MockClient((request) async {
        return http.Response(
          jsonEncode({
            'jsonrpc': '2.0',
            'result': 1000,
            'id': 1
          }),
          200,
        );
      });

      final client = KanariClient('http://localhost:19001/rpc', client: mockClient);
      final balance = await client.getBalance('0x1');

      expect(balance, 1000);
    });

    test('fromEnvironment uses correct URL', () {
      final clientDev = KanariClient.fromEnvironment(KanariEnvironment.dev);
      expect(clientDev.url, 'https://dev-seed.kanari.network/rpc');

      final clientLocal = KanariClient.fromEnvironment(KanariEnvironment.local);
      expect(clientLocal.url, 'http://127.0.0.1:6767/rpc');
    });

    test('transfer signs and submits correctly', () async {
      final mockWallet = KanariWallet(KeyPairData(
        privateKey: 'priv',
        publicKey: 'pub',
        address: '0x123',
        rawPublicKey: Uint8List(32),
        curveType: 'Ed25519',
      ));

      final mockClient = MockClient((request) async {
        final body = jsonDecode(request.body);
        final method = body['method'];

        if (method == 'kanari_getAccount') {
          return http.Response(
            jsonEncode({
              'jsonrpc': '2.0',
              'result': {
                'address': '0x123',
                'balance': 5000,
                'sequence_number': 5,
                'modules': [],
                'token_balances': {}
              },
              'id': 1
            }),
            200,
          );
        }
        if (method == 'kanari_submitTransaction') {
          return http.Response(
            jsonEncode({
              'jsonrpc': '2.0',
              'result': {
                'hash': '0xtxhash',
                'status': 'success',
                'gas_used': 100
              },
              'id': 2
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
      );

      expect(result.hash, '0xtxhash');
      expect(result.status, 'success');
    });
  });
}
