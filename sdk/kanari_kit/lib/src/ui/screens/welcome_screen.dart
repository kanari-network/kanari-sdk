import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
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
        borderRadius: BorderRadius.circular(28),
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

  // --- Dialogs (อัปเดตเป็น Bottom Sheet ทั้งหมด) ---

  void _showUnlockDialog(BuildContext context) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      showDragHandle: true,
      backgroundColor: Theme.of(context).colorScheme.surface,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(32)),
      ),
      builder: (context) => const _UnlockWalletSheet(),
    );
  }

  void _showCreateDialog(BuildContext context) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      showDragHandle: true,
      backgroundColor: Theme.of(context).colorScheme.surface,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(32)),
      ),
      builder: (context) => const _CreateWalletSheet(),
    );
  }

  void _showImportDialog(BuildContext context) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      showDragHandle: true,
      backgroundColor: Theme.of(context).colorScheme.surface,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(32)),
      ),
      builder: (context) => const _M3ImportSheet(),
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

// ============================================================================
// UI: Unlock Wallet Sheet
// ============================================================================
class _UnlockWalletSheet extends StatefulWidget {
  const _UnlockWalletSheet();

  @override
  State<_UnlockWalletSheet> createState() => _UnlockWalletSheetState();
}

class _UnlockWalletSheetState extends State<_UnlockWalletSheet> {
  final _pinController = TextEditingController();

  @override
  void dispose() {
    _pinController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Padding(
      padding: EdgeInsets.fromLTRB(
        24,
        8,
        24,
        MediaQuery.of(context).viewInsets.bottom + 24,
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'Unlock Wallet',
            style: theme.textTheme.headlineSmall?.copyWith(
              fontWeight: FontWeight.bold,
            ),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 24),
          TextField(
            controller: _pinController,
            obscureText: true,
            keyboardType: TextInputType.number,
            inputFormatters: [FilteringTextInputFormatter.digitsOnly],
            maxLength: 6,
            textAlign: TextAlign.center,
            style: const TextStyle(
              letterSpacing: 8,
              fontSize: 24,
              fontWeight: FontWeight.bold,
            ),
            decoration: InputDecoration(
              labelText: '6-Digit PIN',
              counterText: '',
              filled: true,
              fillColor: colorScheme.surfaceVariant.withOpacity(0.3),
              border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(16),
              ),
            ),
          ),
          const SizedBox(height: 32),
          FilledButton(
            style: FilledButton.styleFrom(
              minimumSize: const Size(double.infinity, 56),
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(20),
              ),
            ),
            onPressed: () {
              if (_pinController.text.length == 6) {
                context.read<WalletState>().unlockWallet(_pinController.text);
                Navigator.pop(context);
              } else {
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(
                    content: const Text('Please enter exactly 6 digits'),
                    backgroundColor: colorScheme.error,
                  ),
                );
              }
            },
            child: const Text(
              'Unlock',
              style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
            ),
          ),
        ],
      ),
    );
  }
}

// ============================================================================
// UI: Create Wallet Sheet
// ============================================================================
class _CreateWalletSheet extends StatefulWidget {
  const _CreateWalletSheet();

  @override
  State<_CreateWalletSheet> createState() => _CreateWalletSheetState();
}

class _CreateWalletSheetState extends State<_CreateWalletSheet> {
  final _pinController = TextEditingController();
  KanariCurve _selectedCurve = KanariCurve.ed25519;

  @override
  void dispose() {
    _pinController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Padding(
      padding: EdgeInsets.fromLTRB(
        24,
        8,
        24,
        MediaQuery.of(context).viewInsets.bottom + 24,
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'New Wallet',
            style: theme.textTheme.headlineSmall?.copyWith(
              fontWeight: FontWeight.bold,
            ),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 8),
          const Text(
            'Select the cryptographic curve and set a 6-digit PIN.',
            style: TextStyle(fontSize: 13, color: Colors.grey),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 24),
          DropdownButtonFormField<KanariCurve>(
            value: _selectedCurve,
            isExpanded: true,
            decoration: InputDecoration(
              labelText: 'Curve Type',
              border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(16),
              ),
            ),
            items: KanariCurve.values.map((curve) {
              return DropdownMenuItem(
                value: curve,
                child: Text(curve.name, overflow: TextOverflow.ellipsis),
              );
            }).toList(),
            onChanged: (val) => setState(() => _selectedCurve = val!),
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _pinController,
            obscureText: true,
            keyboardType: TextInputType.number,
            inputFormatters: [FilteringTextInputFormatter.digitsOnly],
            maxLength: 6,
            textAlign: TextAlign.center,
            style: const TextStyle(
              letterSpacing: 8,
              fontSize: 24,
              fontWeight: FontWeight.bold,
            ),
            decoration: InputDecoration(
              labelText: 'Set 6-Digit PIN',
              counterText: '',
              border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(16),
              ),
            ),
          ),
          const SizedBox(height: 32),
          FilledButton(
            style: FilledButton.styleFrom(
              minimumSize: const Size(double.infinity, 56),
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(20),
              ),
            ),
            onPressed: () {
              if (_pinController.text.length == 6) {
                context.read<WalletState>().createNewWallet(
                  curve: _selectedCurve,
                  pin: _pinController.text,
                );
                Navigator.pop(context);
              } else {
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(
                    content: const Text('PIN must be exactly 6 digits'),
                    backgroundColor: colorScheme.error,
                  ),
                );
              }
            },
            child: const Text(
              'Generate',
              style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
            ),
          ),
        ],
      ),
    );
  }
}

// ============================================================================
// UI: Import Wallet Sheet
// ============================================================================
class _M3ImportSheet extends StatefulWidget {
  const _M3ImportSheet();

  @override
  State<_M3ImportSheet> createState() => _M3ImportSheetState();
}

class _M3ImportSheetState extends State<_M3ImportSheet>
    with SingleTickerProviderStateMixin {
  late TabController _tabController;
  final _pinController = TextEditingController();
  final _dataController = TextEditingController();
  KanariCurve _curve = KanariCurve.ed25519;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 2, vsync: this);
  }

  @override
  void dispose() {
    _tabController.dispose();
    _pinController.dispose();
    _dataController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Padding(
      padding: EdgeInsets.fromLTRB(
        24,
        8,
        24,
        MediaQuery.of(context).viewInsets.bottom + 24,
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            'Import Wallet',
            style: theme.textTheme.headlineSmall?.copyWith(
              fontWeight: FontWeight.bold,
            ),
            textAlign: TextAlign.center,
          ),
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
            decoration: InputDecoration(
              labelText: 'Curve Type',
              border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(16),
              ),
            ),
            items: KanariCurve.values
                .map((c) => DropdownMenuItem(value: c, child: Text(c.name)))
                .toList(),
            onChanged: (v) => setState(() => _curve = v!),
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _dataController,
            maxLines: 3,
            decoration: InputDecoration(
              hintText: 'Enter your key or 12 words',
              border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(16),
              ),
            ),
          ),
          const SizedBox(height: 16),
          TextField(
            controller: _pinController,
            obscureText: true,
            keyboardType: TextInputType.number,
            inputFormatters: [FilteringTextInputFormatter.digitsOnly],
            maxLength: 6,
            textAlign: TextAlign.center,
            style: const TextStyle(
              letterSpacing: 8,
              fontSize: 24,
              fontWeight: FontWeight.bold,
            ),
            decoration: InputDecoration(
              labelText: 'Set 6-Digit PIN',
              counterText: '',
              border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(16),
              ),
            ),
          ),
          const SizedBox(height: 32),
          FilledButton(
            style: FilledButton.styleFrom(
              minimumSize: const Size(double.infinity, 56),
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(20),
              ),
            ),
            onPressed: () {
              if (_pinController.text.length != 6) {
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(
                    content: const Text('PIN must be exactly 6 digits'),
                    backgroundColor: colorScheme.error,
                  ),
                );
                return;
              }

              final state = context.read<WalletState>();
              if (_tabController.index == 0) {
                state.importFromPrivateKey(
                  _dataController.text,
                  curve: _curve,
                  pin: _pinController.text,
                );
              } else {
                state.importFromMnemonic(
                  _dataController.text,
                  curve: _curve,
                  pin: _pinController.text,
                );
              }
              Navigator.pop(context);
            },
            child: const Text(
              'Import Now',
              style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
            ),
          ),
        ],
      ),
    );
  }
}
