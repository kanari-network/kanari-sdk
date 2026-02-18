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

    return Scaffold(
      body: Container(
        decoration: BoxDecoration(
          gradient: LinearGradient(
            begin: Alignment.topLeft,
            end: Alignment.bottomRight,
            colors: [
              theme.colorScheme.surface,
              theme.colorScheme.primaryContainer.withOpacity(0.1),
            ],
          ),
        ),
        child: SafeArea(
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 24.0),
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                const Spacer(),
                _buildLogo(theme),
                const SizedBox(height: 32),
                Text(
                  'Kanari Wallet',
                  style: theme.textTheme.headlineLarge?.copyWith(
                    fontWeight: FontWeight.bold,
                    color: theme.colorScheme.primary,
                  ),
                ),
                const SizedBox(height: 8),
                Text(
                  'Secure, Quantum-Safe Digital Wallet',
                  textAlign: TextAlign.center,
                  style: theme.textTheme.bodyMedium?.copyWith(
                    color: theme.colorScheme.onSurface.withOpacity(0.6),
                  ),
                ),
                const Spacer(),
                if (state.hasSavedWallet) ...[
                  _buildActionButton(
                    context: context,
                    label: 'Unlock Saved Wallet',
                    icon: Icons.lock_open_rounded,
                    onPressed: () => _showUnlockDialog(context),
                    isPrimary: true,
                  ),
                  const SizedBox(height: 12),
                  TextButton.icon(
                    onPressed: () => state.deleteSavedWallet(),
                    icon: const Icon(Icons.delete_outline, size: 18),
                    label: const Text('Clear Saved Data'),
                    style: TextButton.styleFrom(
                      foregroundColor: theme.colorScheme.error.withOpacity(0.7),
                    ),
                  ),
                  const Padding(
                    padding: EdgeInsets.symmetric(vertical: 24),
                    child: Row(
                      children: [
                        Expanded(child: Divider()),
                        Padding(
                          padding: EdgeInsets.symmetric(horizontal: 16),
                          child: Text(
                            'OR',
                            style: TextStyle(fontSize: 12, color: Colors.grey),
                          ),
                        ),
                        Expanded(child: Divider()),
                      ],
                    ),
                  ),
                ],
                _buildActionButton(
                  context: context,
                  label: 'Create New Wallet',
                  icon: Icons.add_rounded,
                  onPressed: () => _showCreateDialog(context),
                  isPrimary: !state.hasSavedWallet,
                ),
                const SizedBox(height: 16),
                _buildActionButton(
                  context: context,
                  label: 'Import Existing Wallet',
                  icon: Icons.file_download_outlined,
                  onPressed: () => _showImportDialog(context),
                  isPrimary: false,
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
      padding: const EdgeInsets.all(24),
      decoration: BoxDecoration(
        color: theme.colorScheme.primary.withOpacity(0.1),
        shape: BoxShape.circle,
        border: Border.all(color: theme.colorScheme.primary.withOpacity(0.2)),
      ),
      child: Icon(
        Icons.blur_on_rounded,
        size: 80,
        color: theme.colorScheme.primary,
      ),
    );
  }

  Widget _buildActionButton({
    required BuildContext context,
    required String label,
    required IconData icon,
    required VoidCallback onPressed,
    required bool isPrimary,
  }) {
    final theme = Theme.of(context);
    return SizedBox(
      width: double.infinity,
      child: isPrimary
          ? ElevatedButton.icon(
              onPressed: onPressed,
              icon: Icon(icon),
              label: Text(label),
              style: ElevatedButton.styleFrom(
                padding: const EdgeInsets.symmetric(vertical: 16),
                backgroundColor: theme.colorScheme.primary,
                foregroundColor: theme.colorScheme.onPrimary,
                elevation: 0,
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(16),
                ),
              ),
            )
          : OutlinedButton.icon(
              onPressed: onPressed,
              icon: Icon(icon),
              label: Text(label),
              style: OutlinedButton.styleFrom(
                padding: const EdgeInsets.symmetric(vertical: 16),
                side: BorderSide(
                  color: theme.colorScheme.outline.withOpacity(0.5),
                ),
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(16),
                ),
              ),
            ),
    );
  }

  void _showUnlockDialog(BuildContext context) {
    final passwordController = TextEditingController();
    showDialog(
      context: context,
      builder: (dialogContext) => AlertDialog(
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
            onPressed: () => Navigator.pop(dialogContext),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () {
              context.read<WalletState>().unlockWallet(passwordController.text);
              Navigator.pop(dialogContext);
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
      builder: (dialogContext) => StatefulBuilder(
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
              onPressed: () => Navigator.pop(dialogContext),
              child: const Text('Cancel'),
            ),
            ElevatedButton(
              onPressed: () {
                if (passwordController.text.isNotEmpty) {
                  context.read<WalletState>().createNewWallet(
                    curve: selectedCurve,
                    password: passwordController.text,
                  );
                  Navigator.pop(dialogContext);
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
      builder: (dialogContext) => DefaultTabController(
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
                                    Navigator.pop(dialogContext);
                                  }
                                },
                                child: const Text('Import PK'),
                              ),
                            ],
                          ),
                        ),
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
                                    Navigator.pop(dialogContext);
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
                onPressed: () => Navigator.pop(dialogContext),
                child: const Text('Cancel'),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
