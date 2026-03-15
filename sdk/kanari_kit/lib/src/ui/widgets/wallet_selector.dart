import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';
import 'package:kanari_kit/src/providers/wallet_provider.dart';
import 'package:kanari_kit/src/kanaricurve.dart';
import 'package:provider/provider.dart';

class WalletSelector extends StatelessWidget {
  const WalletSelector({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<WalletState>();
    final theme = Theme.of(context);
    final screenWidth = MediaQuery.of(context).size.width;
    final isSmallScreen = screenWidth < 360;

    return Container(
      decoration: BoxDecoration(
        color: theme.colorScheme.surface.withOpacity(0.1),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: theme.colorScheme.outline.withOpacity(0.1)),
      ),
      child: PopupMenuButton<String>(
        icon: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.account_balance_wallet_rounded,
              size: isSmallScreen ? 18 : 20,
              color: theme.colorScheme.primary,
            ),
            SizedBox(width: isSmallScreen ? 6 : 8),
            Text(
              '${state.wallets.length}',
              style: TextStyle(
                fontSize: isSmallScreen ? 14 : 16,
                fontWeight: FontWeight.w700,
                color: theme.colorScheme.primary,
              ),
            ),
            const SizedBox(width: 4),
            Icon(
              Icons.arrow_drop_down_rounded,
              color: theme.colorScheme.primary,
              size: isSmallScreen ? 18 : 20,
            ),
          ],
        ),
        onSelected: (walletId) async {
          debugPrint('🔘 Wallet selector clicked: $walletId');

          if (walletId == 'add_new') {
            // Show create wallet dialog
            _showCreateDialog(context);
          } else if (walletId == 'manage') {
            // Navigate to manage wallets (optional future feature)
            ScaffoldMessenger.of(context).showSnackBar(
              const SnackBar(
                content: Text('Manage wallets feature coming soon'),
                duration: Duration(seconds: 2),
              ),
            );
          } else {
            // Switch to selected wallet
            debugPrint('🔄 Switching to wallet: $walletId');
            state.switchWallet(walletId);

            // Show feedback
            final walletName = state.wallets.firstWhere(
              (w) => w['id'] == walletId,
              orElse: () => {'name': 'Unknown'},
            )['name'];

            ScaffoldMessenger.of(context).showSnackBar(
              SnackBar(
                content: Text('Switched to $walletName'),
                duration: const Duration(seconds: 1),
              ),
            );
          }
        },
        itemBuilder: (context) {
          final wallets = state.wallets;
          final activeId = state
              .activeWalletId; // Use the new getter instead of state.wallet?['id']

          final items = wallets.map((wallet) {
            final isActive = wallet['id'] == activeId;
            final name = wallet['name'] ?? 'Wallet';
            final address =
                wallet['address'] ??
                wallet['privateKey'].toString().substring(0, 6);

            return PopupMenuItem<String>(
              value: wallet['id'],
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(
                        isActive ? Icons.check_circle : Icons.circle_outlined,
                        color: isActive ? theme.colorScheme.primary : null,
                        size: 18,
                      ),
                      SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          name,
                          style: TextStyle(
                            fontWeight: isActive
                                ? FontWeight.w700
                                : FontWeight.w500,
                            color: isActive ? theme.colorScheme.primary : null,
                          ),
                        ),
                      ),
                    ],
                  ),
                  SizedBox(height: 4),
                  Text(
                    '0x$address...',
                    style: TextStyle(
                      fontSize: 11,
                      color: theme.colorScheme.onSurface.withOpacity(0.5),
                      fontFamily: 'monospace',
                    ),
                  ),
                ],
              ),
            );
          }).toList();

          // Add "Manage Wallets" separator
          items.add(
            const PopupMenuItem(
              value: 'manage',
              child: Divider(),
              enabled: false,
            ),
          );

          // Add "Add New Wallet" option
          items.add(
            PopupMenuItem(
              value: 'add_new',
              child: ListTile(
                leading: Icon(
                  Icons.add_circle_outline,
                  color: theme.colorScheme.primary,
                ),
                title: Text(
                  'Add New Wallet',
                  style: TextStyle(color: theme.colorScheme.primary),
                ),
              ),
            ),
          );

          return items;
        },
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
                  labelText: 'Set Master Password',
                  border: OutlineInputBorder(),
                  helperText: 'This password will be used for all wallets',
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
                } else {
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(
                      content: Text('Master password is required'),
                      backgroundColor: Colors.red,
                    ),
                  );
                }
              },
              child: const Text('Create'),
            ),
          ],
        ),
      ),
    );
  }
}
