import 'dart:async';
import 'dart:ui'; // สำหรับ PointerDeviceKind
import 'package:flutter/material.dart';
import 'package:flutter_spinkit/flutter_spinkit.dart';
import 'package:provider/provider.dart';

import 'package:kanari_kit/src/providers/wallet_provider.dart';
import 'package:kanari_kit/kanari_kit.dart';
import '../widgets/action_button.dart';
import '../widgets/security_card.dart';
import '../network_selector.dart';
import '../wallet_info_card.dart';

class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key});

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  late PageController _pageController;
  Timer? _debounce;

  @override
  void initState() {
    super.initState();
    final state = context.read<WalletState>();
    int initialPage = 0;
    if (state.activeWalletId != null) {
      initialPage = state.wallets.indexWhere(
        (w) => w['id'] == state.activeWalletId,
      );
      if (initialPage == -1) initialPage = 0;
    }

    _pageController = PageController(
      initialPage: initialPage,
      viewportFraction: 0.9,
    );
  }

  @override
  void dispose() {
    _pageController.dispose();
    _debounce?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<WalletState>();
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final screenWidth = MediaQuery.of(context).size.width;
    final isSmallScreen = screenWidth < 360;
    final isMediumScreen = screenWidth >= 360 && screenWidth < 600;

    final screenPadding = isSmallScreen ? 12.0 : (isMediumScreen ? 16.0 : 20.0);
    final wallets = state.wallets;
    final pageCount = wallets.length + 1;

    return Scaffold(
      backgroundColor: colorScheme.surface,
      body: state.isLoading
          ? Center(child: SpinKitFadingCircle(color: colorScheme.primary))
          : RefreshIndicator(
              onRefresh: () => state.refreshBalance(),
              backgroundColor: colorScheme.surface,
              color: colorScheme.primary,
              child: CustomScrollView(
                slivers: [
                  // 1. Navbar แบบกล่องลอย (Floating Island) เลื่อนแล้วหาย
                  SliverAppBar(
                    floating: true, // ทำให้เวลาปัดขึ้น แถบจะแสดงทันที
                    snap: true, // เด้งโชว์เต็มๆ ไม่มาแค่ครึ่งเดียว
                    pinned: false, // เปลี่ยนเป็น false เพื่อให้ปัดลงแล้วหายไป
                    backgroundColor: Colors.transparent,
                    surfaceTintColor: Colors.transparent,
                    elevation: 0,
                    toolbarHeight: 88, // 👈 เพิ่มความสูงอีก 12 (จาก 76 เป็น 88)
                    flexibleSpace: SafeArea(
                      child: Container(
                        // ใช้ margin เพื่อสร้างขอบรอบๆ Navbar
                        margin: EdgeInsets.symmetric(
                          horizontal: screenPadding,
                          vertical: 12, // 👈 ปรับระยะห่างขอบบน-ล่างให้สมดุล
                        ),
                        padding: const EdgeInsets.symmetric(
                          horizontal: 12,
                          vertical: 8,
                        ), // 👈 เพิ่ม Padding ด้านในกันเนื้อหาชนขอบ
                        decoration: BoxDecoration(
                          color: colorScheme.surfaceContainerHigh.withOpacity(
                            0.6,
                          ),
                          borderRadius: BorderRadius.circular(24),
                          border: Border.all(
                            color: colorScheme.outline.withOpacity(0.1),
                          ),
                        ),
                        child: Row(
                          mainAxisAlignment: MainAxisAlignment.spaceBetween,
                          children: [
                            // ฝั่งซ้าย: โลโก้และชื่อ
                            Row(
                              children: [
                                Container(
                                  padding: const EdgeInsets.all(
                                    10,
                                  ), // 👈 ขยายกรอบโลโก้นิดหน่อย
                                  decoration: BoxDecoration(
                                    color: colorScheme.primaryContainer,
                                    borderRadius: BorderRadius.circular(12),
                                  ),
                                  child: Icon(
                                    Icons.hexagon_rounded,
                                    color: colorScheme.onPrimaryContainer,
                                    size: 20,
                                  ),
                                ),
                                const SizedBox(width: 12),
                                Text(
                                  'Kanari',
                                  style: theme.textTheme.titleLarge?.copyWith(
                                    fontWeight: FontWeight.w800,
                                    letterSpacing: -0.5,
                                    color: colorScheme.onSurface,
                                  ),
                                ),
                              ],
                            ),
                            // ฝั่งขวา: Network Selector & Menu
                            Row(
                              children: [
                                const NetworkSelector(),
                                const SizedBox(width: 4),
                                PopupMenuButton<String>(
                                  icon: Icon(
                                    Icons.more_vert_rounded,
                                    color: colorScheme.onSurfaceVariant,
                                  ),
                                  color: colorScheme.surfaceContainer,
                                  shape: RoundedRectangleBorder(
                                    borderRadius: BorderRadius.circular(16),
                                  ),
                                  onSelected: (value) {
                                    if (value == 'change_password') {
                                      _showChangePasswordDialog(context, state);
                                    } else if (value == 'logout') {
                                      state.logout();
                                    }
                                  },
                                  itemBuilder: (context) => [
                                    PopupMenuItem(
                                      value: 'change_password',
                                      child: ListTile(
                                        leading: Icon(
                                          Icons.lock_reset_rounded,
                                          color: colorScheme.primary,
                                        ),
                                        title: const Text('Change Password'),
                                        contentPadding: EdgeInsets.zero,
                                      ),
                                    ),
                                    PopupMenuItem(
                                      value: 'logout',
                                      child: ListTile(
                                        leading: Icon(
                                          Icons.logout_rounded,
                                          color: colorScheme.error,
                                        ),
                                        title: const Text('Logout'),
                                        contentPadding: EdgeInsets.zero,
                                      ),
                                    ),
                                  ],
                                ),
                              ],
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),

                  // 2. Swipable Wallet Carousel (พื้นที่ปัดการ์ด)
                  SliverToBoxAdapter(
                    child: Padding(
                      padding: const EdgeInsets.only(top: 8.0),
                      child: SizedBox(
                        height: isSmallScreen ? 280 : 340,
                        child: PageView.builder(
                          controller: _pageController,
                          itemCount: pageCount,
                          scrollDirection: Axis.horizontal,
                          physics: const BouncingScrollPhysics(),
                          scrollBehavior: const MaterialScrollBehavior()
                              .copyWith(
                                dragDevices: {
                                  PointerDeviceKind.touch,
                                  PointerDeviceKind.mouse,
                                },
                              ),
                          onPageChanged: (index) {
                            if (_debounce?.isActive ?? false) {
                              _debounce!.cancel();
                            }
                            _debounce = Timer(
                              const Duration(milliseconds: 300),
                              () {
                                if (mounted && index < wallets.length) {
                                  final targetWalletId = wallets[index]['id'];
                                  if (context
                                          .read<WalletState>()
                                          .activeWalletId !=
                                      targetWalletId) {
                                    context.read<WalletState>().switchWallet(
                                      targetWalletId,
                                    );
                                  }
                                }
                              },
                            );
                          },
                          itemBuilder: (context, index) {
                            return Padding(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 8.0,
                                vertical: 8.0,
                              ),
                              child: (index < wallets.length)
                                  ? _buildWalletCardPage(
                                      context,
                                      colorScheme,
                                      wallets[index],
                                      theme,
                                      isSmallScreen,
                                    )
                                  : _buildCreateWalletCardPage(
                                      context,
                                      colorScheme,
                                      theme,
                                      isSmallScreen,
                                    ),
                            );
                          },
                        ),
                      ),
                    ),
                  ),

                  // 3. Wallet Address Section
                  SliverToBoxAdapter(
                    child: Padding(
                      padding: EdgeInsets.fromLTRB(
                        screenPadding,
                        24,
                        screenPadding,
                        0,
                      ),
                      child: const WalletInfoCard(),
                    ),
                  ),

                  // 4. Security Card Section
                  SliverToBoxAdapter(
                    child: Padding(
                      padding: EdgeInsets.fromLTRB(
                        screenPadding,
                        20,
                        screenPadding,
                        0,
                      ),
                      child: const SecurityCard(),
                    ),
                  ),

                  // 5. Quick Actions Section
                  SliverToBoxAdapter(
                    child: Padding(
                      padding: EdgeInsets.fromLTRB(
                        screenPadding,
                        20,
                        screenPadding,
                        40,
                      ),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            'Quick Actions',
                            style: theme.textTheme.titleMedium?.copyWith(
                              fontWeight: FontWeight.bold,
                              color: colorScheme.onSurface,
                            ),
                          ),
                          SizedBox(height: isSmallScreen ? 12.0 : 16.0),
                          ActionButton(
                            onPressed: () => _showTransferDialog(context),
                            icon: Icons.send_rounded,
                            label: 'Send KANARI',
                            description: 'Transfer tokens to another address',
                            isPrimary: true,
                          ),
                        ],
                      ),
                    ),
                  ),

                  const SliverToBoxAdapter(child: SizedBox(height: 40)),
                ],
              ),
            ),
    );
  }

  // --- Helper Methods ---

  Widget _buildWalletCardPage(
    BuildContext context,
    ColorScheme colorScheme,
    Map<String, dynamic> walletData,
    ThemeData theme,
    bool isSmallScreen,
  ) {
    return Container(
      padding: EdgeInsets.all(isSmallScreen ? 16 : 24),
      decoration: BoxDecoration(
        gradient: LinearGradient(
          colors: [colorScheme.primary.withOpacity(0.85), colorScheme.primary],
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
        ),
        borderRadius: BorderRadius.circular(32),
        boxShadow: [
          BoxShadow(
            color: colorScheme.primary.withOpacity(0.2),
            blurRadius: 15,
            offset: const Offset(0, 8),
          ),
        ],
      ),
      child: Column(
        children: [
          Text(
            walletData['name'] ?? 'Wallet',
            style: theme.textTheme.titleSmall?.copyWith(
              color: colorScheme.onPrimary.withOpacity(0.8),
              fontWeight: FontWeight.bold,
              letterSpacing: 0.5,
            ),
          ),
          const SizedBox(height: 8),
          _buildIndexSpecificBalanceSection(
            context,
            colorScheme,
            walletData,
            isSmallScreen,
            theme,
          ),
          SizedBox(height: isSmallScreen ? 16 : 24),
          FilledButton.tonalIcon(
            onPressed: () {
              if (walletData['id'] ==
                  context.read<WalletState>().activeWalletId) {
                context.read<WalletState>().refreshBalance();
              }
            },
            icon: const Icon(Icons.refresh_rounded, size: 16),
            label: const Text('Refresh'),
            style: FilledButton.styleFrom(
              backgroundColor: colorScheme.onPrimary.withOpacity(0.15),
              foregroundColor: colorScheme.onPrimary,
              padding: const EdgeInsets.symmetric(horizontal: 24),
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(20),
              ),
              elevation: 0,
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildIndexSpecificBalanceSection(
    BuildContext context,
    ColorScheme colorScheme,
    Map<String, dynamic> walletData,
    bool isSmallScreen,
    ThemeData theme,
  ) {
    final state = context.watch<WalletState>();
    final isActive = walletData['id'] == state.activeWalletId;
    final displayBalance = isActive
        ? (state.balance / 1000000000).toStringAsFixed(6)
        : "---";

    return Container(
      width: double.infinity,
      decoration: BoxDecoration(
        color: colorScheme.onPrimary.withOpacity(0.08),
        borderRadius: BorderRadius.circular(24),
      ),
      padding: EdgeInsets.symmetric(vertical: isSmallScreen ? 20.0 : 28.0),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            'Total Balance',
            style: theme.textTheme.labelMedium?.copyWith(
              color: colorScheme.onPrimary.withOpacity(0.7),
              letterSpacing: 1.5,
              fontWeight: FontWeight.w500,
            ),
          ),
          SizedBox(height: isSmallScreen ? 12 : 16),
          FittedBox(
            fit: BoxFit.scaleDown,
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16.0),
              child: Text(
                displayBalance,
                style: TextStyle(
                  fontSize: isSmallScreen ? 40.0 : 52.0,
                  fontWeight: FontWeight.w300,
                  color: colorScheme.onPrimary,
                  letterSpacing: -1.5,
                  height: 1.0,
                ),
              ),
            ),
          ),
          SizedBox(height: isSmallScreen ? 12 : 16),
          Container(
            padding: EdgeInsets.symmetric(
              horizontal: isSmallScreen ? 12 : 16,
              vertical: isSmallScreen ? 4 : 6,
            ),
            decoration: BoxDecoration(
              color: colorScheme.onPrimary.withOpacity(0.12),
              borderRadius: BorderRadius.circular(20),
            ),
            child: Text(
              'KANARI',
              style: TextStyle(
                fontSize: isSmallScreen ? 10 : 11,
                color: colorScheme.onPrimary,
                fontWeight: FontWeight.w600,
                letterSpacing: 2,
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildCreateWalletCardPage(
    BuildContext context,
    ColorScheme colorScheme,
    ThemeData theme,
    bool isSmallScreen,
  ) {
    return Container(
      padding: EdgeInsets.all(isSmallScreen ? 16 : 24),
      decoration: BoxDecoration(
        color: colorScheme.surfaceVariant.withOpacity(0.3),
        borderRadius: BorderRadius.circular(32),
        border: Border.all(
          color: colorScheme.outlineVariant.withOpacity(0.5),
          width: 2,
        ),
      ),
      child: Column(
        mainAxisAlignment: MainAxisAlignment.center,
        children: [
          Container(
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              color: colorScheme.primaryContainer,
              shape: BoxShape.circle,
            ),
            child: Icon(
              Icons.add_rounded,
              size: 32,
              color: colorScheme.onPrimaryContainer,
            ),
          ),
          const SizedBox(height: 16),
          Text(
            'Create New Wallet',
            style: theme.textTheme.titleMedium?.copyWith(
              fontWeight: FontWeight.bold,
              color: colorScheme.onSurface,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            'Swipe left to manage existing wallets\nor tap below to add a new one.',
            textAlign: TextAlign.center,
            style: theme.textTheme.bodySmall?.copyWith(
              color: colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 24),
          FilledButton.tonal(
            onPressed: () => _showCreateDialog(context),
            child: const Text('Add Wallet'),
          ),
        ],
      ),
    );
  }

  Future<String?> _scanQRCode(BuildContext context) async {
    showDialog(
      context: context,
      builder: (dialogContext) => AlertDialog(
        icon: const Icon(Icons.qr_code_scanner_rounded, size: 32),
        title: const Text('QR Scanner'),
        content: const Text(
          'QR Scanner requires camera access.\n\nThis feature will be available soon.',
          textAlign: TextAlign.center,
        ),
        actions: [
          FilledButton(
            onPressed: () => Navigator.pop(dialogContext),
            child: const Text('Understood'),
          ),
        ],
      ),
    );
    return null;
  }

  void _showTransferDialog(BuildContext context, {String? prefilledAddress}) {
    final recipientController = TextEditingController(text: prefilledAddress);
    final amountController = TextEditingController();
    final theme = Theme.of(context);
    final isSmallScreen = MediaQuery.of(context).size.width < 360;

    showDialog(
      context: context,
      builder: (dialogContext) => AlertDialog(
        icon: const Icon(Icons.send_rounded),
        title: const Text('Transfer KANARI'),
        content: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(
                controller: recipientController,
                style: const TextStyle(fontFamily: 'monospace', fontSize: 13),
                decoration: InputDecoration(
                  labelText: 'Recipient Address',
                  hintText: '0x...',
                  helperText: 'Must be exactly 64 hex characters',
                  suffixIcon: IconButton(
                    icon: const Icon(Icons.qr_code_scanner_rounded),
                    onPressed: () async {
                      Navigator.pop(dialogContext);
                      final scannedAddress = await _scanQRCode(context);
                      if (scannedAddress != null && dialogContext.mounted) {
                        _showTransferDialog(
                          context,
                          prefilledAddress: scannedAddress,
                        );
                      }
                    },
                  ),
                ),
              ),
              SizedBox(height: isSmallScreen ? 12 : 16),
              TextField(
                controller: amountController,
                keyboardType: const TextInputType.numberWithOptions(
                  decimal: true,
                ),
                decoration: InputDecoration(
                  labelText: 'Amount (KANARI)',
                  prefixIcon: const Icon(Icons.account_balance_wallet_rounded),
                  suffixIcon: TextButton(
                    onPressed: () {
                      final balanceMist = context.read<WalletState>().balance;
                      final balanceKanari = balanceMist / 1000000000;
                      amountController.text = balanceKanari.toStringAsFixed(6);
                    },
                    child: const Text('MAX'),
                  ),
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
          FilledButton(
            onPressed: () async {
              final recipient = recipientController.text;
              final amountStr = amountController.text;
              final amountDouble = double.tryParse(amountStr) ?? 0.0;
              final amountMist = (amountDouble * 1000000000).round();

              if (recipient.isEmpty || amountMist <= 0) return;

              var cleanAddress = recipient.startsWith('0x')
                  ? recipient.substring(2)
                  : recipient;

              if (cleanAddress.length != 64 ||
                  !RegExp(r'^[0-9a-fA-F]+$').hasMatch(cleanAddress)) {
                ScaffoldMessenger.of(dialogContext).showSnackBar(
                  SnackBar(
                    content: const Text('Invalid address format.'),
                    backgroundColor: theme.colorScheme.error,
                  ),
                );
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
                        ? theme.colorScheme.error
                        : theme.colorScheme.primary,
                  ),
                );
              }
            },
            child: const Text('Send KANARI'),
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
        icon: const Icon(Icons.password_rounded),
        title: const Text('Change Password'),
        content: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(
                controller: oldPasswordController,
                obscureText: true,
                decoration: const InputDecoration(labelText: 'Old Password'),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: newPasswordController,
                obscureText: true,
                decoration: const InputDecoration(labelText: 'New Password'),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: confirmPasswordController,
                obscureText: true,
                decoration: const InputDecoration(
                  labelText: 'Confirm New Password',
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
          FilledButton(
            onPressed: () async {
              final oldPassword = oldPasswordController.text;
              final newPassword = newPasswordController.text;
              final confirmPassword = confirmPasswordController.text;

              if (oldPassword.isEmpty ||
                  newPassword.isEmpty ||
                  newPassword != confirmPassword) {
                ScaffoldMessenger.of(dialogContext).showSnackBar(
                  SnackBar(
                    content: const Text('Invalid or mismatched passwords'),
                    backgroundColor: Theme.of(dialogContext).colorScheme.error,
                  ),
                );
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
                          ? 'Password changed'
                          : 'Failed to change password',
                    ),
                    backgroundColor: success
                        ? Theme.of(context).colorScheme.primary
                        : Theme.of(context).colorScheme.error,
                  ),
                );
              }
            },
            child: const Text('Update'),
          ),
        ],
      ),
    );
  }

  void _showCreateDialog(BuildContext context) {
    KanariCurve selectedCurve = KanariCurve.ed25519;

    showDialog(
      context: context,
      builder: (dialogContext) => StatefulBuilder(
        builder: (context, setState) => AlertDialog(
          title: const Text('Create New Wallet'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Text(
                'Select the cryptographic curve for your new wallet.',
                style: TextStyle(fontSize: 13, color: Colors.grey),
              ),
              const SizedBox(height: 16),
              DropdownButtonFormField<KanariCurve>(
                value: selectedCurve,
                isExpanded: true,
                decoration: const InputDecoration(
                  labelText: 'Curve Type',
                  border: OutlineInputBorder(),
                ),
                items: KanariCurve.values.map((curve) {
                  return DropdownMenuItem(
                    value: curve,
                    child: Text(curve.name, overflow: TextOverflow.ellipsis),
                  );
                }).toList(),
                onChanged: (val) => setState(() => selectedCurve = val!),
              ),
            ],
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(dialogContext),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () async {
                await context.read<WalletState>().createNewWallet(
                  curve: selectedCurve,
                  password: '',
                );

                if (context.mounted) {
                  Navigator.pop(dialogContext);

                  Future.delayed(const Duration(milliseconds: 300), () {
                    if (_pageController.hasClients) {
                      _pageController.animateToPage(
                        context.read<WalletState>().wallets.length - 1,
                        duration: const Duration(milliseconds: 400),
                        curve: Curves.easeOutCubic,
                      );
                    }
                  });
                }
              },
              child: const Text('Generate'),
            ),
          ],
        ),
      ),
    );
  }
}
