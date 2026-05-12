import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_spinkit/flutter_spinkit.dart';
import 'package:provider/provider.dart';

import '../../client/kanari_client.dart';
import '../../models/account.dart';
import '../../models/transaction.dart';
import '../../providers/wallet_provider.dart';

class DEXScreen extends StatefulWidget {
  const DEXScreen({super.key});

  @override
  State<DEXScreen> createState() => _DEXScreenState();
}

class _DEXScreenState extends State<DEXScreen>
    with SingleTickerProviderStateMixin {
  late final TabController _tabController;
  late ScrollController _scrollController;

  // Controllers for Create Pool
  String? _selectedCoinTypeA;
  String? _selectedCoinTypeB;
  final _feePercentController = TextEditingController(
    text: '30',
  ); // 0.3% default

  List<String> _availableTokens = [];
  bool _isLoadingTokens = false;

  // Controllers for Add/Remove Liquidity
  final _amountAController = TextEditingController();
  final _amountBController = TextEditingController();
  final _lpAmountController = TextEditingController();

  // Controllers for Swap
  final _swapAmountInController = TextEditingController();
  String _swapDirection = 'AtoB'; // AtoB or BtoA

  bool _isLoading = false;
  String? _errorMessage;
  String? _successMessage;

  // Selected pool for operations
  Map<String, dynamic>? _selectedPool;

  // User's pools list
  List<Map<String, dynamic>> _userPools = [];
  bool _isLoadingPools = false;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 3, vsync: this);
    _scrollController = ScrollController();
    _loadUserPools();
    _loadAvailableTokens();
  }

  Future<void> _loadAvailableTokens() async {
    setState(() => _isLoadingTokens = true);

    try {
      final walletState = Provider.of<WalletState>(context, listen: false);
      if (walletState.wallet != null) {
        // Get tokens from wallet balances
        final tokens = walletState.tokenBalances
            .map((token) => token.tokenType)
            .where((type) => type.isNotEmpty)
            .toList();

        if (mounted) {
          setState(() {
            _availableTokens = tokens;
            if (tokens.length >= 2) {
              _selectedCoinTypeA = tokens[0];
              _selectedCoinTypeB = tokens[1];
            } else if (tokens.isNotEmpty) {
              _selectedCoinTypeA = tokens[0];
            }
          });
        }
      }
    } catch (e) {
      debugPrint('Failed to load tokens: $e');
    } finally {
      if (mounted) {
        setState(() => _isLoadingTokens = false);
      }
    }
  }

  Future<void> _loadUserPools() async {
    setState(() => _isLoadingPools = true);

    try {
      final walletState = Provider.of<WalletState>(context, listen: false);
      if (walletState.wallet != null) {
        final address = walletState.wallet!.address;
        final client = KanariClient(walletState.environment.rpcUrl);
        final pools = await client.getUserPools(address);

        // Load available tokens from wallet balance
        final tokens = walletState.tokenBalances
            .map((token) => token.tokenType)
            .where((type) => type.isNotEmpty)
            .toList();

        if (mounted) {
          setState(() {
            _userPools = pools;
            if (pools.isNotEmpty) {
              _selectedPool = pools.first;
            }
            // Update available tokens list
            _availableTokens = tokens;
            // Auto-select first two tokens if available
            if (_availableTokens.length >= 2 && _selectedCoinTypeA == null) {
              _selectedCoinTypeA = _availableTokens[0];
              _selectedCoinTypeB = _availableTokens[1];
            }
          });
        }
      }
    } catch (e) {
      debugPrint('Failed to load pools: $e');
    } finally {
      if (mounted) {
        setState(() => _isLoadingPools = false);
      }
    }
  }

  @override
  void dispose() {
    _tabController.dispose();
    _scrollController.dispose();
    _feePercentController.dispose();
    _amountAController.dispose();
    _amountBController.dispose();
    _lpAmountController.dispose();
    _swapAmountInController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Scaffold(
      backgroundColor: colorScheme.surface,
      body: _isLoadingPools
          ? Center(child: SpinKitFadingCircle(color: colorScheme.primary))
          : RefreshIndicator(
              onRefresh: _loadUserPools,
              backgroundColor: colorScheme.surface,
              color: colorScheme.primary,
              child: CustomScrollView(
                controller: _scrollController,
                slivers: [
                  // TabBar in Sliver
                  SliverToBoxAdapter(
                    child: Container(
                      margin: const EdgeInsets.symmetric(
                        horizontal: 16,
                        vertical: 8,
                      ),
                      decoration: BoxDecoration(
                        color: colorScheme.surfaceContainerHigh,
                        borderRadius: BorderRadius.circular(12),
                      ),
                      child: TabBar(
                        controller: _tabController,
                        labelColor: colorScheme.primary,
                        unselectedLabelColor: colorScheme.onSurfaceVariant,
                        indicatorColor: colorScheme.primary,
                        indicatorWeight: 3,
                        dividerColor: Colors.transparent,
                        tabs: const [
                          Tab(icon: Icon(Icons.swap_horiz), text: 'Swap'),
                          Tab(icon: Icon(Icons.add), text: 'Liquidity'),
                          Tab(icon: Icon(Icons.pool), text: 'Create Pool'),
                        ],
                      ),
                    ),
                  ),

                  // Tab Content
                  SliverFillRemaining(
                    hasScrollBody: true,
                    child: TabBarView(
                      controller: _tabController,
                      children: [
                        _buildSwapTab(),
                        _buildLiquidityTab(),
                        _buildCreatePoolTab(),
                      ],
                    ),
                  ),
                ],
              ),
            ),
    );
  }

  // ==================== CREATE POOL TAB ====================
  Widget _buildCreatePoolTab() {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final screenPadding = MediaQuery.of(context).size.width < 360 ? 12.0 : 16.0;

    return SingleChildScrollView(
      padding: EdgeInsets.all(screenPadding),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Container(
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              color: colorScheme.surfaceContainerHigh,
              borderRadius: BorderRadius.circular(16),
              border: Border.all(color: colorScheme.outline.withOpacity(0.2)),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Icon(
                      Icons.add_circle_outline,
                      color: colorScheme.primary,
                      size: 24,
                    ),
                    const SizedBox(width: 8),
                    Text(
                      'Create New Pool',
                      style: TextStyle(
                        fontSize: 20,
                        fontWeight: FontWeight.bold,
                        color: colorScheme.onSurface,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 16),

                // Coin Type A Selector
                if (_isLoadingTokens)
                  const Center(child: CircularProgressIndicator())
                else if (_availableTokens.isEmpty)
                  Container(
                    padding: const EdgeInsets.all(16),
                    decoration: BoxDecoration(
                      color: colorScheme.errorContainer.withOpacity(0.1),
                      borderRadius: BorderRadius.circular(12),
                      border: Border.all(color: colorScheme.errorContainer),
                    ),
                    child: Row(
                      children: [
                        Icon(Icons.info_outline, color: colorScheme.error),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            'No tokens found in wallet. Add some tokens first.',
                            style: TextStyle(
                              color: colorScheme.onErrorContainer,
                            ),
                          ),
                        ),
                      ],
                    ),
                  )
                else ...[
                  DropdownButtonFormField<String>(
                    value: _selectedCoinTypeA,
                    decoration: InputDecoration(
                      labelText: 'Token A',
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(12),
                      ),
                      filled: true,
                      fillColor: colorScheme.surface,
                      prefixIcon: Icon(Icons.token, color: colorScheme.primary),
                    ),
                    items: _availableTokens.map((tokenType) {
                      final token = context
                          .read<WalletState>()
                          .tokenBalances
                          .firstWhere(
                            (t) => t.tokenType == tokenType,
                            orElse: () => TokenBalance(
                              tokenType: tokenType,
                              symbol: tokenType.split('::').last,
                              amount: 0,
                              decimals: 6,
                            ),
                          );
                      return DropdownMenuItem(
                        value: tokenType,
                        child: Text('${token.symbol} (${token.amount})'),
                      );
                    }).toList(),
                    onChanged: (value) {
                      setState(() => _selectedCoinTypeA = value);
                    },
                  ),

                  const SizedBox(height: 12),

                  // Coin Type B Selector
                  DropdownButtonFormField<String>(
                    value: _selectedCoinTypeB,
                    decoration: InputDecoration(
                      labelText: 'Token B',
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(12),
                      ),
                      filled: true,
                      fillColor: colorScheme.surface,
                      prefixIcon: Icon(Icons.token, color: colorScheme.primary),
                    ),
                    items: _availableTokens.map((tokenType) {
                      final token = context
                          .read<WalletState>()
                          .tokenBalances
                          .firstWhere(
                            (t) => t.tokenType == tokenType,
                            orElse: () => TokenBalance(
                              tokenType: tokenType,
                              symbol: tokenType.split('::').last,
                              amount: 0,
                              decimals: 6,
                            ),
                          );
                      return DropdownMenuItem(
                        value: tokenType,
                        child: Text('${token.symbol} (${token.amount})'),
                      );
                    }).toList(),
                    onChanged: (value) {
                      setState(() => _selectedCoinTypeB = value);
                    },
                  ),
                ],

                const SizedBox(height: 12),

                // Fee Percent Input
                TextField(
                  controller: _feePercentController,
                  keyboardType: TextInputType.number,
                  decoration: InputDecoration(
                    labelText: 'Fee Percent (basis points, e.g., 30 = 0.3%)',
                    border: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(12),
                    ),
                    filled: true,
                    fillColor: colorScheme.surface,
                    prefixIcon: Icon(Icons.percent, color: colorScheme.primary),
                  ),
                ),
                const SizedBox(height: 16),

                // Create Pool Button
                ElevatedButton.icon(
                  onPressed:
                      (_isLoading ||
                          _selectedCoinTypeA == null ||
                          _selectedCoinTypeB == null)
                      ? null
                      : _createPool,
                  icon: Icon(_isLoading ? Icons.hourglass_empty : Icons.pool),
                  label: Text(_isLoading ? 'Creating...' : 'Create Pool'),
                  style: ElevatedButton.styleFrom(
                    minimumSize: const Size.fromHeight(48),
                    backgroundColor: colorScheme.primary,
                    foregroundColor: colorScheme.onPrimary,
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(12),
                    ),
                  ),
                ),
              ],
            ),
          ),

          // Error/Success Messages
          if (_errorMessage != null) ...[
            const SizedBox(height: 12),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: colorScheme.errorContainer.withOpacity(0.1),
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: colorScheme.errorContainer),
              ),
              child: Row(
                children: [
                  Icon(Icons.error_outline, color: colorScheme.error),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      _errorMessage!,
                      style: TextStyle(color: colorScheme.onErrorContainer),
                    ),
                  ),
                ],
              ),
            ),
          ],
          if (_successMessage != null) ...[
            const SizedBox(height: 12),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: colorScheme.primaryContainer.withOpacity(0.1),
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: colorScheme.primaryContainer),
              ),
              child: Row(
                children: [
                  Icon(Icons.check_circle_outline, color: colorScheme.primary),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      _successMessage!,
                      style: TextStyle(color: colorScheme.onPrimaryContainer),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ],
      ),
    );
  }

  Future<void> _createPool() async {
    setState(() {
      _isLoading = true;
      _errorMessage = null;
      _successMessage = null;
    });

    try {
      final walletState = Provider.of<WalletState>(context, listen: false);
      final wallet = walletState.wallet;
      if (wallet == null) throw Exception('No wallet connected');

      if (_selectedCoinTypeA == null || _selectedCoinTypeB == null) {
        throw Exception('Please select both tokens');
      }

      final feePercent = int.parse(_feePercentController.text.trim());

      final client = KanariClient(walletState.environment.rpcUrl);

      final result = await client.createPool(
        wallet: wallet,
        coinTypeA: _selectedCoinTypeA!,
        coinTypeB: _selectedCoinTypeB!,
        feePercent: feePercent,
      );

      if (!mounted) return;
      setState(() {
        _successMessage =
            'Pool created successfully! Tx Hash: ${result.hash.substring(0, 16)}...';
      });

      // Reset selections
      setState(() {
        _selectedCoinTypeA = null;
        _selectedCoinTypeB = null;
      });
      _feePercentController.clear();

      await walletState.refreshBalance();
      await _loadUserPools(); // Refresh pools list
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _errorMessage = 'Failed to create pool: $e';
      });
    } finally {
      if (mounted) {
        setState(() => _isLoading = false);
      }
    }
  }

  // ==================== LIQUIDITY TAB (Combined Add/Remove) ====================
  Widget _buildLiquidityTab() {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final screenPadding = MediaQuery.of(context).size.width < 360 ? 12.0 : 16.0;

    return SingleChildScrollView(
      padding: EdgeInsets.all(screenPadding),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Pool Selection Card
          Container(
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              color: colorScheme.surfaceContainerHigh,
              borderRadius: BorderRadius.circular(16),
              border: Border.all(color: colorScheme.outline.withOpacity(0.2)),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Icon(
                      Icons.pool_outlined,
                      color: colorScheme.primary,
                      size: 20,
                    ),
                    const SizedBox(width: 8),
                    Text(
                      'Select Pool',
                      style: TextStyle(
                        fontSize: 16,
                        fontWeight: FontWeight.bold,
                        color: colorScheme.onSurface,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                if (_userPools.isEmpty)
                  Container(
                    padding: const EdgeInsets.all(16),
                    decoration: BoxDecoration(
                      color: colorScheme.errorContainer.withOpacity(0.1),
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(color: colorScheme.errorContainer),
                    ),
                    child: Row(
                      children: [
                        Icon(Icons.info_outline, color: colorScheme.error),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            'No pools found. Create a pool first.',
                            style: TextStyle(
                              color: colorScheme.onErrorContainer,
                            ),
                          ),
                        ),
                      ],
                    ),
                  )
                else
                  DropdownButtonFormField<Map<String, dynamic>>(
                    value: _selectedPool,
                    decoration: InputDecoration(
                      labelText: 'Choose Pool',
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(12),
                      ),
                      filled: true,
                      fillColor: colorScheme.surface,
                    ),
                    items: _userPools.map((pool) {
                      final name =
                          '${pool['coin_type_a']} / ${pool['coin_type_b']}';
                      return DropdownMenuItem(
                        value: pool,
                        child: Text(
                          name.length > 40
                              ? '${name.substring(0, 40)}...'
                              : name,
                        ),
                      );
                    }).toList(),
                    onChanged: (pool) {
                      setState(() => _selectedPool = pool);
                    },
                  ),
              ],
            ),
          ),

          const SizedBox(height: 16),

          // Action Tabs (Add/Remove)
          if (_selectedPool != null) ...[
            DefaultTabController(
              length: 2,
              child: Column(
                children: [
                  Container(
                    decoration: BoxDecoration(
                      color: colorScheme.surfaceContainerHigh,
                      borderRadius: BorderRadius.circular(12),
                    ),
                    child: TabBar(
                      labelColor: colorScheme.primary,
                      unselectedLabelColor: colorScheme.onSurfaceVariant,
                      indicatorColor: colorScheme.primary,
                      indicatorWeight: 3,
                      dividerColor: Colors.transparent,
                      tabs: const [
                        Tab(text: 'Add Liquidity'),
                        Tab(text: 'Remove'),
                      ],
                    ),
                  ),
                  SizedBox(
                    height: 400,
                    child: TabBarView(
                      children: [
                        _buildAddLiquidityForm(),
                        _buildRemoveLiquidityForm(),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          ],

          // Error/Success Messages
          if (_errorMessage != null) ...[
            const SizedBox(height: 12),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: colorScheme.errorContainer.withOpacity(0.1),
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: colorScheme.errorContainer),
              ),
              child: Row(
                children: [
                  Icon(Icons.error_outline, color: colorScheme.error),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      _errorMessage!,
                      style: TextStyle(color: colorScheme.onErrorContainer),
                    ),
                  ),
                ],
              ),
            ),
          ],
          if (_successMessage != null) ...[
            const SizedBox(height: 12),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: colorScheme.primaryContainer.withOpacity(0.1),
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: colorScheme.primaryContainer),
              ),
              child: Row(
                children: [
                  Icon(Icons.check_circle_outline, color: colorScheme.primary),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      _successMessage!,
                      style: TextStyle(color: colorScheme.onPrimaryContainer),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ],
      ),
    );
  }

  Widget _buildAddLiquidityForm() {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          TextField(
            controller: _amountAController,
            keyboardType: TextInputType.number,
            decoration: InputDecoration(
              labelText: 'Amount ${_selectedPool!['coin_type_a']}',
              border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(12),
              ),
              filled: true,
              fillColor: colorScheme.surface,
              prefixIcon: Icon(
                Icons.add_circle_outline,
                color: colorScheme.primary,
              ),
            ),
          ),
          const SizedBox(height: 12),
          TextField(
            controller: _amountBController,
            keyboardType: TextInputType.number,
            decoration: InputDecoration(
              labelText: 'Amount ${_selectedPool!['coin_type_b']}',
              border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(12),
              ),
              filled: true,
              fillColor: colorScheme.surface,
              prefixIcon: Icon(
                Icons.add_circle_outline,
                color: colorScheme.primary,
              ),
            ),
          ),
          const SizedBox(height: 16),
          ElevatedButton.icon(
            onPressed: _isLoading ? null : _addLiquidity,
            icon: Icon(_isLoading ? Icons.hourglass_empty : Icons.add),
            label: Text(_isLoading ? 'Adding...' : 'Add Liquidity'),
            style: ElevatedButton.styleFrom(
              minimumSize: const Size.fromHeight(48),
              backgroundColor: colorScheme.primary,
              foregroundColor: colorScheme.onPrimary,
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(12),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildRemoveLiquidityForm() {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          TextField(
            controller: _lpAmountController,
            keyboardType: TextInputType.number,
            decoration: InputDecoration(
              labelText: 'LP Token Amount to Remove',
              border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(12),
              ),
              filled: true,
              fillColor: colorScheme.surface,
              prefixIcon: Icon(
                Icons.remove_circle_outline,
                color: colorScheme.error,
              ),
            ),
          ),
          const SizedBox(height: 16),
          ElevatedButton.icon(
            onPressed: _isLoading ? null : _removeLiquidity,
            icon: Icon(_isLoading ? Icons.hourglass_empty : Icons.remove),
            label: Text(_isLoading ? 'Removing...' : 'Remove Liquidity'),
            style: ElevatedButton.styleFrom(
              minimumSize: const Size.fromHeight(48),
              backgroundColor: colorScheme.error,
              foregroundColor: colorScheme.onError,
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(12),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Future<void> _addLiquidity() async {
    setState(() {
      _isLoading = true;
      _errorMessage = null;
      _successMessage = null;
    });

    try {
      final walletState = Provider.of<WalletState>(context, listen: false);
      final wallet = walletState.wallet;
      if (wallet == null) throw Exception('No wallet connected');

      if (_selectedPool == null) {
        throw Exception('Please select a pool first');
      }

      final amountA = int.parse(_amountAController.text.trim());
      final amountB = int.parse(_amountBController.text.trim());

      if (amountA <= 0 || amountB <= 0) {
        throw Exception('Amounts must be greater than zero');
      }

      final client = KanariClient(walletState.environment.rpcUrl);

      final result = await client.addLiquidity(
        wallet: wallet,
        poolObjectId: _selectedPool!['pool_id'],
        coinTypeA: _selectedPool!['coin_type_a'],
        coinTypeB: _selectedPool!['coin_type_b'],
        amountA: amountA,
        amountB: amountB,
      );

      if (!mounted) return;
      setState(() {
        _successMessage =
            'Liquidity added successfully! Tx: ${result.hash.substring(0, 16)}...';
      });

      _amountAController.clear();
      _amountBController.clear();

      await walletState.refreshBalance();
      await _loadUserPools(); // Refresh pools list
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _errorMessage = 'Failed to add liquidity: $e';
      });
    } finally {
      if (mounted) {
        setState(() => _isLoading = false);
      }
    }
  }

  Future<void> _removeLiquidity() async {
    setState(() {
      _isLoading = true;
      _errorMessage = null;
      _successMessage = null;
    });

    try {
      final walletState = Provider.of<WalletState>(context, listen: false);
      final wallet = walletState.wallet;
      if (wallet == null) throw Exception('No wallet connected');

      if (_selectedPool == null) {
        throw Exception('Please select a pool first');
      }

      final lpAmount = int.parse(_lpAmountController.text.trim());

      if (lpAmount <= 0) {
        throw Exception('LP amount must be greater than zero');
      }

      final client = KanariClient(walletState.environment.rpcUrl);

      final result = await client.removeLiquidity(
        wallet: wallet,
        poolObjectId: _selectedPool!['pool_id'],
        coinTypeA: _selectedPool!['coin_type_a'],
        coinTypeB: _selectedPool!['coin_type_b'],
        lpTokenAmount: lpAmount,
      );

      if (!mounted) return;
      setState(() {
        _successMessage =
            'Liquidity removed successfully! Tx: ${result.hash.substring(0, 16)}...';
      });

      _lpAmountController.clear();

      await walletState.refreshBalance();
      await _loadUserPools(); // Refresh pools list
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _errorMessage = 'Failed to remove liquidity: $e';
      });
    } finally {
      if (mounted) {
        setState(() => _isLoading = false);
      }
    }
  }

  // ==================== SWAP TAB (Improved) ====================
  Widget _buildSwapTab() {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final screenPadding = MediaQuery.of(context).size.width < 360 ? 12.0 : 16.0;

    return SingleChildScrollView(
      padding: EdgeInsets.all(screenPadding),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          // Pool Selection Card
          Container(
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              color: colorScheme.surfaceContainerHigh,
              borderRadius: BorderRadius.circular(16),
              border: Border.all(color: colorScheme.outline.withOpacity(0.2)),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Icon(
                      Icons.pool_outlined,
                      color: colorScheme.primary,
                      size: 20,
                    ),
                    const SizedBox(width: 8),
                    Text(
                      'Select Pool',
                      style: TextStyle(
                        fontSize: 16,
                        fontWeight: FontWeight.bold,
                        color: colorScheme.onSurface,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                if (_userPools.isEmpty)
                  Container(
                    padding: const EdgeInsets.all(16),
                    decoration: BoxDecoration(
                      color: colorScheme.errorContainer.withOpacity(0.1),
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(color: colorScheme.errorContainer),
                    ),
                    child: Row(
                      children: [
                        Icon(Icons.info_outline, color: colorScheme.error),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            'No pools found. Create a pool first.',
                            style: TextStyle(
                              color: colorScheme.onErrorContainer,
                            ),
                          ),
                        ),
                      ],
                    ),
                  )
                else
                  DropdownButtonFormField<Map<String, dynamic>>(
                    value: _selectedPool,
                    decoration: InputDecoration(
                      labelText: 'Choose Pool',
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(12),
                      ),
                      filled: true,
                      fillColor: colorScheme.surface,
                    ),
                    items: _userPools.map((pool) {
                      final name =
                          '${pool['coin_type_a']} / ${pool['coin_type_b']}';
                      return DropdownMenuItem(
                        value: pool,
                        child: Text(
                          name.length > 40
                              ? '${name.substring(0, 40)}...'
                              : name,
                        ),
                      );
                    }).toList(),
                    onChanged: (pool) {
                      setState(() => _selectedPool = pool);
                    },
                  ),
              ],
            ),
          ),

          const SizedBox(height: 16),

          // Swap Form Card
          if (_selectedPool != null)
            Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: colorScheme.surfaceContainerHigh,
                borderRadius: BorderRadius.circular(16),
                border: Border.all(color: colorScheme.outline.withOpacity(0.2)),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(
                        Icons.swap_horiz,
                        color: colorScheme.primary,
                        size: 20,
                      ),
                      const SizedBox(width: 8),
                      Text(
                        'Swap Tokens',
                        style: TextStyle(
                          fontSize: 18,
                          fontWeight: FontWeight.bold,
                          color: colorScheme.onSurface,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 16),

                  // From Token Display
                  Container(
                    padding: const EdgeInsets.all(12),
                    decoration: BoxDecoration(
                      color: colorScheme.primaryContainer.withOpacity(0.3),
                      borderRadius: BorderRadius.circular(12),
                      border: Border.all(
                        color: colorScheme.primary.withOpacity(0.3),
                      ),
                    ),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'From:',
                          style: TextStyle(
                            fontSize: 12,
                            color: colorScheme.onPrimaryContainer,
                          ),
                        ),
                        const SizedBox(height: 4),
                        Text(
                          _swapDirection == 'AtoB'
                              ? _selectedPool!['coin_type_a']
                              : _selectedPool!['coin_type_b'],
                          style: TextStyle(
                            fontSize: 14,
                            fontWeight: FontWeight.w600,
                            color: colorScheme.onPrimaryContainer,
                          ),
                        ),
                      ],
                    ),
                  ),

                  const SizedBox(height: 12),

                  // Direction Selector
                  SegmentedButton<String>(
                    segments: const [
                      ButtonSegment(
                        value: 'AtoB',
                        label: Text('→'),
                        icon: Icon(Icons.arrow_forward, size: 16),
                      ),
                      ButtonSegment(
                        value: 'BtoA',
                        label: Text('←'),
                        icon: Icon(Icons.arrow_back, size: 16),
                      ),
                    ],
                    selected: {_swapDirection},
                    onSelectionChanged: (Set<String> newSelection) {
                      setState(() => _swapDirection = newSelection.first);
                    },
                  ),

                  const SizedBox(height: 12),

                  // To Token Display
                  Container(
                    padding: const EdgeInsets.all(12),
                    decoration: BoxDecoration(
                      color: colorScheme.tertiaryContainer.withOpacity(0.3),
                      borderRadius: BorderRadius.circular(12),
                      border: Border.all(
                        color: colorScheme.tertiary.withOpacity(0.3),
                      ),
                    ),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'To:',
                          style: TextStyle(
                            fontSize: 12,
                            color: colorScheme.onTertiaryContainer,
                          ),
                        ),
                        const SizedBox(height: 4),
                        Text(
                          _swapDirection == 'AtoB'
                              ? _selectedPool!['coin_type_b']
                              : _selectedPool!['coin_type_a'],
                          style: TextStyle(
                            fontSize: 14,
                            fontWeight: FontWeight.w600,
                            color: colorScheme.onTertiaryContainer,
                          ),
                        ),
                      ],
                    ),
                  ),

                  const SizedBox(height: 16),

                  // Amount Input
                  TextField(
                    controller: _swapAmountInController,
                    keyboardType: TextInputType.number,
                    decoration: InputDecoration(
                      labelText: 'Amount to Swap',
                      hintText: 'Enter amount',
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(12),
                      ),
                      prefixIcon: Icon(
                        Icons.currency_exchange,
                        color: colorScheme.primary,
                      ),
                      filled: true,
                      fillColor: colorScheme.surface,
                    ),
                  ),

                  const SizedBox(height: 16),

                  // Swap Button
                  ElevatedButton.icon(
                    onPressed: _isLoading ? null : _swap,
                    icon: Icon(
                      _isLoading ? Icons.hourglass_empty : Icons.swap_horiz,
                    ),
                    label: Text(_isLoading ? 'Swapping...' : 'Swap Now'),
                    style: ElevatedButton.styleFrom(
                      minimumSize: const Size.fromHeight(48),
                      backgroundColor: colorScheme.primary,
                      foregroundColor: colorScheme.onPrimary,
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(12),
                      ),
                    ),
                  ),
                ],
              ),
            ),

          // Error/Success Messages
          if (_errorMessage != null) ...[
            const SizedBox(height: 12),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: colorScheme.errorContainer.withOpacity(0.1),
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: colorScheme.errorContainer),
              ),
              child: Row(
                children: [
                  Icon(Icons.error_outline, color: colorScheme.error),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      _errorMessage!,
                      style: TextStyle(color: colorScheme.onErrorContainer),
                    ),
                  ),
                ],
              ),
            ),
          ],
          if (_successMessage != null) ...[
            const SizedBox(height: 12),
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: colorScheme.primaryContainer.withOpacity(0.1),
                borderRadius: BorderRadius.circular(12),
                border: Border.all(color: colorScheme.primaryContainer),
              ),
              child: Row(
                children: [
                  Icon(Icons.check_circle_outline, color: colorScheme.primary),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      _successMessage!,
                      style: TextStyle(color: colorScheme.onPrimaryContainer),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ],
      ),
    );
  }

  Future<void> _swap() async {
    setState(() {
      _isLoading = true;
      _errorMessage = null;
      _successMessage = null;
    });

    try {
      final walletState = Provider.of<WalletState>(context, listen: false);
      final wallet = walletState.wallet;
      if (wallet == null) throw Exception('No wallet connected');

      if (_selectedPool == null) {
        throw Exception('Please select a pool first');
      }

      final amountIn = int.parse(_swapAmountInController.text.trim());

      if (amountIn <= 0) {
        throw Exception('Amount must be greater than zero');
      }

      final client = KanariClient(walletState.environment.rpcUrl);

      TransactionResult result;
      if (_swapDirection == 'AtoB') {
        result = await client.swapAForB(
          wallet: wallet,
          poolObjectId: _selectedPool!['pool_id'],
          coinTypeA: _selectedPool!['coin_type_a'],
          coinTypeB: _selectedPool!['coin_type_b'],
          amountIn: amountIn,
        );
      } else {
        result = await client.swapBForA(
          wallet: wallet,
          poolObjectId: _selectedPool!['pool_id'],
          coinTypeA: _selectedPool!['coin_type_a'],
          coinTypeB: _selectedPool!['coin_type_b'],
          amountIn: amountIn,
        );
      }

      if (!mounted) return;
      setState(() {
        _successMessage =
            'Swap completed! Tx: ${result.hash.substring(0, 16)}...';
      });

      _swapAmountInController.clear();

      await walletState.refreshBalance();
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _errorMessage = 'Failed to swap: $e';
      });
    } finally {
      if (mounted) {
        setState(() => _isLoading = false);
      }
    }
  }

  String _getShortTokenName(String tokenType) {
    // Extract last part after ::
    final parts = tokenType.split('::');
    if (parts.length >= 3) {
      return parts.last;
    }
    // If format is different, just truncate
    return tokenType.length > 20
        ? '${tokenType.substring(0, 20)}...'
        : tokenType;
  }
}
