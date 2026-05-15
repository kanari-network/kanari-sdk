import 'dart:async';
import 'dart:ui';
import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_spinkit/flutter_spinkit.dart';
import 'package:kanari_pay/src/ui/widgets/token_logo.dart';
import 'package:provider/provider.dart';

import 'package:kanari_pay/src/providers/wallet_provider.dart';
import 'package:kanari_pay/kanari_pay.dart';
import '../widgets/security_card.dart';
import '../network_selector.dart';
import '../wallet_info_card.dart';

class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key});

  @override
  HomeScreenState createState() => HomeScreenState();
}

class HomeScreenState extends State<HomeScreen> {
  late PageController _pageController;
  late ScrollController _scrollController;
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

    _scrollController = ScrollController();
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
                              color: colorScheme.surface,
                              borderRadius: BorderRadius.circular(16),
                              border: Border.all(
                                color: colorScheme.outline.withOpacity(0.2),
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
                                        color:
                                            colorScheme.surfaceContainerHighest,
                                        borderRadius: BorderRadius.circular(12),
                                        border: Border.all(
                                          color: colorScheme.outline
                                              .withOpacity(0.2),
                                        ),
                                      ),
                                      child: Icon(
                                        Icons.hexagon_rounded,
                                        color: colorScheme.onSurface,
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
                                    IconButton(
                                      onPressed: () {
                                        Navigator.of(
                                          context,
                                        ).pushNamed('/settings');
                                      },
                                      icon: Icon(
                                        Icons.settings_rounded,
                                        color: colorScheme.onSurfaceVariant,
                                      ),
                                      tooltip: 'Settings',
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

                      const SliverToBoxAdapter(child: SizedBox(height: 100)),
                    ],
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
            color: colorScheme.surface,
            borderRadius: BorderRadius.circular(16),
            border: Border.all(color: colorScheme.outline.withOpacity(0.2)),
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

              return Padding(
                padding: EdgeInsets.symmetric(
                  horizontal: isSmallScreen ? 16 : 20,
                  vertical: isSmallScreen ? 12 : 16,
                ),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.center,
                  children: [
                    TokenLogo(
                      tokenType: token.tokenType,
                      symbol: token.symbol,
                      size: isSmallScreen ? 40 : 44,
                      logoUrl: token.iconUrl,
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
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(
                child: Text(
                  walletData['name'] ?? 'Wallet',
                  style: theme.textTheme.titleSmall?.copyWith(
                    color: colorScheme.onPrimary.withOpacity(0.8),
                    fontWeight: FontWeight.bold,
                    letterSpacing: 0.5,
                  ),
                ),
              ),
              IconButton(
                onPressed: () => _confirmDeleteWallet(
                  context,
                  walletData['id'] as String,
                  walletData['name'] ?? 'Wallet',
                ),
                icon: Icon(
                  Icons.delete_outline_rounded,
                  color: colorScheme.onPrimary.withOpacity(0.85),
                ),
                tooltip: 'Delete Wallet',
              ),
            ],
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
                  color: colorScheme.surfaceContainerHighest,
                  shape: BoxShape.circle,
                  border: Border.all(
                    color: colorScheme.outline.withOpacity(0.3),
                  ),
                ),
                child: Icon(
                  Icons.add_rounded,
                  size: 32,
                  color: colorScheme.onSurface,
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

  Future<void> _confirmDeleteWallet(
    BuildContext context,
    String walletId,
    String walletName,
  ) async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('Delete Wallet'),
        content: Text(
          'Are you sure you want to delete "$walletName"? This cannot be undone.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.of(dialogContext).pop(true),
            style: FilledButton.styleFrom(
              backgroundColor: Theme.of(context).colorScheme.error,
              foregroundColor: Theme.of(context).colorScheme.onError,
            ),
            child: const Text('Delete'),
          ),
        ],
      ),
    );

    if (confirmed != true) return;

    await context.read<WalletState>().removeWallet(walletId);

    if (_pageController.hasClients) {
      final currentPage =
          _pageController.page?.round() ?? _pageController.initialPage;
      final remainingWallets = context.read<WalletState>().wallets.length;
      final targetPage = remainingWallets > 0
          ? currentPage.clamp(0, remainingWallets - 1)
          : 0;

      if (targetPage != currentPage) {
        _pageController.jumpToPage(targetPage);
      }
    }
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
