import 'package:flutter_test/flutter_test.dart';
import 'package:kanari_kit/kanari_kit.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'dart:convert';

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
  });
}
