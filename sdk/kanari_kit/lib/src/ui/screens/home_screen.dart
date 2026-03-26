import 'dart:async';
import 'dart:ui';
import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_spinkit/flutter_spinkit.dart';
import 'package:provider/provider.dart';

// 👇 เพิ่ม Import สำหรับระบบสแกน QR Code แล้ว
import 'package:mobile_scanner/mobile_scanner.dart';

import 'package:kanari_kit/src/providers/wallet_provider.dart';
import 'package:kanari_kit/kanari_kit.dart';
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
  late ScrollController _scrollController;
  Timer? _debounce;
  bool _showBottomBar = true;

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

    _scrollController = ScrollController();
    _scrollController.addListener(() {
      if (_scrollController.position.userScrollDirection ==
          ScrollDirection.reverse) {
        if (_showBottomBar) {
          setState(() => _showBottomBar = false);
        }
      } else if (_scrollController.position.userScrollDirection ==
          ScrollDirection.forward) {
        if (!_showBottomBar) {
          setState(() => _showBottomBar = true);
        }
      }
    });
  }

  @override
  void dispose() {
    _pageController.dispose();
    _scrollController.dispose();
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
          : Stack(
              children: [
                RefreshIndicator(
                  onRefresh: () => state.refreshBalance(),
                  backgroundColor: colorScheme.surface,
                  color: colorScheme.primary,
                  child: CustomScrollView(
                    controller: _scrollController,
                    slivers: [
                      SliverAppBar(
                        floating: true,
                        snap: true,
                        pinned: false,
                        backgroundColor: Colors.transparent,
                        surfaceTintColor: Colors.transparent,
                        elevation: 0,
                        toolbarHeight: 88,
                        flexibleSpace: SafeArea(
                          child: Container(
                            margin: EdgeInsets.symmetric(
                              horizontal: screenPadding,
                              vertical: 12,
                            ),
                            padding: const EdgeInsets.symmetric(
                              horizontal: 12,
                              vertical: 8,
                            ),
                            decoration: BoxDecoration(
                              color: colorScheme.surfaceContainerHigh
                                  .withOpacity(0.6),
                              borderRadius: BorderRadius.circular(24),
                              border: Border.all(
                                color: colorScheme.outline.withOpacity(0.1),
                              ),
                            ),
                            child: Row(
                              mainAxisAlignment: MainAxisAlignment.spaceBetween,
                              children: [
                                Row(
                                  children: [
                                    Container(
                                      padding: const EdgeInsets.all(10),
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
                                      style: theme.textTheme.titleLarge
                                          ?.copyWith(
                                            fontWeight: FontWeight.w800,
                                            letterSpacing: -0.5,
                                            color: colorScheme.onSurface,
                                          ),
                                    ),
                                  ],
                                ),
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
                                        if (value == 'change_pin') {
                                          _showChangePinDialog(context, state);
                                        } else if (value == 'logout') {
                                          state.logout();
                                        }
                                      },
                                      itemBuilder: (context) => [
                                        PopupMenuItem(
                                          value: 'change_pin',
                                          child: ListTile(
                                            leading: Icon(
                                              Icons.pin_rounded,
                                              color: colorScheme.primary,
                                            ),
                                            title: const Text('Change PIN'),
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
                                      final targetWalletId =
                                          wallets[index]['id'];
                                      if (context
                                              .read<WalletState>()
                                              .activeWalletId !=
                                          targetWalletId) {
                                        context
                                            .read<WalletState>()
                                            .switchWallet(targetWalletId);
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

                      SliverToBoxAdapter(
                        child: Padding(
                          padding: EdgeInsets.fromLTRB(
                            screenPadding,
                            16,
                            screenPadding,
                            0,
                          ),
                          child: _buildAssetsSection(
                            context,
                            theme,
                            colorScheme,
                            isSmallScreen,
                          ),
                        ),
                      ),

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

                      const SliverToBoxAdapter(child: SizedBox(height: 120)),
                    ],
                  ),
                ),

                Positioned(
                  bottom: 0,
                  left: 0,
                  right: 0,
                  child: AnimatedSlide(
                    duration: const Duration(milliseconds: 300),
                    offset: _showBottomBar ? Offset.zero : const Offset(0, 1.5),
                    curve: Curves.easeOutCubic,
                    child: SafeArea(
                      child: Padding(
                        padding: EdgeInsets.fromLTRB(
                          screenPadding,
                          0,
                          screenPadding,
                          16,
                        ),
                        child: _buildFloatingActionBar(
                          context,
                          theme,
                          colorScheme,
                          isSmallScreen,
                        ),
                      ),
                    ),
                  ),
                ),
              ],
            ),
    );
  }

  Widget _buildAssetsSection(
    BuildContext context,
    ThemeData theme,
    ColorScheme colorScheme,
    bool isSmallScreen,
  ) {
    final state = context.watch<WalletState>();
    final tokens = state.tokenBalances;

    if (tokens.isEmpty) {
      return const SizedBox.shrink();
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.only(left: 4, bottom: 12),
          child: Text(
            'Your Assets',
            style: theme.textTheme.labelSmall?.copyWith(
              fontWeight: FontWeight.w600,
              color: theme.colorScheme.onSurface.withOpacity(0.4),
              letterSpacing: 1.5,
            ),
          ),
        ),
        Container(
          decoration: BoxDecoration(
            color: colorScheme.surfaceVariant.withOpacity(0.15),
            borderRadius: BorderRadius.circular(24),
            border: Border.all(
              color: colorScheme.outline.withOpacity(0.06),
              width: 1,
            ),
          ),
          child: ListView.separated(
            shrinkWrap: true,
            physics: const NeverScrollableScrollPhysics(),
            itemCount: tokens.length,
            separatorBuilder: (context, index) => Divider(
              height: 1,
              indent: 16,
              endIndent: 16,
              color: colorScheme.outline.withOpacity(0.05),
            ),
            itemBuilder: (context, index) {
              final token = tokens[index];
              final formattedAmount =
                  token.amount / math.pow(10, token.decimals);
              final isKanari = token.tokenType == 'KANARI';

              final iconColor = isKanari ? colorScheme.primary : Colors.teal;
              final iconData = isKanari
                  ? Icons.hexagon_rounded
                  : Icons.toll_rounded;

              return Padding(
                padding: EdgeInsets.symmetric(
                  horizontal: isSmallScreen ? 16 : 20,
                  vertical: isSmallScreen ? 12 : 16,
                ),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.center,
                  children: [
                    Container(
                      padding: const EdgeInsets.all(10),
                      decoration: BoxDecoration(
                        color: iconColor.withOpacity(0.1),
                        shape: BoxShape.circle,
                      ),
                      child: Icon(iconData, color: iconColor, size: 20),
                    ),
                    SizedBox(width: isSmallScreen ? 12 : 16),
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          Text(
                            token.symbol,
                            style: theme.textTheme.titleMedium?.copyWith(
                              fontWeight: FontWeight.bold,
                              height: 1.1,
                            ),
                          ),
                          if (!isKanari) ...[
                            const SizedBox(height: 4),
                            Text(
                              token.tokenType,
                              style: TextStyle(
                                fontSize: 10,
                                color: colorScheme.onSurface.withOpacity(0.4),
                                height: 1.1,
                              ),
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                            ),
                          ],
                        ],
                      ),
                    ),
                    const SizedBox(width: 8),
                    Text(
                      formattedAmount.toStringAsFixed(4),
                      style: theme.textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ],
                ),
              );
            },
          ),
        ),
      ],
    );
  }

  Widget _buildFloatingActionBar(
    BuildContext context,
    ThemeData theme,
    ColorScheme colorScheme,
    bool isSmallScreen,
  ) {
    return Container(
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHigh.withOpacity(0.9),
        borderRadius: BorderRadius.circular(28),
        border: Border.all(color: colorScheme.outline.withOpacity(0.1)),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withOpacity(0.15),
            blurRadius: 20,
            offset: const Offset(0, 8),
          ),
        ],
      ),
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          borderRadius: BorderRadius.circular(28),
          onTap: () => _showTransferDialog(context),
          child: Padding(
            padding: EdgeInsets.symmetric(
              horizontal: isSmallScreen ? 16 : 20,
              vertical: isSmallScreen ? 12 : 16,
            ),
            child: Row(
              children: [
                Container(
                  padding: EdgeInsets.all(isSmallScreen ? 10 : 14),
                  decoration: BoxDecoration(
                    color: colorScheme.primary,
                    borderRadius: BorderRadius.circular(18),
                  ),
                  child: Icon(
                    Icons.send_rounded,
                    color: colorScheme.onPrimary,
                    size: isSmallScreen ? 20 : 24,
                  ),
                ),
                SizedBox(width: isSmallScreen ? 12 : 16),
                Expanded(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'Send / Transfer',
                        style: theme.textTheme.titleMedium?.copyWith(
                          fontWeight: FontWeight.bold,
                          color: colorScheme.onSurface,
                        ),
                      ),
                      Text(
                        'Transfer assets to another address',
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ],
                  ),
                ),
                Icon(
                  Icons.chevron_right_rounded,
                  color: colorScheme.onSurfaceVariant,
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildWalletCardPage(
    BuildContext context,
    ColorScheme colorScheme,
    Map<String, dynamic> walletData,
    ThemeData theme,
    bool isSmallScreen,
  ) {
    return Container(
      padding: EdgeInsets.symmetric(
        horizontal: isSmallScreen ? 16 : 24,
        vertical: isSmallScreen ? 16 : 24,
      ),
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

          Expanded(
            child: Center(
              child: SingleChildScrollView(
                physics: const BouncingScrollPhysics(),
                child: Padding(
                  padding: const EdgeInsets.symmetric(vertical: 8.0),
                  child: _buildIndexSpecificBalanceSection(
                    context,
                    colorScheme,
                    walletData,
                    isSmallScreen,
                    theme,
                  ),
                ),
              ),
            ),
          ),

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
      padding: EdgeInsets.symmetric(vertical: isSmallScreen ? 16.0 : 28.0),
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
          SizedBox(height: isSmallScreen ? 8 : 16),
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
          SizedBox(height: isSmallScreen ? 8 : 16),
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
      child: Center(
        child: SingleChildScrollView(
          physics: const BouncingScrollPhysics(),
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
        ),
      ),
    );
  }

  void _showTransferDialog(BuildContext context, {String? prefilledAddress}) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      showDragHandle: true,
      backgroundColor: Theme.of(context).colorScheme.surface,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(32)),
      ),
      builder: (context) =>
          _TransferBottomSheet(prefilledAddress: prefilledAddress),
    );
  }

  void _showChangePinDialog(BuildContext context, WalletState state) {
    final oldPinController = TextEditingController();
    final newPinController = TextEditingController();
    final confirmPinController = TextEditingController();

    showDialog(
      context: context,
      builder: (dialogContext) => AlertDialog(
        icon: const Icon(Icons.pin_rounded),
        title: const Text('Change PIN'),
        content: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextField(
                controller: oldPinController,
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
                decoration: const InputDecoration(
                  labelText: 'Old PIN',
                  counterText: '',
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: newPinController,
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
                decoration: const InputDecoration(
                  labelText: 'New PIN',
                  counterText: '',
                ),
              ),
              const SizedBox(height: 12),
              TextField(
                controller: confirmPinController,
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
                decoration: const InputDecoration(
                  labelText: 'Confirm New PIN',
                  counterText: '',
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
              final oldPin = oldPinController.text;
              final newPin = newPinController.text;
              final confirmPin = confirmPinController.text;

              if (oldPin.length != 6 ||
                  newPin.length != 6 ||
                  newPin != confirmPin) {
                ScaffoldMessenger.of(dialogContext).showSnackBar(
                  SnackBar(
                    content: const Text('Invalid PIN or PINs do not match'),
                    backgroundColor: Theme.of(dialogContext).colorScheme.error,
                  ),
                );
                return;
              }

              Navigator.pop(dialogContext);
              final success = await state.changePin(oldPin, newPin);

              if (context.mounted) {
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(
                    content: Text(
                      success
                          ? 'PIN changed successfully'
                          : 'Failed to change PIN (Old PIN incorrect?)',
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
                  pin: '',
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

// ============================================================================
// โค้ดส่วน UI ของ Bottom Sheet สำหรับโอนเงิน
// ============================================================================
class _TransferBottomSheet extends StatefulWidget {
  final String? prefilledAddress;
  const _TransferBottomSheet({this.prefilledAddress});

  @override
  State<_TransferBottomSheet> createState() => _TransferBottomSheetState();
}

class _TransferBottomSheetState extends State<_TransferBottomSheet> {
  late TextEditingController _recipientController;
  late TextEditingController _amountController;
  String _selectedTokenType = '';

  @override
  void initState() {
    super.initState();
    _recipientController = TextEditingController(text: widget.prefilledAddress);
    _amountController = TextEditingController();
  }

  @override
  void dispose() {
    _recipientController.dispose();
    _amountController.dispose();
    super.dispose();
  }

  // 👇 แก้ไขฟังก์ชันนี้ให้เปิดหน้ากล้อง
  Future<String?> _scanQRCode(BuildContext context) async {
    final result = await Navigator.push<String>(
      context,
      MaterialPageRoute(builder: (context) => const QRScannerScreen()),
    );
    return result;
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final isSmallScreen = MediaQuery.of(context).size.width < 360;
    final walletState = context.watch<WalletState>();

    List<DropdownMenuItem<String>> tokenItems = [
      DropdownMenuItem(
        value: '',
        child: Text(
          'KANARI (${(walletState.balance / 1000000000).toStringAsFixed(4)})',
          style: const TextStyle(fontWeight: FontWeight.bold),
        ),
      ),
    ];

    for (var token in walletState.tokenBalances) {
      if (token.tokenType == 'KANARI') continue;
      final formattedAmount = token.amount / math.pow(10, token.decimals);
      tokenItems.add(
        DropdownMenuItem(
          value: token.tokenType,
          child: Text(
            '${token.symbol} (${formattedAmount.toStringAsFixed(4)})',
          ),
        ),
      );
    }

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
            'Transfer Assets',
            style: theme.textTheme.headlineSmall?.copyWith(
              fontWeight: FontWeight.bold,
              color: colorScheme.onSurface,
            ),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 24),

          TextField(
            controller: _recipientController,
            style: const TextStyle(fontFamily: 'monospace', fontSize: 13),
            decoration: InputDecoration(
              labelText: 'Recipient Address',
              hintText: '0x...',
              helperText: 'Must be exactly 64 hex characters',
              suffixIcon: IconButton(
                icon: const Icon(Icons.qr_code_scanner_rounded),
                onPressed: () async {
                  final scannedAddress = await _scanQRCode(context);
                  if (scannedAddress != null) {
                    setState(() {
                      _recipientController.text = scannedAddress;
                    });
                  }
                },
              ),
            ),
          ),
          SizedBox(height: isSmallScreen ? 12 : 16),

          DropdownButtonFormField<String>(
            initialValue: _selectedTokenType,
            isExpanded: true,
            decoration: const InputDecoration(
              labelText: 'Asset to send',
              prefixIcon: Icon(Icons.toll_rounded),
            ),
            items: tokenItems,
            onChanged: (val) {
              setState(() {
                _selectedTokenType = val!;
                _amountController.clear();
              });
            },
          ),
          SizedBox(height: isSmallScreen ? 12 : 16),

          TextField(
            controller: _amountController,
            keyboardType: const TextInputType.numberWithOptions(decimal: true),
            decoration: InputDecoration(
              labelText: 'Amount',
              prefixIcon: const Icon(Icons.account_balance_wallet_rounded),
              suffixIcon: TextButton(
                onPressed: () {
                  if (_selectedTokenType == '') {
                    final balanceKanari = walletState.balance / 1000000000;
                    _amountController.text = balanceKanari.toStringAsFixed(6);
                  } else {
                    final selectedToken = walletState.tokenBalances.firstWhere(
                      (t) => t.tokenType == _selectedTokenType,
                    );
                    final maxAmount =
                        selectedToken.amount /
                        math.pow(10, selectedToken.decimals);
                    _amountController.text = maxAmount.toStringAsFixed(
                      selectedToken.decimals < 6 ? selectedToken.decimals : 6,
                    );
                  }
                },
                child: const Text('MAX'),
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
            onPressed: () async {
              final recipient = _recipientController.text;
              final amountStr = _amountController.text;
              final amountDouble = double.tryParse(amountStr) ?? 0.0;

              if (recipient.isEmpty || amountDouble <= 0) return;

              var cleanAddress = recipient.startsWith('0x')
                  ? recipient.substring(2)
                  : recipient;

              if (cleanAddress.length != 64 ||
                  !RegExp(r'^[0-9a-fA-F]+$').hasMatch(cleanAddress)) {
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(
                    content: const Text('Invalid address format.'),
                    backgroundColor: colorScheme.error,
                  ),
                );
                return;
              }

              Navigator.pop(context);

              String? result;
              final ws = context.read<WalletState>();

              if (_selectedTokenType == '') {
                final amountMist = (amountDouble * 1000000000).round();
                result = await ws.transfer(recipient, amountMist);
              } else {
                final selectedToken = ws.tokenBalances.firstWhere(
                  (t) => t.tokenType == _selectedTokenType,
                );
                final amountBaseUnits =
                    (amountDouble * math.pow(10, selectedToken.decimals))
                        .round();
                result = await ws.transferToken(
                  recipient,
                  _selectedTokenType,
                  amountBaseUnits,
                );
              }

              if (context.mounted) {
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(
                    content: Text(
                      result?.startsWith('Error:') == true
                          ? result!
                          : 'Transaction successful',
                    ),
                    backgroundColor: result?.startsWith('Error:') == true
                        ? colorScheme.error
                        : colorScheme.primary,
                  ),
                );
              }
            },
            child: const Text(
              'Send Assets',
              style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
            ),
          ),
        ],
      ),
    );
  }
}

// ============================================================================
// 👉 UI: หน้าต่างสแกน QR Code พร้อมกล้อง (สำหรับ Mobile Scanner v7+)
// ============================================================================
class QRScannerScreen extends StatefulWidget {
  const QRScannerScreen({super.key});

  @override
  State<QRScannerScreen> createState() => _QRScannerScreenState();
}

class _QRScannerScreenState extends State<QRScannerScreen> {
  late MobileScannerController cameraController;
  bool isProcessing = false;

  @override
  void initState() {
    super.initState();
    cameraController = MobileScannerController();
  }

  @override
  void dispose() {
    cameraController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Scaffold(
      backgroundColor: Colors.black,
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        foregroundColor: Colors.white,
        title: const Text('Scan QR Code'),
        actions: [
          IconButton(
            icon: const Icon(Icons.flashlight_on_rounded, color: Colors.yellow),
            onPressed: () => cameraController.toggleTorch(),
          ),
          IconButton(
            icon: const Icon(
              Icons.flip_camera_android_rounded,
              color: Colors.white,
            ),
            onPressed: () => cameraController.switchCamera(),
          ),
        ],
      ),
      body: Stack(
        children: [
          MobileScanner(
            controller: cameraController,
            onDetect: (capture) {
              if (isProcessing) return;

              final List<Barcode> barcodes = capture.barcodes;
              for (final barcode in barcodes) {
                if (barcode.rawValue != null) {
                  setState(() => isProcessing = true);
                  final String code = barcode.rawValue!;
                  Navigator.pop(context, code);
                  break;
                }
              }
            },
          ),
          Center(
            child: Container(
              width: 250,
              height: 250,
              decoration: BoxDecoration(
                border: Border.all(color: colorScheme.primary, width: 3),
                borderRadius: BorderRadius.circular(24),
              ),
            ),
          ),
          Positioned(
            bottom: 48,
            left: 0,
            right: 0,
            child: Center(
              child: Container(
                padding: const EdgeInsets.symmetric(
                  horizontal: 16,
                  vertical: 8,
                ),
                decoration: BoxDecoration(
                  color: Colors.black54,
                  borderRadius: BorderRadius.circular(20),
                ),
                child: const Text(
                  'Align QR code within the frame',
                  style: TextStyle(color: Colors.white),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
