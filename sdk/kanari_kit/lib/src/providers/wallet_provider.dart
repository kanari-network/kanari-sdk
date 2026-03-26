import 'package:flutter/material.dart';
import 'package:kanari_kit/kanari_kit.dart';

class WalletState extends ChangeNotifier {
  KanariClient? _client;
  KanariWallet? _wallet;
  List<Map<String, dynamic>> _wallets = [];

  int _balance = 0;
  List<TokenBalance> _tokenBalances = [];

  bool _isLoading = false;
  String? _error;
  String? _activeWalletId;
  KanariEnvironment _environment = KanariEnvironment.local;

  bool _isUnlocked = false;

  KanariClient? get client => _client;
  KanariWallet? get wallet => _wallet;
  List<Map<String, dynamic>> get wallets => _wallets;

  int get balance => _balance;
  List<TokenBalance> get tokenBalances => _tokenBalances;

  bool get isLoading => _isLoading;
  String? get error => _error;
  String? get activeWalletId => _activeWalletId;
  bool get hasWallet => _wallets.isNotEmpty;
  KanariEnvironment get environment => _environment;

  bool get isUnlocked => _isUnlocked;

  Future<void> initialize() async {
    _updateClient();
    _wallets = await WalletStorage.loadAllWallets();
    notifyListeners();
  }

  void _updateClient() {
    _client = KanariClient.fromEnvironment(_environment);
  }

  Future<void> setEnvironment(KanariEnvironment env) async {
    if (_environment == env) return;
    _environment = env;
    _updateClient();
    if (_wallet != null) {
      await refreshBalance();
    }
    notifyListeners();
  }

  Future<void> loadWallets() async {
    _wallets = await WalletStorage.loadAllWallets();
    if (_wallets.isNotEmpty) {
      await _loadActiveWallet();
    }
  }

  Future<void> _loadActiveWallet() async {
    final activeId = await WalletStorage.getActiveWalletId();
    _activeWalletId = activeId;
    Map<String, dynamic>? activeWalletData;

    if (activeId != null) {
      activeWalletData = _wallets.cast<Map<String, dynamic>?>().firstWhere(
        (w) => w?['id'] == activeId,
        orElse: () => null,
      );
    }

    activeWalletData ??= _wallets.isNotEmpty ? _wallets.first : null;

    if (activeWalletData != null) {
      await _instantiateWallet(activeWalletData);
      debugPrint('✅ Loaded active wallet: ${activeWalletData['name']}');
    }
  }

  Future<void> _instantiateWallet(Map<String, dynamic> data) async {
    final curve = KanariCurve.values.firstWhere(
      (c) => c.name == data['curve'],
      orElse: () => KanariCurve.ed25519,
    );

    if (data['mnemonic'] != null &&
        data['mnemonic'].toString().isNotEmpty &&
        !curve.isPostQuantum) {
      _wallet = await KanariWallet.fromMnemonic(data['mnemonic'], curve: curve);
    } else {
      _wallet = await KanariWallet.fromPrivateKey(
        data['privateKey'],
        curve: curve,
      );
    }
    await refreshBalance();
  }

  Future<void> switchWallet(String walletId) async {
    final walletData = _wallets.cast<Map<String, dynamic>?>().firstWhere(
      (w) => w?['id'] == walletId,
      orElse: () => null,
    );

    if (walletData == null) return;

    await WalletStorage.setActiveWallet(walletId);
    _activeWalletId = walletId;
    await _instantiateWallet(walletData);
    notifyListeners();
  }

  Future<void> addWallet(Map<String, dynamic> walletData, [String? pin]) async {
    _wallets.add(walletData);

    // บันทึกรหัส PIN ลง Storage สำหรับกระเป๋าใบแรกสุด
    if (pin != null && pin.isNotEmpty && _wallets.length == 1) {
      await WalletStorage.savePassword(pin);
      debugPrint('🔐 Master PIN saved');
    }

    await WalletStorage.saveAllWallets(_wallets);
    await switchWallet(walletData['id']);
    notifyListeners();
  }

  Future<void> removeWallet(String walletId) async {
    _wallets.removeWhere((w) => w['id'] == walletId);
    await WalletStorage.saveAllWallets(_wallets);

    final activeId = await WalletStorage.getActiveWalletId();
    if (activeId == walletId && _wallets.isNotEmpty) {
      await switchWallet(_wallets.first['id']);
    } else if (_wallets.isEmpty) {
      _wallet = null;
      _balance = 0;
      _tokenBalances = [];
    }
    notifyListeners();
  }

  Future<void> createNewWallet({
    KanariCurve curve = KanariCurve.ed25519,
    required String pin,
  }) async {
    _setLoading(true);
    _error = null;
    try {
      final wallet = await KanariWallet.generate(curve: curve);
      final walletData = {
        'id': DateTime.now().millisecondsSinceEpoch.toString(),
        'name': 'Wallet ${_wallets.length + 1}',
        'mnemonic': wallet.mnemonic,
        'privateKey': wallet.privateKey,
        'curve': curve.name,
        'createdAt': DateTime.now().toIso8601String(),
      };

      await addWallet(walletData, pin);
      _isUnlocked = true;
      await refreshBalance();
    } catch (e) {
      _error = "Creation failed: $e";
    } finally {
      _setLoading(false);
    }
  }

  Future<void> unlockWallet(String pin) async {
    _setLoading(true);
    _error = null;
    try {
      final isValid = await WalletStorage.verifyPassword(pin);
      if (!isValid) {
        _error = "Invalid PIN";
        _setLoading(false);
        notifyListeners();
        return;
      }

      await loadWallets();

      if (_wallets.isEmpty) {
        _error = "No saved wallets";
        _setLoading(false);
        notifyListeners();
        return;
      }

      _isUnlocked = true;
      _error = null;
    } catch (e) {
      _error = "Unlock failed: $e";
    } finally {
      _setLoading(false);
      notifyListeners();
    }
  }

  Future<void> importFromPrivateKey(
    String pk, {
    KanariCurve curve = KanariCurve.ed25519,
    String? pin,
  }) async {
    _setLoading(true);
    _error = null;
    try {
      String cleanPk = pk.trim();
      final wallet = await KanariWallet.fromPrivateKey(cleanPk, curve: curve);

      final walletData = {
        'id': DateTime.now().millisecondsSinceEpoch.toString(),
        'name': 'Imported Wallet ${_wallets.length + 1}',
        'mnemonic': '',
        'privateKey': wallet.privateKey,
        'curve': curve.name,
        'createdAt': DateTime.now().toIso8601String(),
      };

      await addWallet(walletData, pin);
      _isUnlocked = true;
      await refreshBalance();
    } catch (e) {
      _error = "Import PK failed: $e";
    } finally {
      _setLoading(false);
    }
  }

  Future<void> importFromMnemonic(
    String mnemonic, {
    KanariCurve curve = KanariCurve.ed25519,
    String? pin,
  }) async {
    _setLoading(true);
    _error = null;
    try {
      final wallet = await KanariWallet.fromMnemonic(mnemonic, curve: curve);

      final walletData = {
        'id': DateTime.now().millisecondsSinceEpoch.toString(),
        'name': 'Imported Wallet ${_wallets.length + 1}',
        'mnemonic': mnemonic,
        'privateKey': wallet.privateKey,
        'curve': curve.name,
        'createdAt': DateTime.now().toIso8601String(),
      };

      await addWallet(walletData, pin);
      _isUnlocked = true;
      await refreshBalance();
    } catch (e) {
      _error = "Import Mnemonic failed: $e";
    } finally {
      _setLoading(false);
    }
  }

  void logout() {
    _wallet = null;
    _balance = 0;
    _tokenBalances = [];
    _error = null;
    _isUnlocked = false;
    notifyListeners();
  }

  Future<void> deleteAllWallets() async {
    await WalletStorage.deleteAllWallets();
    _wallets = [];
    _wallet = null;
    _balance = 0;
    _tokenBalances = [];
    _error = null;
    _isUnlocked = false;
    notifyListeners();
  }

  Future<void> refreshBalance() async {
    if (_client != null && _wallet != null) {
      try {
        _balance = await _client!.getBalance(_wallet!.address);
        try {
          _tokenBalances = await _client!.getAllBalances(_wallet!.address);
        } catch (e) {
          _tokenBalances = [];
        }
        _error = null;
        notifyListeners();
      } catch (e) {
        _error = "Refresh balance failed: $e";
        notifyListeners();
      }
    }
  }

  Future<String?> transfer(String recipient, int amount) async {
    if (_client == null || _wallet == null) return "Client not initialized";
    _setLoading(true);
    try {
      final result = await _client!.transfer(
        wallet: _wallet!,
        recipient: recipient,
        amount: amount,
      );
      await refreshBalance();
      _setLoading(false);
      return "Success: Hash ${result.hash}";
    } catch (e) {
      _setLoading(false);
      return "Error: $e";
    }
  }

  Future<String?> executeFunction({
    required String packageAddress,
    required String module,
    required String function,
    List<String> typeArgs = const [],
    List<List<int>> args = const [],
  }) async {
    if (_client == null || _wallet == null) return "Client not initialized";
    _setLoading(true);
    try {
      final result = await _client!.executeFunction(
        wallet: _wallet!,
        package: packageAddress,
        module: module,
        function: function,
        typeArgs: typeArgs,
        args: args,
      );
      await refreshBalance();
      _setLoading(false);
      return "Success: Hash ${result.hash}";
    } catch (e) {
      _setLoading(false);
      return "Error: $e";
    }
  }

  Future<String?> burn(int amount) async {
    if (_client == null || _wallet == null) return "Client not initialized";
    _setLoading(true);
    try {
      final result = await _client!.burn(wallet: _wallet!, amount: amount);
      await refreshBalance();
      _setLoading(false);
      return "Success: Hash ${result.hash}";
    } catch (e) {
      _setLoading(false);
      return "Error: $e";
    }
  }

  void _setLoading(bool value) {
    _isLoading = value;
    notifyListeners();
  }

  // 👉 แก้ไขให้เปลี่ยนเป็น changePin
  Future<bool> changePin(String oldPin, String newPin) async {
    try {
      final isValid = await WalletStorage.verifyPassword(oldPin);
      if (!isValid) return false;

      await WalletStorage.savePassword(newPin);
      await WalletStorage.saveAllWallets(_wallets);

      notifyListeners();
      return true;
    } catch (e) {
      return false;
    }
  }

  Future<String?> transferToken(
    String recipient,
    String tokenType,
    int amount,
  ) async {
    if (_client == null || _wallet == null) return "Client not initialized";
    _setLoading(true);
    try {
      final result = await _client!.transferToken(
        wallet: _wallet!,
        recipient: recipient,
        tokenType: tokenType,
        amount: amount,
      );
      await refreshBalance();
      _setLoading(false);
      return "Success: Hash ${result.hash}";
    } catch (e) {
      _setLoading(false);
      return "Error: $e";
    }
  }
}
