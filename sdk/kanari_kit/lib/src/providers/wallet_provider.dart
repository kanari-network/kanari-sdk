import 'package:flutter/material.dart';
import 'package:kanari_kit/kanari_kit.dart';

class WalletState extends ChangeNotifier {
  KanariClient? _client;
  KanariWallet? _wallet;
  List<Map<String, dynamic>> _wallets = [];
  int _balance = 0;
  bool _isLoading = false;
  String? _error;
  String? _activeWalletId; // Track active wallet ID
  KanariEnvironment _environment = KanariEnvironment.local;

  KanariClient? get client => _client;
  KanariWallet? get wallet => _wallet;
  List<Map<String, dynamic>> get wallets => _wallets;
  int get balance => _balance;
  bool get isLoading => _isLoading;
  String? get error => _error;
  String? get activeWalletId => _activeWalletId; // Getter for active wallet ID
  bool get hasWallet => _wallets.isNotEmpty;
  KanariEnvironment get environment => _environment;

  Future<void> initialize() async {
    _updateClient();
    await loadWallets();
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

  /// Load all wallets from storage
  Future<void> loadWallets() async {
    _wallets = await WalletStorage.loadAllWallets();
    if (_wallets.isNotEmpty) {
      // Load active wallet
      await _loadActiveWallet();
    }
  }

  /// Load active wallet
  Future<void> _loadActiveWallet() async {
    final activeId = await WalletStorage.getActiveWalletId();
    _activeWalletId = activeId; // Store active wallet ID
    Map<String, dynamic>? activeWalletData;

    if (activeId != null) {
      activeWalletData = _wallets.cast<Map<String, dynamic>?>().firstWhere(
        (w) => w?['id'] == activeId,
        orElse: () => null,
      );
    }

    // If no active wallet or not found, use first wallet
    activeWalletData ??= _wallets.isNotEmpty
        ? _wallets.first as Map<String, dynamic>
        : null;

    if (activeWalletData != null) {
      await _instantiateWallet(activeWalletData);
      debugPrint('✅ Loaded active wallet: ${activeWalletData['name']} (ID: $_activeWalletId)');
    }
  }

  /// Instantiate wallet from data
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

  /// Switch to a different wallet
  Future<void> switchWallet(String walletId) async {
    debugPrint('🔄 Switching to wallet ID: $walletId');
    
    final walletData = _wallets.cast<Map<String, dynamic>?>().firstWhere(
      (w) => w?['id'] == walletId,
      orElse: () => null,
    );

    if (walletData == null) {
      debugPrint('❌ Wallet not found: $walletId');
      return;
    }

    debugPrint('✅ Found wallet: ${walletData['name']} (ID: ${walletData['id']})');
    
    // Set active wallet in storage
    await WalletStorage.setActiveWallet(walletId);
    
    // Update active wallet ID in state
    _activeWalletId = walletId;
    debugPrint(' Updated _activeWalletId to: $walletId');
    
    // Instantiate the wallet with cryptographic data
    await _instantiateWallet(walletData);
    
    debugPrint('✅ Wallet switched successfully');
    debugPrint('   - Active wallet ID: $_activeWalletId');
    debugPrint('   - Current wallet address: ${_wallet?.address}');
    debugPrint('   - Total wallets: ${_wallets.length}');
    debugPrint('   - Will notify listeners to rebuild UI');
    
    // Notify listeners to rebuild UI with new active wallet
    notifyListeners();
    
    debugPrint('✅ Listeners notified - UI should rebuild now');
  }

  /// Add new wallet to the list
  Future<void> addWallet(Map<String, dynamic> walletData, [String? password]) async {
    _wallets.add(walletData);
    
    // Save password hash if provided (for new wallets)
    if (password != null && password.isNotEmpty) {
      await WalletStorage.savePassword(password);
      debugPrint('🔐 Password saved for wallet');
    }
    
    await WalletStorage.saveAllWallets(_wallets);
    await switchWallet(walletData['id']);
    debugPrint('✅ Wallet added: ${walletData['name']}');
    notifyListeners();
  }

  /// Remove wallet from the list
  Future<void> removeWallet(String walletId) async {
    _wallets.removeWhere((w) => w['id'] == walletId);
    await WalletStorage.saveAllWallets(_wallets);

    // If removed wallet was active, switch to first available
    final activeId = await WalletStorage.getActiveWalletId();
    if (activeId == walletId && _wallets.isNotEmpty) {
      await switchWallet(_wallets.first['id']);
    } else if (_wallets.isEmpty) {
      _wallet = null;
      _balance = 0;
    }

    notifyListeners();
  }

  Future<void> createNewWallet({
    KanariCurve curve = KanariCurve.ed25519,
    required String password,
  }) async {
    _setLoading(true);
    _error = null;
    try {
      debugPrint("Starting wallet generation with curve: ${curve.name}...");
      final wallet = await KanariWallet.generate(curve: curve);
      debugPrint("Wallet generated: ${wallet.address}");

      final walletData = {
        'id': DateTime.now().millisecondsSinceEpoch.toString(),
        'name': 'Wallet ${_wallets.length + 1}',
        'mnemonic': wallet.mnemonic,
        'privateKey': wallet.privateKey,
        'curve': curve.name,
        'createdAt': DateTime.now().toIso8601String(),
      };

      await addWallet(walletData, password);
      await refreshBalance();
    } catch (e, stack) {
      _error = "Creation failed: $e";
      debugPrint(_error);
      debugPrint(stack.toString());
    } finally {
      _setLoading(false);
    }
  }

  Future<void> unlockWallet(String password) async {
    _setLoading(true);
    _error = null;
    try {
      debugPrint('🔓 Unlocking wallet...');
      final data = await WalletStorage.loadWallet(password);
      if (data == null) {
        debugPrint('❌ Failed to load wallet - invalid password');
        _error = "Invalid password or no saved wallet";
        _setLoading(false);
        notifyListeners();
        return;
      }

      debugPrint('✅ Wallet loaded: ${data['name']}');
      debugPrint('📦 Wallet ID: ${data['id']}');
      debugPrint(' Wallet curve: ${data['curve']}');
      
      // Reload all wallets from storage to ensure _wallets list is populated
      debugPrint('🔄 Loading all wallets...');
      await loadWallets();
      debugPrint('✅ Wallets loaded: ${_wallets.length} wallet(s)');
      debugPrint('✅ Active wallet: ${_wallet?.address ?? "none"}');
      debugPrint('✅ Has wallet: ${hasWallet}');
      
      _error = null;
    } catch (e, stack) {
      debugPrint('❌ Unlock error: $e');
      debugPrint(stack.toString());
      _error = "Unlock failed: $e";
    } finally {
      _setLoading(false);
      debugPrint('🔔 Notifying listeners...');
      notifyListeners(); // Critical: Notify UI to navigate to home screen
    }
  }

  Future<void> importFromPrivateKey(
    String pk, {
    KanariCurve curve = KanariCurve.ed25519,
    String? password,
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

      await addWallet(walletData, password);
      await refreshBalance();
    } catch (e) {
      _error = "Import PK failed: $e";
      debugPrint(_error);
    } finally {
      _setLoading(false);
    }
  }

  Future<void> importFromMnemonic(
    String mnemonic, {
    KanariCurve curve = KanariCurve.ed25519,
    String? password,
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

      await addWallet(walletData, password);
      await refreshBalance();
    } catch (e) {
      _error = "Import Mnemonic failed: $e";
      debugPrint(_error);
    } finally {
      _setLoading(false);
    }
  }

  void logout() {
    _wallet = null;
    _balance = 0;
    _error = null;
    notifyListeners();
  }

  Future<void> deleteAllWallets() async {
    debugPrint('🗑️ Deleting all wallets...');
    await WalletStorage.deleteWallet();
    _wallets = [];
    _wallet = null;
    _balance = 0;
    _error = null;
    debugPrint('✅ All wallets deleted');
    debugPrint('  - _wallets: ${_wallets.length}');
    debugPrint('  - _wallet: ${_wallet}');
    debugPrint('  - hasWallet: ${hasWallet}');
    notifyListeners();
    debugPrint('🔔 Notified listeners');
  }

  Future<void> refreshBalance() async {
    if (_client != null && _wallet != null) {
      try {
        _balance = await _client!.getBalance(_wallet!.address);
        _error = null;
        notifyListeners();
      } catch (e) {
        _error = "Refresh balance failed: $e";
        debugPrint(_error);
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
}
