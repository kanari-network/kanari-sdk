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
    final screenPadding = isSmallScreen ? 12.0 : (isMediumScreen ? 16.0 : 20.0);
    final sectionSpacing = isSmallScreen
        ? 16.0
        : (isMediumScreen ? 20.0 : 28.0);

    return Scaffold(
      backgroundColor: theme.colorScheme.surface,
      body: state.isLoading
          ? Center(child: SpinKitFadingCircle(color: theme.colorScheme.primary))
          : RefreshIndicator(
              onRefresh: () => state.refreshBalance(),
              backgroundColor: theme.colorScheme.surface,
              color: theme.colorScheme.primary,
              child: CustomScrollView(
                slivers: [
                  // Modern Header with Gradient
                  SliverToBoxAdapter(
                    child: Container(
                      decoration: BoxDecoration(
                        gradient: LinearGradient(
                          colors: [
                            theme.colorScheme.primary,
                            theme.colorScheme.primary.withOpacity(0.8),
                          ],
                          begin: Alignment.topLeft,
                          end: Alignment.bottomRight,
                        ),
                        borderRadius: const BorderRadius.only(
                          bottomLeft: Radius.circular(32),
                          bottomRight: Radius.circular(32),
                        ),
                      ),
                      child: SafeArea(
                        child: Padding(
                          padding: EdgeInsets.all(screenPadding),
                          child: Column(
                            children: [
                              // Top Bar
                              Row(
                                mainAxisAlignment:
                                    MainAxisAlignment.spaceBetween,
                                children: [
                                  // Logo/Title
                                  Row(
                                    children: [
                                      Container(
                                        padding: const EdgeInsets.all(8),
                                        decoration: BoxDecoration(
                                          color: Colors.white.withOpacity(0.2),
                                          borderRadius: BorderRadius.circular(
                                            12,
                                          ),
                                        ),
                                        child: Icon(
                                          Icons.hexagon,
                                          color: Colors.white,
                                          size: isSmallScreen ? 20 : 24,
                                        ),
                                      ),
                                      SizedBox(width: isSmallScreen ? 8 : 12),
                                      Text(
                                        'Kanari',
                                        style: theme.textTheme.titleLarge
                                            ?.copyWith(
                                              fontWeight: FontWeight.w800,
                                              letterSpacing: -0.5,
                                              color: Colors.white,
                                              fontSize: isSmallScreen ? 18 : 22,
                                            ),
                                      ),
                                    ],
                                  ),
                                  // Settings Menu
                                  PopupMenuButton<String>(
                                    icon: const Icon(
                                      Icons.more_vert_rounded,
                                      color: Colors.white,
                                    ),
                                    color: theme.colorScheme.surface,
                                    onSelected: (value) {
                                      if (value == 'change_password') {
                                        _showChangePasswordDialog(
                                          context,
                                          state,
                                        );
                                      } else if (value == 'logout') {
                                        state.logout();
                                      }
                                    },
                                    itemBuilder: (context) => [
                                      const PopupMenuItem(
                                        value: 'change_password',
                                        child: ListTile(
                                          leading: Icon(
                                            Icons.lock_reset,
                                            color: Colors.blue,
                                          ),
                                          title: Text('Change Password'),
                                        ),
                                      ),
                                      const PopupMenuItem(
                                        value: 'logout',
                                        child: ListTile(
                                          leading: Icon(
                                            Icons.logout_rounded,
                                            color: Colors.red,
                                          ),
                                          title: Text('Logout'),
                                        ),
                                      ),
                                    ],
                                  ),
                                ],
                              ),
                              SizedBox(height: sectionSpacing),

                              // Balance Card - Integrated in header
                              Container(
                                padding: EdgeInsets.all(
                                  isSmallScreen ? 16 : 20,
                                ),
                                decoration: BoxDecoration(
                                  color: Colors.white.withOpacity(0.15),
                                  borderRadius: BorderRadius.circular(24),
                                  border: Border.all(
                                    color: Colors.white.withOpacity(0.2),
                                  ),
                                ),
                                child: Column(
                                  children: [
                                    // Network & Wallet Selectors
                                    Row(
                                      mainAxisAlignment:
                                          MainAxisAlignment.center,
                                      children: [
                                        Container(
                                          padding: EdgeInsets.symmetric(
                                            horizontal: isSmallScreen ? 12 : 16,
                                            vertical: isSmallScreen ? 6 : 8,
                                          ),
                                          decoration: BoxDecoration(
                                            color: Colors.white.withOpacity(
                                              0.2,
                                            ),
                                            borderRadius: BorderRadius.circular(
                                              12,
                                            ),
                                          ),
                                          child: Row(
                                            mainAxisSize: MainAxisSize.min,
                                            children: [
                                              Icon(
                                                Icons.dns_rounded,
                                                color: Colors.white,
                                                size: isSmallScreen ? 14 : 16,
                                              ),
                                              const SizedBox(width: 6),
                                              const NetworkSelector(),
                                            ],
                                          ),
                                        ),
                                        const SizedBox(width: 8),
                                        Container(
                                          padding: EdgeInsets.symmetric(
                                            horizontal: isSmallScreen ? 12 : 16,
                                            vertical: isSmallScreen ? 6 : 8,
                                          ),
                                          decoration: BoxDecoration(
                                            color: Colors.white.withOpacity(
                                              0.2,
                                            ),
                                            borderRadius: BorderRadius.circular(
                                              12,
                                            ),
                                          ),
                                          child: Row(
                                            mainAxisSize: MainAxisSize.min,
                                            children: [
                                              Icon(
                                                Icons
                                                    .account_balance_wallet_rounded,
                                                color: Colors.white,
                                                size: isSmallScreen ? 14 : 16,
                                              ),
                                              const SizedBox(width: 6),
                                              const WalletSelector(),
                                            ],
                                          ),
                                        ),
                                      ],
                                    ),
                                    SizedBox(height: isSmallScreen ? 16 : 20),

                                    // Balance Amount
                                    const Text(
                                      'Total Balance',
                                      style: TextStyle(
                                        color: Colors.white70,
                                        fontSize: 12,
                                        fontWeight: FontWeight.w500,
                                      ),
                                    ),
                                    SizedBox(height: isSmallScreen ? 4 : 8),
                                    const BalanceCard(),

                                    // Refresh Button
                                    SizedBox(height: isSmallScreen ? 12 : 16),
                                    Material(
                                      color: Colors.white.withOpacity(0.2),
                                      borderRadius: BorderRadius.circular(12),
                                      child: InkWell(
                                        borderRadius: BorderRadius.circular(12),
                                        onTap: () => state.refreshBalance(),
                                        child: Padding(
                                          padding: const EdgeInsets.symmetric(
                                            horizontal: 16,
                                            vertical: 8,
                                          ),
                                          child: Row(
                                            mainAxisSize: MainAxisSize.min,
                                            children: [
                                              Icon(
                                                Icons.refresh_rounded,
                                                color: Colors.white,
                                                size: 16,
                                              ),
                                              const SizedBox(width: 6),
                                              const Text(
                                                'Refresh',
                                                style: TextStyle(
                                                  color: Colors.white,
                                                  fontSize: 13,
                                                  fontWeight: FontWeight.w600,
                                                ),
                                              ),
                                            ],
                                          ),
                                        ),
                                      ),
                                    ),
                                  ],
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                    ),
                  ),

                  // Wallet Address Card
                  SliverToBoxAdapter(
                    child: Padding(
                      padding: EdgeInsets.all(screenPadding),
                      child: const WalletInfoCard(),
                    ),
                  ),

                  // Security Card
                  SliverToBoxAdapter(
                    child: Padding(
                      padding: EdgeInsets.symmetric(horizontal: screenPadding),
                      child: const SecurityCard(),
                    ),
                  ),

                  // Quick Actions Section
                  SliverToBoxAdapter(
                    child: Padding(
                      padding: EdgeInsets.all(screenPadding),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            children: [
                              Container(
                                padding: EdgeInsets.symmetric(
                                  horizontal: isSmallScreen ? 12 : 16,
                                  vertical: isSmallScreen ? 6 : 8,
                                ),
                                decoration: BoxDecoration(
                                  color: theme.colorScheme.primaryContainer,
                                  borderRadius: BorderRadius.circular(12),
                                ),
                                child: Row(
                                  mainAxisSize: MainAxisSize.min,
                                  children: [
                                    Icon(
                                      Icons.bolt_rounded,
                                      size: isSmallScreen ? 14 : 16,
                                      color:
                                          theme.colorScheme.onPrimaryContainer,
                                    ),
                                    SizedBox(width: isSmallScreen ? 6 : 8),
                                    Text(
                                      'Quick Actions',
                                      style: theme.textTheme.titleMedium
                                          ?.copyWith(
                                            fontWeight: FontWeight.w700,
                                            letterSpacing: -0.3,
                                          ),
                                    ),
                                  ],
                                ),
                              ),
                            ],
                          ),
                          SizedBox(height: isSmallScreen ? 16.0 : 20.0),
                          _buildActionList(context, isSmallScreen),
                        ],
                      ),
                    ),
                  ),

                  // Bottom Spacing
                  const SliverToBoxAdapter(child: SizedBox(height: 20)),
                ],
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

  Future<String?> _scanQRCode(BuildContext context) async {
    // Placeholder for QR scanner - will be implemented with platform-specific code
    // For now, show a message that this feature requires camera permission
    showDialog(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('QR Scanner'),
        content: const Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(Icons.qr_code_scanner_rounded, size: 64, color: Colors.blue),
            SizedBox(height: 16),
            Text(
              'QR Scanner requires camera access.\n\nThis feature will be available soon.',
              textAlign: TextAlign.center,
              style: TextStyle(fontSize: 14),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(dialogContext),
            child: const Text('OK'),
          ),
        ],
      ),
    );

    return null;
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

  void _showTransferDialog(BuildContext context, {String? prefilledAddress}) {
    final recipientController = TextEditingController(text: prefilledAddress);
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
                  suffixIcon: IconButton(
                    icon: const Icon(Icons.qr_code_scanner_rounded),
                    onPressed: () async {
                      Navigator.pop(dialogContext);
                      final scannedAddress = await _scanQRCode(context);
                      if (scannedAddress != null && dialogContext.mounted) {
                        // Reopen dialog with scanned address
                        _showTransferDialog(
                          context,
                          prefilledAddress: scannedAddress,
                        );
                      }
                    },
                    tooltip: 'Scan QR Code',
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
                  suffixIcon: TextButton(
                    onPressed: () {
                      // Calculate max amount (balance in KANARI)
                      final balanceMist =
                          context.read<WalletState>().balance ?? 0;
                      final balanceKanari = balanceMist / 1000000000;
                      amountController.text = balanceKanari.toStringAsFixed(6);
                    },
                    style: TextButton.styleFrom(
                      padding: EdgeInsets.symmetric(
                        horizontal: isSmallScreen ? 8 : 12,
                        vertical: isSmallScreen ? 4 : 8,
                      ),
                    ),
                    child: Container(
                      padding: EdgeInsets.symmetric(
                        horizontal: isSmallScreen ? 8 : 12,
                        vertical: isSmallScreen ? 4 : 6,
                      ),
                      decoration: BoxDecoration(
                        color: Theme.of(
                          dialogContext,
                        ).colorScheme.primary.withOpacity(0.1),
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: Text(
                        'MAX',
                        style: TextStyle(
                          fontSize: isSmallScreen ? 11 : 12,
                          fontWeight: FontWeight.w700,
                          color: Theme.of(dialogContext).colorScheme.primary,
                          letterSpacing: 0.5,
                        ),
                      ),
                    ),
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
                        backgroundColor: Theme.of(
                          dialogContext,
                        ).colorScheme.error,
                        behavior: SnackBarBehavior.floating,
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(16),
                        ),
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
                        content: const Text(
                          'Invalid address format. Use hex characters only (0-9, a-f)',
                        ),
                        backgroundColor: Theme.of(
                          dialogContext,
                        ).colorScheme.error,
                        behavior: SnackBarBehavior.floating,
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(16),
                        ),
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
                        content: Text(
                          'Address must be exactly 64 hex characters. Current: ${cleanAddress.length}',
                        ),
                        backgroundColor: Theme.of(
                          dialogContext,
                        ).colorScheme.error,
                        behavior: SnackBarBehavior.floating,
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(16),
                        ),
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
                        backgroundColor: Theme.of(
                          dialogContext,
                        ).colorScheme.error,
                        behavior: SnackBarBehavior.floating,
                        shape: RoundedRectangleBorder(
                          borderRadius: BorderRadius.circular(16),
                        ),
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
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(16),
                      ),
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

  void _showChangePasswordDialog(BuildContext context, WalletState state) {
    final oldPasswordController = TextEditingController();
    final newPasswordController = TextEditingController();
    final confirmPasswordController = TextEditingController();

    showDialog(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('Change Password'),
        content: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(
                controller: oldPasswordController,
                decoration: const InputDecoration(
                  labelText: 'Old Password',
                  border: OutlineInputBorder(),
                ),
                obscureText: true,
              ),
              const SizedBox(height: 12),
              TextField(
                controller: newPasswordController,
                decoration: const InputDecoration(
                  labelText: 'New Password',
                  border: OutlineInputBorder(),
                ),
                obscureText: true,
              ),
              const SizedBox(height: 12),
              TextField(
                controller: confirmPasswordController,
                decoration: const InputDecoration(
                  labelText: 'Confirm New Password',
                  border: OutlineInputBorder(),
                ),
                obscureText: true,
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
              final oldPassword = oldPasswordController.text;
              final newPassword = newPasswordController.text;
              final confirmPassword = confirmPasswordController.text;

              if (oldPassword.isEmpty ||
                  newPassword.isEmpty ||
                  confirmPassword.isEmpty) {
                if (dialogContext.mounted) {
                  ScaffoldMessenger.of(dialogContext).showSnackBar(
                    SnackBar(
                      content: const Text('All fields are required'),
                      backgroundColor: Theme.of(
                        dialogContext,
                      ).colorScheme.error,
                      behavior: SnackBarBehavior.floating,
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(16),
                      ),
                    ),
                  );
                }
                return;
              }

              if (newPassword != confirmPassword) {
                if (dialogContext.mounted) {
                  ScaffoldMessenger.of(dialogContext).showSnackBar(
                    SnackBar(
                      content: const Text('Passwords do not match'),
                      backgroundColor: Theme.of(
                        dialogContext,
                      ).colorScheme.error,
                      behavior: SnackBarBehavior.floating,
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(16),
                      ),
                    ),
                  );
                }
                return;
              }

              Navigator.pop(dialogContext);
              final success = await state.changePassword(
                oldPassword,
                newPassword,
              );
              if (context.mounted) {
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(
                    content: Text(
                      success
                          ? 'Password changed successfully'
                          : 'Failed to change password. Please check your old password.',
                    ),
                    backgroundColor: success
                        ? Theme.of(context).colorScheme.primary
                        : Theme.of(context).colorScheme.error,
                    behavior: SnackBarBehavior.floating,
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(16),
                    ),
                  ),
                );
              }
            },
            child: const Text('Change Password'),
          ),
        ],
      ),
    );
  }
}
