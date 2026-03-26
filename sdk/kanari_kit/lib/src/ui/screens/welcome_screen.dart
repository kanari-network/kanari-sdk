import 'package:flutter/material.dart';
import 'package:kanari_kit/src/providers/wallet_provider.dart';
import 'package:provider/provider.dart';
import 'package:kanari_kit/kanari_kit.dart';

class WelcomeScreen extends StatelessWidget {
  const WelcomeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<WalletState>();
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Scaffold(
      body: Container(
        decoration: BoxDecoration(
          gradient: LinearGradient(
            begin: Alignment.topCenter,
            end: Alignment.bottomCenter,
            colors: [
              colorScheme.surface,
              colorScheme.primaryContainer.withOpacity(0.12),
            ],
          ),
        ),
        child: SafeArea(
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 24.0),
            child: Column(
              children: [
                const Spacer(),
                _buildLogo(theme),
                const SizedBox(height: 32),
                Text(
                  'Kanari Wallet',
                  style: theme.textTheme.displaySmall?.copyWith(
                    fontWeight: FontWeight.bold,
                    color: colorScheme.onSurface,
                    letterSpacing: -0.5,
                  ),
                ),
                const SizedBox(height: 12),
                Text(
                  'Secure, Quantum-Safe Digital Wallet',
                  textAlign: TextAlign.center,
                  style: theme.textTheme.bodyLarge?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
                const Spacer(),

                // M3 Action Area
                if (state.hasWallet) ...[
                  FilledButton.icon(
                    onPressed: () => _showUnlockDialog(context),
                    icon: const Icon(Icons.lock_open_rounded),
                    label: const Text('Unlock Saved Wallet'),
                    style: FilledButton.styleFrom(
                      minimumSize: const Size(double.infinity, 56),
                    ),
                  ),
                  const SizedBox(height: 12),
                  TextButton.icon(
                    onPressed: () => _showDeleteConfirm(context, state),
                    icon: const Icon(Icons.delete_outline_rounded, size: 20),
                    label: const Text('Clear All Wallet Data'),
                    style: TextButton.styleFrom(
                      foregroundColor: colorScheme.error,
                    ),
                  ),
                  _buildDivider(),
                ],

                FilledButton.tonalIcon(
                  onPressed: () => _showCreateDialog(context),
                  icon: const Icon(Icons.add_rounded),
                  label: const Text('Create New Wallet'),
                  style: FilledButton.styleFrom(
                    minimumSize: const Size(double.infinity, 56),
                  ),
                ),
                const SizedBox(height: 12),
                OutlinedButton.icon(
                  onPressed: () => _showImportDialog(context),
                  icon: const Icon(Icons.file_download_outlined),
                  label: const Text('Import Existing Wallet'),
                  style: OutlinedButton.styleFrom(
                    minimumSize: const Size(double.infinity, 56),
                  ),
                ),
                const SizedBox(height: 48),
              ],
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildLogo(ThemeData theme) {
    return Container(
      padding: const EdgeInsets.all(28),
      decoration: BoxDecoration(
        color: theme.colorScheme.primaryContainer,
        borderRadius: BorderRadius.circular(28), // M3 Extra Large Shape
      ),
      child: Icon(
        Icons.blur_on_rounded,
        size: 72,
        color: theme.colorScheme.onPrimaryContainer,
      ),
    );
  }

  Widget _buildDivider() {
    return const Padding(
      padding: EdgeInsets.symmetric(vertical: 24),
      child: Row(
        children: [
          Expanded(child: Divider()),
          Padding(
            padding: EdgeInsets.symmetric(horizontal: 16),
            child: Text(
              'OR',
              style: TextStyle(
                fontSize: 12,
                fontWeight: FontWeight.w500,
                color: Colors.grey,
              ),
            ),
          ),
          Expanded(child: Divider()),
        ],
      ),
    );
  }

  // --- Dialogs ปรับเป็นสไตล์ M3 ---

  void _showUnlockDialog(BuildContext context) {
    final controller = TextEditingController();
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        icon: const Icon(Icons.lock_outline_rounded),
        title: const Text('Unlock Wallet'),
        content: TextField(
          controller: controller,
          obscureText: true,
          decoration: InputDecoration(
            labelText: 'Password',
            filled: true,
            fillColor: Theme.of(
              context,
            ).colorScheme.surfaceVariant.withOpacity(0.3),
            border: OutlineInputBorder(borderRadius: BorderRadius.circular(12)),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () {
              context.read<WalletState>().unlockWallet(controller.text);
              Navigator.pop(context);
            },
            child: const Text('Unlock'),
          ),
        ],
      ),
    );
  }

  void _showCreateDialog(BuildContext context) {
    final controller = TextEditingController();
    KanariCurve selectedCurve = KanariCurve.ed25519;

    showDialog(
      context: context,
      builder: (context) => StatefulBuilder(
        builder: (context, setState) => AlertDialog(
          icon: const Icon(Icons.account_balance_wallet_outlined),
          title: const Text('New Wallet'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              SegmentedButton<KanariCurve>(
                segments: const [
                  ButtonSegment(
                    value: KanariCurve.ed25519,
                    label: Text('Ed25519'),
                  ),
                  ButtonSegment(
                    value: KanariCurve.k256,
                    label: Text('Secp256k1'),
                  ),
                ],
                selected: {selectedCurve},
                onSelectionChanged: (val) =>
                    setState(() => selectedCurve = val.first),
              ),
              const SizedBox(height: 20),
              TextField(
                controller: controller,
                obscureText: true,
                decoration: const InputDecoration(
                  labelText: 'Set Password',
                  border: OutlineInputBorder(),
                ),
              ),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(context),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () {
                if (controller.text.isNotEmpty) {
                  context.read<WalletState>().createNewWallet(
                    curve: selectedCurve,
                    password: controller.text,
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
    // ใช้สไตล์คล้ายๆ กัน แต่ปรับ TabBar ให้เข้ากับ M3
    showModalBottomSheet(
      // เปลี่ยนมาใช้ BottomSheet สำหรับการ Import เพื่อความสะดวก
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      showDragHandle: true,
      builder: (context) => _M3ImportSheet(),
    );
  }

  void _showDeleteConfirm(BuildContext context, WalletState state) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Delete All Data?'),
        content: const Text(
          'This action cannot be undone. Make sure you have backed up your seed phrases.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () {
              state.deleteAllWallets();
              Navigator.pop(context);
            },
            child: const Text(
              'Clear Everything',
              style: TextStyle(color: Colors.red),
            ),
          ),
        ],
      ),
    );
  }
}

// Widget แยกสำหรับ Import Sheet สไตล์ M3
class _M3ImportSheet extends StatefulWidget {
  @override
  State<_M3ImportSheet> createState() => _M3ImportSheetState();
}

class _M3ImportSheetState extends State<_M3ImportSheet>
    with SingleTickerProviderStateMixin {
  late TabController _tabController;
  final _passController = TextEditingController();
  final _dataController = TextEditingController();
  KanariCurve _curve = KanariCurve.ed25519;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 2, vsync: this);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: EdgeInsets.fromLTRB(
        24,
        8,
        24,
        MediaQuery.of(context).viewInsets.bottom + 24,
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text('Import Wallet', style: theme.textTheme.headlineSmall),
          const SizedBox(height: 16),
          TabBar(
            controller: _tabController,
            tabs: const [
              Tab(text: 'Private Key'),
              Tab(text: 'Mnemonic'),
            ],
          ),
          const SizedBox(height: 24),
          DropdownButtonFormField<KanariCurve>(
            value: _curve,
            decoration: const InputDecoration(
              labelText: 'Curve Type',
              border: OutlineInputBorder(),
            ),
            items: KanariCurve.values
                .map((c) => DropdownMenuItem(value: c, child: Text(c.name)))
                .toList(),
            onChanged: (v) => setState(() => _curve = v!),
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _passController,
            obscureText: true,
            decoration: const InputDecoration(
              labelText: 'Set Password',
              border: OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _dataController,
            maxLines: 3,
            decoration: InputDecoration(
              hintText: 'Enter your key or 12 words',
              border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(12),
              ),
            ),
          ),
          const SizedBox(height: 24),
          FilledButton(
            style: FilledButton.styleFrom(
              minimumSize: const Size(double.infinity, 56),
            ),
            onPressed: () {
              final state = context.read<WalletState>();
              if (_tabController.index == 0) {
                state.importFromPrivateKey(
                  _dataController.text,
                  curve: _curve,
                  password: _passController.text,
                );
              } else {
                state.importFromMnemonic(
                  _dataController.text,
                  curve: _curve,
                  password: _passController.text,
                );
              }
              Navigator.pop(context);
            },
            child: const Text('Import Now'),
          ),
        ],
      ),
    );
  }
}
