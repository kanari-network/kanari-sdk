import 'package:flutter/material.dart';
import 'package:kanari_kit/kanari_kit.dart';
import 'package:provider/provider.dart';
import 'package:flutter_spinkit/flutter_spinkit.dart';
import 'package:kanari_crypto/kanari_crypto.dart';
import 'src/ui/balance_card.dart';
import 'src/ui/network_selector.dart';
import 'src/ui/wallet_info_card.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  try {
    // ใช้ฟังก์ชัน init ที่รวมอยู่ใน kanari_crypto
    await initKanariCrypto();
    debugPrint("Kanari Crypto initialized");
  } catch (e) {
    debugPrint("Kanari Crypto init error: $e");
  }
  runApp(
    MultiProvider(
      providers: [ChangeNotifierProvider(create: (_) => WalletState())],
      child: const KanariApp(),
    ),
  );
}

class WalletState extends ChangeNotifier {
  KanariClient? _client;
  KanariWallet? _wallet;
  int _balance = 0;
  bool _isLoading = false;
  String? _error;
  bool _hasSavedWallet = false;
  KanariEnvironment _environment = KanariEnvironment.local;

  KanariClient? get client => _client;
  KanariWallet? get wallet => _wallet;
  int get balance => _balance;
  bool get isLoading => _isLoading;
  String? get error => _error;
  bool get hasSavedWallet => _hasSavedWallet;
  KanariEnvironment get environment => _environment;

  Future<void> initialize() async {
    _updateClient();
    _hasSavedWallet = await WalletStorage.hasWallet();
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

  Future<void> createNewWallet({
    KanariCurve curve = KanariCurve.ed25519,
    required String password,
    bool rememberMe = true,
  }) async {
    _setLoading(true);
    _error = null;
    try {
      debugPrint("Starting wallet generation with curve: ${curve.name}...");
      _wallet = await KanariWallet.generate(curve: curve);
      debugPrint("Wallet generated: ${_wallet?.address}");

      if (rememberMe && _wallet != null) {
        await WalletStorage.saveWallet(
          mnemonic: _wallet!.mnemonic,
          privateKey: _wallet!.privateKey,
          curve: curve,
          password: password,
        );
        _hasSavedWallet = true;
      }

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
      final data = await WalletStorage.loadWallet(password);
      if (data == null) {
        _error = "Invalid password or no saved wallet";
        return;
      }

      final curve = KanariCurve.values.firstWhere(
        (c) => c.name == data['curve'],
        orElse: () => KanariCurve.ed25519,
      );

      if (data['mnemonic'] != null &&
          data['mnemonic'].toString().isNotEmpty &&
          !curve.isPostQuantum) {
        _wallet = await KanariWallet.fromMnemonic(
          data['mnemonic'],
          curve: curve,
        );
      } else {
        _wallet = await KanariWallet.fromPrivateKey(
          data['privateKey'],
          curve: curve,
        );
      }

      await refreshBalance();
    } catch (e) {
      _error = "Unlock failed: $e";
    } finally {
      _setLoading(false);
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
      // Let the Rust side handle all prefix variations (kanari, kanapqc, kanahybrid)
      _wallet = await KanariWallet.fromPrivateKey(cleanPk, curve: curve);

      if (password != null && _wallet != null) {
        await WalletStorage.saveWallet(
          mnemonic: '', // No mnemonic for PK import
          privateKey: _wallet!.privateKey,
          curve: curve,
          password: password,
        );
        _hasSavedWallet = true;
      }

      await refreshBalance();
    } catch (e) {
      _error = "Import PK failed: $e";
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

  Future<void> deleteSavedWallet() async {
    await WalletStorage.deleteWallet();
    _hasSavedWallet = false;
    logout();
  }

  Future<void> importFromMnemonic(
    String mnemonic, {
    KanariCurve curve = KanariCurve.ed25519,
    String? password,
  }) async {
    _setLoading(true);
    _error = null;
    try {
      _wallet = await KanariWallet.fromMnemonic(mnemonic, curve: curve);

      if (password != null && _wallet != null) {
        await WalletStorage.saveWallet(
          mnemonic: mnemonic,
          privateKey: _wallet!.privateKey,
          curve: curve,
          password: password,
        );
        _hasSavedWallet = true;
      }

      await refreshBalance();
    } catch (e) {
      _error = "Import Mnemonic failed: $e";
      debugPrint(_error);
    } finally {
      _setLoading(false);
    }
  }

  Future<void> refreshBalance() async {
    if (_client != null && _wallet != null) {
      try {
        _balance = await _client!.getBalance(_wallet!.address);
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
}

class KanariApp extends StatelessWidget {
  const KanariApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Kanari Wallet',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: Colors.blueAccent,
          brightness: Brightness.dark,
        ),
        useMaterial3: true,
      ),
      home: const WalletHomePage(),
    );
  }
}

class WalletHomePage extends StatefulWidget {
  const WalletHomePage({super.key});

  @override
  State<WalletHomePage> createState() => _WalletHomePageState();
}

class _WalletHomePageState extends State<WalletHomePage> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<WalletState>().initialize();
    });
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<WalletState>();

    return Scaffold(
      appBar: AppBar(
        title: const Text(
          'Kanari Wallet',
          style: TextStyle(fontWeight: FontWeight.bold),
        ),
        centerTitle: false,
        actions: [
          const NetworkSelector(),
          const SizedBox(width: 8),
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: state.isLoading ? null : () => state.refreshBalance(),
          ),
          if (state.wallet != null)
            IconButton(
              icon: const Icon(Icons.logout),
              onPressed: () => state.logout(),
              tooltip: 'Logout',
            ),
          const SizedBox(width: 8),
        ],
      ),
      body: state.isLoading
          ? const Center(child: SpinKitFadingCircle(color: Colors.blueAccent))
          : SingleChildScrollView(
              padding: const EdgeInsets.all(20.0),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  if (state.error != null) _buildErrorBanner(state.error!),
                  if (state.wallet == null)
                    _buildWelcomeScreen(context, state)
                  else ...[
                    const BalanceCard(),
                    const SizedBox(height: 24),
                    const WalletInfoCard(),
                    const SizedBox(height: 24),
                    _buildSecurityInfo(state),
                    const SizedBox(height: 32),
                    ElevatedButton.icon(
                      onPressed: () => _showTransferDialog(context),
                      icon: const Icon(Icons.send_rounded),
                      label: const Text(
                        'Send KANARI',
                        style: TextStyle(
                          fontSize: 16,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                      style: ElevatedButton.styleFrom(
                        padding: const EdgeInsets.symmetric(vertical: 18),
                        backgroundColor: Colors.blueAccent,
                        foregroundColor: Colors.white,
                        elevation: 4,
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(16),
                        ),
                      ),
                    ),
                    const SizedBox(height: 12),
                    Row(
                      children: [
                        Expanded(
                          child: OutlinedButton.icon(
                            onPressed: () => _showExecuteDialog(context),
                            icon: const Icon(Icons.code_rounded),
                            label: const Text('Execute'),
                            style: OutlinedButton.styleFrom(
                              padding: const EdgeInsets.symmetric(vertical: 16),
                              shape: RoundedRectangleBorder(
                                borderRadius: BorderRadius.circular(16),
                              ),
                            ),
                          ),
                        ),
                        const SizedBox(width: 12),
                        Expanded(
                          child: OutlinedButton.icon(
                            onPressed: () => _showBurnDialog(context),
                            icon: const Icon(
                              Icons.local_fire_department_rounded,
                            ),
                            label: const Text('Burn'),
                            style: OutlinedButton.styleFrom(
                              padding: const EdgeInsets.symmetric(vertical: 16),
                              foregroundColor: Colors.orangeAccent,
                              side: const BorderSide(
                                color: Colors.orangeAccent,
                              ),
                              shape: RoundedRectangleBorder(
                                borderRadius: BorderRadius.circular(16),
                              ),
                            ),
                          ),
                        ),
                      ],
                    ),
                  ],
                ],
              ),
            ),
    );
  }

  void _showExecuteDialog(BuildContext context) {
    final packageController = TextEditingController();
    final moduleController = TextEditingController();
    final functionController = TextEditingController();

    showDialog(
      context: context,
      builder: (innerContext) => AlertDialog(
        title: const Text('Execute Move Function'),
        content: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(
                controller: packageController,
                decoration: const InputDecoration(
                  labelText: 'Package Address',
                  hintText: '0x...',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: moduleController,
                decoration: const InputDecoration(
                  labelText: 'Module Name',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: functionController,
                decoration: const InputDecoration(
                  labelText: 'Function Name',
                  border: OutlineInputBorder(),
                ),
              ),
              const SizedBox(height: 8),
              const Text(
                'Note: Arguments and Type Args are currently limited to defaults in this UI.',
                style: TextStyle(fontSize: 11, color: Colors.grey),
              ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(innerContext),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () async {
              if (packageController.text.isEmpty ||
                  moduleController.text.isEmpty ||
                  functionController.text.isEmpty)
                return;

              final pkg = packageController.text;
              final mod = moduleController.text;
              final fun = functionController.text;

              Navigator.pop(innerContext);
              final result = await context.read<WalletState>().executeFunction(
                packageAddress: pkg,
                module: mod,
                function: fun,
              );
              if (context.mounted) {
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(content: Text(result ?? "Unknown error")),
                );
              }
            },
            child: const Text('Execute'),
          ),
        ],
      ),
    );
  }

  void _showBurnDialog(BuildContext context) {
    final amountController = TextEditingController();

    showDialog(
      context: context,
      builder: (innerContext) => AlertDialog(
        title: const Text('Burn KANARI'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Text(
              'Warning: This action will permanently destroy tokens. Only admins can perform this.',
              style: TextStyle(color: Colors.orangeAccent, fontSize: 13),
            ),
            const SizedBox(height: 16),
            TextField(
              controller: amountController,
              decoration: const InputDecoration(
                labelText: 'Amount to Burn',
                border: OutlineInputBorder(),
              ),
              keyboardType: TextInputType.number,
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(innerContext),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () async {
              final amountStr = amountController.text;
              final amountDouble = double.tryParse(amountStr) ?? 0.0;
              final amountMist = (amountDouble * 1000000000).round();

              if (amountMist <= 0) return;

              Navigator.pop(innerContext);
              final result = await context.read<WalletState>().burn(amountMist);
              if (context.mounted) {
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(content: Text(result ?? "Unknown error")),
                );
              }
            },
            style: ElevatedButton.styleFrom(
              backgroundColor: Colors.orangeAccent,
            ),
            child: const Text('Burn'),
          ),
        ],
      ),
    );
  }

  Widget _buildErrorBanner(String error) {
    return Container(
      padding: const EdgeInsets.all(12),
      margin: const EdgeInsets.only(bottom: 20),
      decoration: BoxDecoration(
        color: Colors.red.withOpacity(0.1),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: Colors.red.withOpacity(0.3)),
      ),
      child: Row(
        children: [
          const Icon(Icons.error_outline, color: Colors.redAccent, size: 20),
          const SizedBox(width: 12),
          Expanded(
            child: Text(
              error,
              style: const TextStyle(color: Colors.redAccent, fontSize: 13),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildWelcomeScreen(BuildContext context, WalletState state) {
    return Column(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        const SizedBox(height: 60),
        Icon(
          Icons.blur_on_rounded,
          size: 100,
          color: Colors.blueAccent.withOpacity(0.5),
        ),
        const SizedBox(height: 24),
        const Text(
          'Welcome to Kanari',
          style: TextStyle(fontSize: 28, fontWeight: FontWeight.bold),
        ),
        const SizedBox(height: 8),
        const Text(
          'Secure, Quantum-Safe Digital Wallet',
          style: TextStyle(color: Colors.grey),
        ),
        const SizedBox(height: 60),
        if (state.hasSavedWallet) ...[
          SizedBox(
            width: double.infinity,
            child: ElevatedButton.icon(
              onPressed: () => _showUnlockDialog(context),
              icon: const Icon(Icons.lock_open_rounded),
              label: const Text('Unlock Saved Wallet'),
              style: ElevatedButton.styleFrom(
                padding: const EdgeInsets.symmetric(vertical: 16),
                backgroundColor: Colors.greenAccent.withOpacity(0.1),
                foregroundColor: Colors.greenAccent,
                side: BorderSide(color: Colors.greenAccent.withOpacity(0.5)),
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(16),
                ),
              ),
            ),
          ),
          const SizedBox(height: 12),
          TextButton.icon(
            onPressed: () => state.deleteSavedWallet(),
            icon: const Icon(Icons.delete_outline, size: 18),
            label: const Text('Clear Saved Data'),
            style: TextButton.styleFrom(
              foregroundColor: Colors.redAccent.withOpacity(0.7),
            ),
          ),
          const Padding(
            padding: EdgeInsets.symmetric(vertical: 30),
            child: Row(
              children: [
                Expanded(child: Divider()),
                Padding(
                  padding: EdgeInsets.symmetric(horizontal: 16),
                  child: Text(
                    'OR',
                    style: TextStyle(color: Colors.grey, fontSize: 12),
                  ),
                ),
                Expanded(child: Divider()),
              ],
            ),
          ),
        ],
        SizedBox(
          width: double.infinity,
          child: ElevatedButton(
            onPressed: () => _showCreateDialog(context),
            style: ElevatedButton.styleFrom(
              padding: const EdgeInsets.symmetric(vertical: 16),
              backgroundColor: Colors.blueAccent,
              foregroundColor: Colors.white,
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(16),
              ),
            ),
            child: const Text(
              'Create New Wallet',
              style: TextStyle(fontWeight: FontWeight.bold),
            ),
          ),
        ),
        const SizedBox(height: 16),
        SizedBox(
          width: double.infinity,
          child: OutlinedButton.icon(
            onPressed: () => _showImportDialog(context),
            icon: const Icon(Icons.file_download_outlined),
            label: const Text('Import Existing Wallet'),
            style: OutlinedButton.styleFrom(
              padding: const EdgeInsets.symmetric(vertical: 16),
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(16),
              ),
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildSecurityInfo(WalletState state) {
    if (state.wallet == null) return const SizedBox.shrink();
    return Card(
      elevation: 0,
      color: Colors.orangeAccent.withOpacity(0.05),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(16),
        side: BorderSide(color: Colors.orangeAccent.withOpacity(0.2)),
      ),
      child: Padding(
        padding: const EdgeInsets.all(20.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Row(
              children: [
                Icon(
                  Icons.security_rounded,
                  color: Colors.orangeAccent,
                  size: 20,
                ),
                SizedBox(width: 10),
                Text(
                  'Security Info',
                  style: TextStyle(
                    fontWeight: FontWeight.bold,
                    color: Colors.orangeAccent,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 20),
            _buildSecurityField(
              'Mnemonic Seed',
              state.wallet!.mnemonic ?? 'Not available for this curve',
            ),
            const SizedBox(height: 16),
            _buildSecurityField('Private Key', state.wallet!.privateKey),
          ],
        ),
      ),
    );
  }

  Widget _buildSecurityField(String label, String value) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          label,
          style: const TextStyle(
            fontSize: 12,
            color: Colors.grey,
            fontWeight: FontWeight.bold,
          ),
        ),
        const SizedBox(height: 6),
        Container(
          width: double.infinity,
          padding: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: Colors.black26,
            borderRadius: BorderRadius.circular(10),
          ),
          child: SelectableText(
            value,
            style: const TextStyle(
              fontFamily: 'monospace',
              fontSize: 12,
              color: Colors.orangeAccent,
            ),
          ),
        ),
      ],
    );
  }

  void _showUnlockDialog(BuildContext context) {
    final passwordController = TextEditingController();
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Unlock Wallet'),
        content: TextField(
          controller: passwordController,
          decoration: const InputDecoration(
            labelText: 'Password',
            border: OutlineInputBorder(),
          ),
          obscureText: true,
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () {
              context.read<WalletState>().unlockWallet(passwordController.text);
              Navigator.pop(context);
            },
            child: const Text('Unlock'),
          ),
        ],
      ),
    );
  }

  void _showCreateDialog(BuildContext context) {
    final passwordController = TextEditingController();
    KanariCurve selectedCurve = KanariCurve.ed25519;

    showDialog(
      context: context,
      builder: (context) => StatefulBuilder(
        builder: (context, setState) => AlertDialog(
          title: const Text('Create New Wallet'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              DropdownButtonFormField<KanariCurve>(
                value: selectedCurve,
                decoration: const InputDecoration(
                  labelText: 'Curve Type',
                  border: OutlineInputBorder(),
                ),
                items: KanariCurve.values.map((curve) {
                  return DropdownMenuItem(
                    value: curve,
                    child: Text(curve.name),
                  );
                }).toList(),
                onChanged: (val) => setState(() => selectedCurve = val!),
              ),
              const SizedBox(height: 16),
              TextField(
                controller: passwordController,
                decoration: const InputDecoration(
                  labelText: 'Set Password',
                  border: OutlineInputBorder(),
                ),
                obscureText: true,
              ),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('Cancel'),
            ),
            ElevatedButton(
              onPressed: () {
                if (passwordController.text.isNotEmpty) {
                  context.read<WalletState>().createNewWallet(
                    curve: selectedCurve,
                    password: passwordController.text,
                  );
                  Navigator.pop(context);
                }
              },
              child: const Text('Generate'),
            ),
          ],
        ),
      ),
    );
  }

  void _showImportDialog(BuildContext context) {
    final state = context.read<WalletState>();
    final pkController = TextEditingController();
    final mnemonicController = TextEditingController();
    final passwordController = TextEditingController();
    KanariCurve selectedCurve = KanariCurve.ed25519;

    showDialog(
      context: context,
      builder: (context) => DefaultTabController(
        length: 2,
        child: StatefulBuilder(
          builder: (context, setState) => AlertDialog(
            title: const Text('Import Wallet'),
            content: SizedBox(
              width: double.maxFinite,
              height: 400,
              child: Column(
                children: [
                  const TabBar(
                    tabs: [
                      Tab(text: 'Private Key'),
                      Tab(text: 'Mnemonic'),
                    ],
                  ),
                  const SizedBox(height: 16),
                  DropdownButtonFormField<KanariCurve>(
                    value: selectedCurve,
                    decoration: const InputDecoration(
                      labelText: 'Curve Type',
                      border: OutlineInputBorder(),
                    ),
                    items: KanariCurve.values.map((curve) {
                      return DropdownMenuItem(
                        value: curve,
                        child: Text(curve.name),
                      );
                    }).toList(),
                    onChanged: (val) => setState(() => selectedCurve = val!),
                  ),
                  const SizedBox(height: 16),
                  TextField(
                    controller: passwordController,
                    decoration: const InputDecoration(
                      labelText: 'Set Password (to save)',
                      border: OutlineInputBorder(),
                    ),
                    obscureText: true,
                  ),
                  const Divider(height: 32),
                  Expanded(
                    child: TabBarView(
                      children: [
                        // Import via Private Key
                        SingleChildScrollView(
                          child: Column(
                            children: [
                              TextField(
                                controller: pkController,
                                decoration: const InputDecoration(
                                  labelText: 'Private Key',
                                  hintText: 'Enter your private key',
                                  border: OutlineInputBorder(),
                                ),
                              ),
                              const SizedBox(height: 16),
                              ElevatedButton(
                                onPressed: () {
                                  if (pkController.text.isNotEmpty) {
                                    state.importFromPrivateKey(
                                      pkController.text,
                                      curve: selectedCurve,
                                      password:
                                          passwordController.text.isNotEmpty
                                          ? passwordController.text
                                          : null,
                                    );
                                    Navigator.pop(context);
                                  }
                                },
                                child: const Text('Import PK'),
                              ),
                            ],
                          ),
                        ),
                        // Import via Mnemonic
                        SingleChildScrollView(
                          child: Column(
                            children: [
                              TextField(
                                controller: mnemonicController,
                                maxLines: 3,
                                decoration: const InputDecoration(
                                  labelText: 'Mnemonic (12 words)',
                                  hintText: 'Enter your seed words',
                                  border: OutlineInputBorder(),
                                ),
                              ),
                              const SizedBox(height: 16),
                              ElevatedButton(
                                onPressed: () {
                                  if (mnemonicController.text.isNotEmpty) {
                                    state.importFromMnemonic(
                                      mnemonicController.text,
                                      curve: selectedCurve,
                                      password:
                                          passwordController.text.isNotEmpty
                                          ? passwordController.text
                                          : null,
                                    );
                                    Navigator.pop(context);
                                  }
                                },
                                child: const Text('Import Seed'),
                              ),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
            actions: [
              TextButton(
                onPressed: () => Navigator.pop(context),
                child: const Text('Cancel'),
              ),
            ],
          ),
        ),
      ),
    );
  }

  void _showTransferDialog(BuildContext context) {
    final recipientController = TextEditingController();
    final amountController = TextEditingController();

    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Transfer KANARI'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: recipientController,
              decoration: const InputDecoration(labelText: 'Recipient Address'),
            ),
            TextField(
              controller: amountController,
              decoration: const InputDecoration(labelText: 'Amount'),
              keyboardType: TextInputType.number,
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () async {
              final recipient = recipientController.text;
              final amountStr = amountController.text;
              final amountDouble = double.tryParse(amountStr) ?? 0.0;
              final amountMist = (amountDouble * 1000000000).round();

              if (recipient.isEmpty || amountMist <= 0) return;

              Navigator.pop(context);
              final result = await context.read<WalletState>().transfer(
                recipient,
                amountMist,
              );
              if (context.mounted) {
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(content: Text(result ?? "Unknown error")),
                );
              }
            },
            child: const Text('Send'),
          ),
        ],
      ),
    );
  }
}
