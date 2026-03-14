import 'package:flutter/material.dart';
import 'package:kanari_kit/src/providers/wallet_provider.dart';
import 'package:provider/provider.dart';
import 'package:flutter_spinkit/flutter_spinkit.dart';

import '../widgets/action_button.dart';
import '../widgets/security_card.dart';
import '../balance_card.dart';
import '../network_selector.dart';
import '../wallet_info_card.dart';
import '../widgets/wallet_selector.dart';

class HomeScreen extends StatelessWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<WalletState>();
    final theme = Theme.of(context);
    final screenWidth = MediaQuery.of(context).size.width;
    final isSmallScreen = screenWidth < 360;
    final isMediumScreen = screenWidth >= 360 && screenWidth < 600;
    
    // Responsive padding
    final screenPadding = isSmallScreen ? 16.0 : (isMediumScreen ? 20.0 : 24.0);
    final sectionSpacing = isSmallScreen ? 20.0 : (isMediumScreen ? 24.0 : 32.0);
    final topSpacing = isSmallScreen ? 12.0 : (isMediumScreen ? 16.0 : 20.0);

    return Scaffold(
      extendBodyBehindAppBar: true,
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        elevation: 0,
        title: Text(
          'Kanari',
          style: theme.textTheme.titleLarge?.copyWith(
            fontWeight: FontWeight.w800,
            letterSpacing: -0.5,
            color: theme.colorScheme.onSurface,
          ),
        ),
        actions: [
          const WalletSelector(),
          SizedBox(width: isSmallScreen ? 4 : 8),
          const NetworkSelector(),
          SizedBox(width: isSmallScreen ? 4 : 8),
          Container(
            decoration: BoxDecoration(
              color: theme.colorScheme.surface.withOpacity(0.1),
              borderRadius: BorderRadius.circular(12),
              border: Border.all(
                color: theme.colorScheme.outline.withOpacity(0.1),
              ),
            ),
            child: IconButton(
              icon: const Icon(Icons.refresh_rounded),
              onPressed: state.isLoading ? null : () => state.refreshBalance(),
              style: IconButton.styleFrom(
                padding: EdgeInsets.all(isSmallScreen ? 8 : 10),
              ),
            ),
          ),
          SizedBox(width: isSmallScreen ? 4 : 8),
          Container(
            decoration: BoxDecoration(
              color: theme.colorScheme.error.withOpacity(0.1),
              borderRadius: BorderRadius.circular(12),
              border: Border.all(
                color: theme.colorScheme.error.withOpacity(0.2),
              ),
            ),
            child: IconButton(
              icon: const Icon(Icons.logout_rounded),
              onPressed: () => state.logout(),
              tooltip: 'Logout',
              color: theme.colorScheme.error,
              style: IconButton.styleFrom(
                padding: EdgeInsets.all(isSmallScreen ? 8 : 10),
              ),
            ),
          ),
          SizedBox(width: isSmallScreen ? 12 : 16),
        ],
      ),
      body: Container(
        decoration: BoxDecoration(
          gradient: LinearGradient(
            colors: [
              theme.colorScheme.primary.withOpacity(0.03),
              theme.colorScheme.secondary.withOpacity(0.02),
            ],
            begin: Alignment.topCenter,
            end: Alignment.bottomCenter,
          ),
        ),
        child: state.isLoading
            ? Center(child: SpinKitFadingCircle(color: theme.colorScheme.primary))
            : RefreshIndicator(
                onRefresh: () => state.refreshBalance(),
                backgroundColor: theme.colorScheme.surface,
                color: theme.colorScheme.primary,
                child: SingleChildScrollView(
                  physics: const AlwaysScrollableScrollPhysics(),
                  padding: EdgeInsets.all(screenPadding),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      SizedBox(height: topSpacing),
                      const BalanceCard(),
                      SizedBox(height: sectionSpacing),
                      const WalletInfoCard(),
                      SizedBox(height: sectionSpacing),
                      const SecurityCard(),
                      SizedBox(height: isSmallScreen ? 28.0 : 48.0),
                      Row(
                        children: [
                          Container(
                            padding: EdgeInsets.symmetric(
                              horizontal: isSmallScreen ? 12 : 16,
                              vertical: isSmallScreen ? 6 : 8,
                            ),
                            decoration: BoxDecoration(
                              color: theme.colorScheme.primary.withOpacity(0.1),
                              borderRadius: BorderRadius.circular(12),
                              border: Border.all(
                                color: theme.colorScheme.primary.withOpacity(0.2),
                              ),
                            ),
                            child: Row(
                              mainAxisSize: MainAxisSize.min,
                              children: [
                                Icon(
                                  Icons.bolt_rounded,
                                  size: isSmallScreen ? 14 : 16,
                                  color: theme.colorScheme.primary,
                                ),
                                SizedBox(width: isSmallScreen ? 6 : 8),
                                Text(
                                  'Quick Actions',
                                  style: theme.textTheme.titleMedium?.copyWith(
                                    fontWeight: FontWeight.w700,
                                    letterSpacing: -0.3,
                                  ),
                                ),
                              ],
                            ),
                          ),
                        ],
                      ),
                      SizedBox(height: isSmallScreen ? 16.0 : 24.0),
                      _buildActionList(context, isSmallScreen),
                    ],
                  ),
                ),
              ),
      ),
    );
  }

  Widget _buildActionList(BuildContext context, bool isSmallScreen) {
    if (isSmallScreen) {
      // Stack buttons vertically on very small screens
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
          ActionButton(
            onPressed: () => _showExecuteDialog(context),
            icon: Icons.code_rounded,
            label: 'Execute',
            description: 'Move function',
          ),
          const SizedBox(height: 12),
          ActionButton(
            onPressed: () => _showBurnDialog(context),
            icon: Icons.local_fire_department_rounded,
            label: 'Burn',
            description: 'Destroy tokens',
            color: Colors.orangeAccent,
          ),
        ],
      );
    } else {
      // Use row layout for larger screens
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
    final screenWidth = MediaQuery.of(context).size.width;
    final isSmallScreen = screenWidth < 360;

    showDialog(
      context: context,
      builder: (dialogContext) => AlertDialog(
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(24)),
        title: Padding(
          padding: EdgeInsets.only(bottom: isSmallScreen ? 12 : 16),
          child: Text(
            'Transfer KANARI',
            style: TextStyle(
              fontWeight: FontWeight.w700,
              letterSpacing: -0.5,
              fontSize: isSmallScreen ? 17 : 20,
            ),
          ),
        ),
        content: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              SizedBox(height: isSmallScreen ? 4 : 8),
              TextField(
                controller: recipientController,
                style: TextStyle(
                  fontFamily: 'monospace',
                  fontSize: isSmallScreen ? 12 : 13,
                  letterSpacing: 0.3,
                ),
                decoration: InputDecoration(
                  labelText: 'Recipient Address',
                  hintText: '0x + 64 hex characters',
                  helperText: 'Must be exactly 64 hex characters',
                  helperStyle: TextStyle(
                    fontSize: isSmallScreen ? 10 : 11,
                    color: Theme.of(
                      dialogContext,
                    ).colorScheme.onSurface.withOpacity(0.5),
                  ),
                  border: OutlineInputBorder(
                    borderRadius: BorderRadius.circular(16),
                  ),
                  filled: true,
                  fillColor: Theme.of(
                    dialogContext,
                  ).colorScheme.surfaceVariant.withOpacity(0.1),
                  contentPadding: EdgeInsets.symmetric(
                    horizontal: isSmallScreen ? 12 : 16,
                    vertical: isSmallScreen ? 12 : 16,
                  ),
                ),
              ),
              SizedBox(height: isSmallScreen ? 12 : 16),
              TextField(
                controller: amountController,
                style: TextStyle(
                  fontSize: isSmallScreen ? 14 : 16,
                  fontWeight: FontWeight.w600,
                ),
                keyboardType: TextInputType.number,
                decoration: InputDecoration(
                  labelText: 'Amount (KANARI)',
                  border: OutlineInputBorder(
                    borderRadius: BorderRadius.circular(16),
                  ),
                  filled: true,
                  fillColor: Theme.of(
                    dialogContext,
                  ).colorScheme.surfaceVariant.withOpacity(0.1),
                  prefixIcon: Icon(
                    Icons.account_balance_wallet_rounded,
                    size: isSmallScreen ? 20 : 24,
                  ),
                  contentPadding: EdgeInsets.symmetric(
                    horizontal: isSmallScreen ? 12 : 16,
                    vertical: isSmallScreen ? 12 : 16,
                  ),
                ),
              ),
              SizedBox(height: isSmallScreen ? 4 : 8),
            ],
          ),
        ),
        actionsPadding: EdgeInsets.all(isSmallScreen ? 12 : 16),
        actions: [
          SizedBox(
            width: double.infinity,
            child: OutlinedButton(
              onPressed: () => Navigator.pop(dialogContext),
              style: OutlinedButton.styleFrom(
                foregroundColor: Theme.of(dialogContext).colorScheme.onSurface,
                side: BorderSide(
                  color: Theme.of(
                    dialogContext,
                  ).colorScheme.outline.withOpacity(0.3),
                ),
                padding: EdgeInsets.symmetric(
                  vertical: isSmallScreen ? 12 : 16,
                ),
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(16),
                ),
              ),
              child: Text(
                'Cancel',
                style: TextStyle(
                  fontSize: isSmallScreen ? 13 : 15,
                  fontWeight: FontWeight.w600,
                ),
              ),
            ),
          ),
          SizedBox(height: isSmallScreen ? 8 : 12),
          SizedBox(
            width: double.infinity,
            child: ElevatedButton(
              onPressed: () async {
                final recipient = recipientController.text;
                final amountStr = amountController.text;
                final amountDouble = double.tryParse(amountStr) ?? 0.0;
                final amountMist = (amountDouble * 1000000000).round();

                // Validate recipient address format
                if (recipient.isEmpty) {
                  if (dialogContext.mounted) {
                    ScaffoldMessenger.of(dialogContext).showSnackBar(
                      SnackBar(
                        content: const Text('Recipient address is required'),
                        backgroundColor: Theme.of(dialogContext).colorScheme.error,
                        behavior: SnackBarBehavior.floating,
                        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
                      ),
                    );
                  }
                  return;
                }

                // Basic format validation before sending to client
                // Address must be exactly 64 hex characters (with or without 0x prefix)
                var cleanAddress = recipient.startsWith('0x')
                    ? recipient.substring(2)
                    : recipient;

                if (!RegExp(r'^[0-9a-fA-F]+$').hasMatch(cleanAddress)) {
                  if (dialogContext.mounted) {
                    ScaffoldMessenger.of(dialogContext).showSnackBar(
                      SnackBar(
                        content: const Text('Invalid address format. Use hex characters only (0-9, a-f)'),
                        backgroundColor: Theme.of(dialogContext).colorScheme.error,
                        behavior: SnackBarBehavior.floating,
                        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
                        duration: const Duration(seconds: 5),
                      ),
                    );
                  }
                  return;
                }

                if (cleanAddress.length != 64) {
                  if (dialogContext.mounted) {
                    ScaffoldMessenger.of(dialogContext).showSnackBar(
                      SnackBar(
                        content: Text('Address must be exactly 64 hex characters. Current: ${cleanAddress.length}'),
                        backgroundColor: Theme.of(dialogContext).colorScheme.error,
                        behavior: SnackBarBehavior.floating,
                        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
                        duration: const Duration(seconds: 6),
                      ),
                    );
                  }
                  return;
                }

                if (amountMist <= 0) {
                  if (dialogContext.mounted) {
                    ScaffoldMessenger.of(dialogContext).showSnackBar(
                      SnackBar(
                        content: const Text('Amount must be greater than 0'),
                        backgroundColor: Theme.of(dialogContext).colorScheme.error,
                        behavior: SnackBarBehavior.floating,
                        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
                      ),
                    );
                  }
                  return;
                }

                Navigator.pop(dialogContext);
                final result = await context.read<WalletState>().transfer(
                  recipient,
                  amountMist,
                );
                if (context.mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(
                      content: Text(
                        result?.startsWith('Error:') == true
                            ? result!
                            : 'Transaction successful',
                      ),
                      backgroundColor: result?.startsWith('Error:') == true
                          ? Theme.of(context).colorScheme.error
                          : Theme.of(context).colorScheme.primary,
                      behavior: SnackBarBehavior.floating,
                      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
                    ),
                  );
                }
              },
              style: ElevatedButton.styleFrom(
                backgroundColor: Theme.of(dialogContext).colorScheme.primary,
                foregroundColor: Colors.white,
                padding: EdgeInsets.symmetric(
                  vertical: isSmallScreen ? 12 : 16,
                ),
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(16),
                ),
                elevation: 0,
              ),
              child: Text(
                'Send',
                style: TextStyle(
                  fontSize: isSmallScreen ? 13 : 15,
                  fontWeight: FontWeight.w600,
                  letterSpacing: 0.5,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
