enum KanariEnvironment {
  local('http://127.0.0.1:6767'),
  dev('https://dev-seed.kanari.network'),
  test('https://test-seed.rooch.network'),
  main('https://main-seed.kanari.network');

  final String url;
  const KanariEnvironment(this.url);

  /// Returns the RPC endpoint URL (appends /rpc to the base URL)
  String get rpcUrl => '$url/rpc';
}
