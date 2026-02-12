import 'package:flutter/material.dart';
import 'package:kanari_kit/kanari_kit.dart';
import 'package:provider/provider.dart';
import 'package:flutter_spinkit/flutter_spinkit.dart';
import 'package:kanari_crypto/kanari_crypto.dart';

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
      providers: [
        ChangeNotifierProvider(create: (_) => WalletState()),
      ],
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

  KanariClient? get client => _client;
  KanariWallet? get wallet => _wallet;
  int get balance => _balance;
  bool get isLoading => _isLoading;
  String? get error => _error;

  Future<void> initialize() async {
    _client = KanariClient.fromEnvironment(KanariEnvironment.local);
    // ไม่สร้างอัตโนมัติเพื่อให้ผู้ใช้กดสร้างเอง
    notifyListeners();
  }

  Future<void> createNewWallet() async {
    _setLoading(true);
    _error = null;
    try {
      debugPrint("Starting wallet generation...");
      _wallet = await KanariWallet.generate();
      debugPrint("Wallet generated: ${_wallet?.address}");
      await refreshBalance();
    } catch (e, stack) {
      _error = "Creation failed: $e";
      debugPrint(_error);
      debugPrint(stack.toString());
    } finally {
      _setLoading(false);
    }
  }

  Future<void> importFromPrivateKey(String pk) async {
    _setLoading(true);
    _error = null;
    try {
      // Clean prefix 'kanari' if present
      String cleanPk = pk.trim();
      if (cleanPk.startsWith('kanari')) {
        cleanPk = cleanPk.substring(6);
      }
      
      _wallet = await KanariWallet.fromPrivateKey(cleanPk);
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

  Future<void> importFromMnemonic(String mnemonic) async {
    _setLoading(true);
    _error = null;
    try {
      _wallet = await KanariWallet.fromMnemonic(mnemonic);
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
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.blueAccent, brightness: Brightness.dark),
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
        title: const Text('Kanari Wallet'),
        actions: [
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
        ],
      ),
      body: state.isLoading
          ? const Center(child: SpinKitFadingCircle(color: Colors.blueAccent))
          : SingleChildScrollView(
              padding: const EdgeInsets.all(16.0),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  if (state.error != null)
                    Container(
                      padding: const EdgeInsets.all(12),
                      margin: const EdgeInsets.only(bottom: 16),
                      decoration: BoxDecoration(
                        color: Colors.red.withOpacity(0.1),
                        borderRadius: BorderRadius.circular(8),
                        border: Border.all(color: Colors.red.withOpacity(0.5)),
                      ),
                      child: Text(
                        state.error!,
                        style: const TextStyle(color: Colors.red),
                        textAlign: TextAlign.center,
                      ),
                    ),
                  if (state.wallet == null)
                    Center(
                      child: Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        children: [
                          const SizedBox(height: 100),
                          ElevatedButton(
                            onPressed: () => state.createNewWallet(),
                            child: const Text('Create New Wallet'),
                          ),
                          const SizedBox(height: 12),
                          TextButton.icon(
                            onPressed: () => _showImportDialog(context),
                            icon: const Icon(Icons.download),
                            label: const Text('Import Wallet (PK / Seed)'),
                          ),
                        ],
                      ),
                    )
                  else ...[
                    _buildBalanceCard(state),
                    const SizedBox(height: 20),
                    _buildWalletInfo(state),
                    const SizedBox(height: 20),
                    _buildSecurityInfo(state),
                    const SizedBox(height: 30),
                    ElevatedButton.icon(
                      onPressed: () => _showTransferDialog(context),
                      icon: const Icon(Icons.send),
                      label: const Text('Send KANARI'),
                      style: ElevatedButton.styleFrom(
                        padding: const EdgeInsets.symmetric(vertical: 16),
                      ),
                    ),
                  ],
                ],
              ),
            ),
    );
  }

  Widget _buildBalanceCard(WalletState state) {
    return Card(
      elevation: 4,
      child: Padding(
        padding: const EdgeInsets.all(24.0),
        child: Column(
          children: [
            const Text('Total Balance', style: TextStyle(fontSize: 16, color: Colors.grey)),
            const SizedBox(height: 8),
            Text(
              '${state.balance} KANARI',
              style: const TextStyle(fontSize: 32, fontWeight: FontWeight.bold),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildWalletInfo(WalletState state) {
    if (state.wallet == null) return const SizedBox.shrink();
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text('My Address:', style: TextStyle(fontWeight: FontWeight.bold)),
        SelectableText(
          state.wallet!.address,
          style: const TextStyle(fontFamily: 'monospace', color: Colors.blue),
        ),
        const SizedBox(height: 10),
        Text('Environment: ${KanariEnvironment.local.name.toUpperCase()}'),
      ],
    );
  }

  Widget _buildSecurityInfo(WalletState state) {
    if (state.wallet == null) return const SizedBox.shrink();
    return Card(
      color: Colors.red.withOpacity(0.1),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: BorderSide(color: Colors.red.withOpacity(0.3)),
      ),
      child: Padding(
        padding: const EdgeInsets.all(16.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Row(
              children: [
                Icon(Icons.warning_amber_rounded, color: Colors.red),
                SizedBox(width: 8),
                Text('Security Info (Private)', style: TextStyle(fontWeight: FontWeight.bold, color: Colors.red)),
              ],
            ),
            const SizedBox(height: 12),
            const Text('Seed Words (Mnemonic):', style: TextStyle(fontWeight: FontWeight.bold)),
            Container(
              padding: const EdgeInsets.all(8),
              decoration: BoxDecoration(color: Colors.black26, borderRadius: BorderRadius.circular(8)),
              child: SelectableText(
                state.wallet!.mnemonic ?? 'N/A',
                style: const TextStyle(fontFamily: 'monospace', color: Colors.orangeAccent),
              ),
            ),
            const SizedBox(height: 12),
            const Text('Private Key:', style: TextStyle(fontWeight: FontWeight.bold)),
            Container(
              padding: const EdgeInsets.all(8),
              decoration: BoxDecoration(color: Colors.black26, borderRadius: BorderRadius.circular(8)),
              child: SelectableText(
                state.wallet!.privateKey,
                style: const TextStyle(fontFamily: 'monospace', color: Colors.orangeAccent),
              ),
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

    showDialog(
      context: context,
      builder: (context) => DefaultTabController(
        length: 2,
        child: AlertDialog(
          title: const Text('Import Wallet'),
          content: SizedBox(
            width: double.maxFinite,
            height: 250,
            child: Column(
              children: [
                const TabBar(
                  tabs: [
                    Tab(text: 'Private Key'),
                    Tab(text: 'Mnemonic'),
                  ],
                ),
                Expanded(
                  child: TabBarView(
                    children: [
                      // Import via Private Key
                      Padding(
                        padding: const EdgeInsets.only(top: 16),
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
                                  state.importFromPrivateKey(pkController.text);
                                  Navigator.pop(context);
                                }
                              },
                              child: const Text('Import PK'),
                            ),
                          ],
                        ),
                      ),
                      // Import via Mnemonic
                      Padding(
                        padding: const EdgeInsets.only(top: 16),
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
                                  state.importFromMnemonic(mnemonicController.text);
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
            TextButton(onPressed: () => Navigator.pop(context), child: const Text('Cancel')),
          ],
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
          TextButton(onPressed: () => Navigator.pop(context), child: const Text('Cancel')),
          ElevatedButton(
            onPressed: () async {
              final recipient = recipientController.text;
              final amount = int.tryParse(amountController.text) ?? 0;
              if (recipient.isEmpty || amount <= 0) return;

              Navigator.pop(context);
              final result = await context.read<WalletState>().transfer(recipient, amount);
              if (mounted) {
                ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(result ?? "Unknown error")));
              }
            },
            child: const Text('Send'),
          ),
        ],
      ),
    );
  }
}
