import 'package:flutter/material.dart';
import 'package:kanari_kit/src/providers/wallet_provider.dart';
import 'package:provider/provider.dart';
import 'package:flutter_spinkit/flutter_spinkit.dart';

import '../widgets/action_button.dart';
import '../widgets/security_card.dart';
import '../balance_card.dart';
import '../network_selector.dart';
import '../wallet_info_card.dart';

class HomeScreen extends StatelessWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<WalletState>();
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: Text(
          'Kanari',
          style: theme.textTheme.titleLarge?.copyWith(
            fontWeight: FontWeight.bold,
          ),
        ),
        actions: [
          const NetworkSelector(),
          const SizedBox(width: 8),
          IconButton(
            icon: const Icon(Icons.refresh_rounded),
            onPressed: state.isLoading ? null : () => state.refreshBalance(),
          ),
          IconButton(
            icon: const Icon(Icons.logout_rounded),
            onPressed: () => state.logout(),
            tooltip: 'Logout',
          ),
          const SizedBox(width: 8),
        ],
      ),
      body: state.isLoading
          ? Center(child: SpinKitFadingCircle(color: theme.colorScheme.primary))
          : RefreshIndicator(
              onRefresh: () => state.refreshBalance(),
              child: SingleChildScrollView(
                physics: const AlwaysScrollableScrollPhysics(),
                padding: const EdgeInsets.all(20.0),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    const BalanceCard(),
                    const SizedBox(height: 24),
                    const WalletInfoCard(),
                    const SizedBox(height: 24),
                    const SecurityCard(),
                    const SizedBox(height: 32),
                    Text(
                      'Actions',
                      style: theme.textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.bold,
                      ),
                    ),
                    const SizedBox(height: 16),
                    _buildActionGrid(context),
                  ],
                ),
              ),
            ),
    );
  }

  Widget _buildActionGrid(BuildContext context) {
    return Column(
      children: [
        ActionButton(
          onPressed: () => _showTransferDialog(context),
          icon: Icons.send_rounded,
          label: 'Send KANARI',
          description: 'Transfer tokens',
          isPrimary: true,
        ),
        const SizedBox(height: 12),
        Row(
          children: [
            Expanded(
              child: ActionButton(
                onPressed: () => _showExecuteDialog(context),
                icon: Icons.code_rounded,
                label: 'Execute',
                description: 'Move function',
              ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: ActionButton(
                onPressed: () => _showBurnDialog(context),
                icon: Icons.local_fire_department_rounded,
                label: 'Burn',
                description: 'Destroy tokens',
                color: Colors.orangeAccent,
              ),
            ),
          ],
        ),
      ],
    );
  }

  void _showExecuteDialog(BuildContext context) {
    final packageController = TextEditingController();
    final moduleController = TextEditingController();
    final functionController = TextEditingController();

    showDialog(
      context: context,
      builder: (dialogContext) => AlertDialog(
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
            onPressed: () => Navigator.pop(dialogContext),
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

              Navigator.pop(dialogContext);
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
      builder: (dialogContext) => AlertDialog(
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
            onPressed: () => Navigator.pop(dialogContext),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () async {
              final amountStr = amountController.text;
              final amountDouble = double.tryParse(amountStr) ?? 0.0;
              final amountMist = (amountDouble * 1000000000).round();

              if (amountMist <= 0) return;

              Navigator.pop(dialogContext);
              final result = await context.read<WalletState>().burn(amountMist);
              if (context.mounted) {
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(content: Text(result ?? "Unknown error")),
                );
              }
            },
            style: ElevatedButton.styleFrom(
              backgroundColor: Colors.orangeAccent,
              foregroundColor: Colors.white,
            ),
            child: const Text('Burn'),
          ),
        ],
      ),
    );
  }

  void _showTransferDialog(BuildContext context) {
    final recipientController = TextEditingController();
    final amountController = TextEditingController();

    showDialog(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('Transfer KANARI'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            TextField(
              controller: recipientController,
              decoration: const InputDecoration(
                labelText: 'Recipient Address',
                border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 12),
            TextField(
              controller: amountController,
              decoration: const InputDecoration(
                labelText: 'Amount',
                border: OutlineInputBorder(),
              ),
              keyboardType: TextInputType.number,
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(dialogContext),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            onPressed: () async {
              final recipient = recipientController.text;
              final amountStr = amountController.text;
              final amountDouble = double.tryParse(amountStr) ?? 0.0;
              final amountMist = (amountDouble * 1000000000).round();

              if (recipient.isEmpty || amountMist <= 0) return;

              Navigator.pop(dialogContext);
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
