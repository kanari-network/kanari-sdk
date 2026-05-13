import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_spinkit/flutter_spinkit.dart';
import 'package:provider/provider.dart';

import '../../client/dex_client.dart';

import '../../models/transaction.dart';
import '../../providers/wallet_provider.dart';
import '../widgets/app_ui.dart';

class DEXScreen extends StatefulWidget {
  const DEXScreen({super.key});

  @override
  State<DEXScreen> createState() => _DEXScreenState();
}

class _DEXScreenState extends State<DEXScreen> with SingleTickerProviderStateMixin {
  late final TabController _tabController;
  late DexClient _dexClient;
  bool _isLoading = false;
  String? _errorMessage;
  String? _successMessage;
  
  List<Map<String, dynamic>> _pools = [];
  List<String> _tokens = [];
  String? _selectedPoolId;
  String? _selectedCoinTypeA;
  String? _selectedCoinTypeB;

  // Form controllers
  final _amountAController = TextEditingController();
  final _amountBController = TextEditingController();
  final _swapAmountController = TextEditingController();

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 3, vsync: this);
    final walletState = context.read<WalletState>();
    _dexClient = DexClient(walletState.client?.url ?? 'http://localhost:30731');
    _loadPoolsAndTokens();
  }

  @override
  void dispose() {
    _tabController.dispose();
    _amountAController.dispose();
    _amountBController.dispose();
    _swapAmountController.dispose();
    super.dispose();
  }

  Future<void> _loadPoolsAndTokens() async {
    setState(() {
      _isLoading = true;
      _errorMessage = null;
      _successMessage = null;
    });

    try {
      final walletState = context.read<WalletState>();
      final address = walletState.wallet?.address;

      if (address != null) {
        final pools = await _dexClient.getUserPools(address);
        final tokens = await _dexClient.getUserTokens(address);

        setState(() {
          _pools = pools;
          _tokens = tokens;
        });
      }
    } catch (e) {
      debugPrint('[DEX] Error loading pools and tokens: $e');
      if (mounted) {
        setState(() {
          _errorMessage = 'Failed to load data: $e';
        });
      }
    } finally {
      if (mounted) {
        setState(() => _isLoading = false);
      }
    }
  }

  void _clearMessages() {
    setState(() {
      _errorMessage = null;
      _successMessage = null;
    });
  }

  Future<void> _createPool() async {
    final walletState = context.read<WalletState>();
    final wallet = walletState.wallet;

    if (wallet == null) {
      setState(() {
        _errorMessage = 'Please connect wallet first';
      });
      return;
    }

    if (_tokens.length < 2) {
      setState(() {
        _errorMessage = 'Need at least 2 different tokens';
      });
      return;
    }

    setState(() {
      _isLoading = true;
      _errorMessage = null;
      _successMessage = null;
    });

    try {
      final result = await _dexClient.createPool(
        wallet: wallet,
        coinTypeA: _tokens[0],
        coinTypeB: _tokens[1],
        feePercent: 30, // 0.3% fee
      );

      if (mounted) {
        setState(() {
          _successMessage = 'Pool created! TX: ${result.hash.substring(0, 16)}...';
        });
        await _loadPoolsAndTokens();
      }
    } catch (e) {
      debugPrint('[DEX] Error creating pool: $e');
      if (mounted) {
        setState(() {
          _errorMessage = 'Failed to create pool: $e';
        });
      }
    } finally {
      if (mounted) {
        setState(() => _isLoading = false);
      }
    }
  }

  Future<void> _addLiquidity() async {
    final walletState = context.read<WalletState>();
    final wallet = walletState.wallet;

    if (wallet == null || _selectedPoolId == null) {
      setState(() {
        _errorMessage = 'Please select a pool first';
      });
      return;
    }

    final amountA = int.tryParse(_amountAController.text);
    final amountB = int.tryParse(_amountBController.text);

    if (amountA == null || amountB == null || amountA <= 0 || amountB <= 0) {
      setState(() {
        _errorMessage = 'Please enter valid amounts';
      });
      return;
    }

    setState(() {
      _isLoading = true;
      _errorMessage = null;
      _successMessage = null;
    });

    try {
      // Get owned objects to find coin object IDs
      final objects = await _dexClient.getOwnedObjects(wallet.address);

      String? coinAObjectId;
      String? coinBObjectId;

      for (final obj in objects) {
        final objType = obj['type'] as String? ?? '';
        if (objType.contains(_selectedCoinTypeA!) &&
            objType.contains('Coin<')) {
          coinAObjectId = obj['id'] as String? ?? obj['objectId'] as String?;
        }
        if (objType.contains(_selectedCoinTypeB!) &&
            objType.contains('Coin<')) {
          coinBObjectId = obj['id'] as String? ?? obj['objectId'] as String?;
        }
      }

      if (coinAObjectId == null || coinBObjectId == null) {
        throw Exception(
          'Could not find coin objects. Please ensure you have enough balance.',
        );
      }

      final result = await _dexClient.addLiquidity(
        wallet: wallet,
        poolObjectId: _selectedPoolId!,
        coinTypeA: _selectedCoinTypeA!,
        coinTypeB: _selectedCoinTypeB!,
        coinAObjectId: coinAObjectId,
        coinBObjectId: coinBObjectId,
        amountA: amountA,
        amountB: amountB,
      );

      if (mounted) {
        setState(() {
          _successMessage = 'Liquidity added! TX: ${result.hash.substring(0, 16)}...';
        });
        await _loadPoolsAndTokens();
        _amountAController.clear();
        _amountBController.clear();
      }
    } catch (e) {
      debugPrint('[DEX] Error adding liquidity: $e');
      if (mounted) {
        setState(() {
          _errorMessage = 'Failed to add liquidity: $e';
        });
      }
    } finally {
      if (mounted) {
        setState(() => _isLoading = false);
      }
    }
  }

  Future<void> _swap(String direction) async {
    final walletState = context.read<WalletState>();
    final wallet = walletState.wallet;

    if (wallet == null || _selectedPoolId == null) {
      setState(() {
        _errorMessage = 'Please select a pool first';
      });
      return;
    }

    final amountIn = int.tryParse(_swapAmountController.text);

    if (amountIn == null || amountIn <= 0) {
      setState(() {
        _errorMessage = 'Please enter a valid amount';
      });
      return;
    }

    setState(() {
      _isLoading = true;
      _errorMessage = null;
      _successMessage = null;
    });

    try {
      // Get owned objects to find coin object ID
      final objects = await _dexClient.getOwnedObjects(wallet.address);

      final coinInType = direction == 'A'
          ? _selectedCoinTypeA
          : _selectedCoinTypeB;
      String? coinInObjectId;

      for (final obj in objects) {
        final objType = obj['type'] as String? ?? '';
        if (objType.contains(coinInType!) && objType.contains('Coin<')) {
          coinInObjectId = obj['id'] as String? ?? obj['objectId'] as String?;
          break;
        }
      }

      if (coinInObjectId == null) {
        throw Exception(
          'Could not find input coin object. Please ensure you have enough balance.',
        );
      }

      TransactionResult result;
      if (direction == 'A') {
        result = await _dexClient.swapAForB(
          wallet: wallet,
          poolObjectId: _selectedPoolId!,
          coinTypeA: _selectedCoinTypeA!,
          coinTypeB: _selectedCoinTypeB!,
          coinInObjectId: coinInObjectId,
          amountIn: amountIn,
        );
      } else {
        result = await _dexClient.swapBForA(
          wallet: wallet,
          poolObjectId: _selectedPoolId!,
          coinTypeA: _selectedCoinTypeA!,
          coinTypeB: _selectedCoinTypeB!,
          coinInObjectId: coinInObjectId,
          amountIn: amountIn,
        );
      }

      if (mounted) {
        setState(() {
          _successMessage = 'Swap successful! TX: ${result.hash.substring(0, 16)}...';
        });
        await _loadPoolsAndTokens();
        _swapAmountController.clear();
      }
    } catch (e) {
      debugPrint('[DEX] Error swapping: $e');
      if (mounted) {
        setState(() {
          _errorMessage = 'Failed to swap: $e';
        });
      }
    } finally {
      if (mounted) {
        setState(() => _isLoading = false);
      }
    }
  }

  Widget _buildFeedback(ColorScheme colorScheme) {
    if (_errorMessage == null && _successMessage == null) {
      return const SizedBox.shrink();
    }

    final isError = _errorMessage != null;
    final message = isError ? _errorMessage! : _successMessage!;

    return Column(
      children: [
        if (isError)
          AppErrorBanner(message: message)
        else
          Container(
            margin: const EdgeInsets.only(bottom: 16),
            padding: const EdgeInsets.all(14),
            decoration: BoxDecoration(
              color: Colors.green.withOpacity(0.12),
              borderRadius: BorderRadius.circular(16),
              border: Border.all(color: Colors.green.withOpacity(0.3)),
            ),
            child: Row(
              children: [
                const Icon(
                  Icons.check_circle_outline_rounded,
                  color: Colors.green,
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Text(
                    message,
                    style: const TextStyle(color: Colors.green),
                  ),
                ),
                IconButton(
                  onPressed: _clearMessages,
                  icon: const Icon(Icons.close, color: Colors.green),
                ),
              ],
            ),
          ),
      ],
    );
  }

  Widget _buildPrimaryButton({
    required VoidCallback? onPressed,
    required IconData icon,
    required String label,
    bool isLoading = false,
  }) {
    return AppWideButton(
      onPressed: (isLoading || onPressed == null) ? null : onPressed,
      icon: icon,
      label: isLoading ? 'Processing...' : label,
      style: AppWideButtonStyle.primary,
    );
  }

  Widget _buildOutlinedButton({
    required VoidCallback? onPressed,
    required IconData icon,
    required String label,
    bool isLoading = false,
  }) {
    return AppWideButton(
      onPressed: (isLoading || onPressed == null) ? null : onPressed,
      icon: icon,
      label: isLoading ? 'Processing...' : label,
      style: AppWideButtonStyle.outlined,
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return AppGradientScaffold(
      appBar: AppBar(
        title: const Text('DEX'),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            onPressed: _isLoading ? null : _loadPoolsAndTokens,
          ),
        ],
      ),
      body: _isLoading
          ? Center(child: SpinKitFadingCircle(color: colorScheme.primary))
          : RefreshIndicator(
              onRefresh: _loadPoolsAndTokens,
              backgroundColor: colorScheme.surface,
              color: colorScheme.primary,
              child: CustomScrollView(
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
                          Tab(icon: Icon(Icons.pool), text: 'Pools'),
                          Tab(icon: Icon(Icons.add_circle), text: 'Liquidity'),
                          Tab(icon: Icon(Icons.swap_horiz), text: 'Swap'),
                        ],
                      ),
                    ),
                  ),

                  // Feedback Messages
                  SliverToBoxAdapter(
                    child: Padding(
                      padding: const EdgeInsets.symmetric(horizontal: 16),
                      child: _buildFeedback(colorScheme),
                    ),
                  ),

                  // Tab Content
                  SliverFillRemaining(
                    hasScrollBody: true,
                    child: TabBarView(
                      controller: _tabController,
                      children: [
                        _buildPoolsTab(),
                        _buildLiquidityTab(),
                        _buildSwapTab(),
                      ],
                    ),
                  ),
                ],
              ),
            ),
    );
  }

  Widget _buildPoolsTab() {
    final colorScheme = Theme.of(context).colorScheme;

    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _buildBanner(
            title: 'Your Pools',
            subtitle: 'Manage your liquidity pools',
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const SizedBox(height: 12),
                if (_pools.isEmpty)
                  Container(
                    padding: const EdgeInsets.all(16),
                    decoration: BoxDecoration(
                      color: colorScheme.surfaceVariant.withOpacity(0.3),
                      borderRadius: BorderRadius.circular(12),
                    ),
                    child: Row(
                      children: [
                        Icon(Icons.info_outline, color: colorScheme.onSurfaceVariant),
                        const SizedBox(width: 12),
                        Expanded(
                          child: Text(
                            'No pools found. Create one first!',
                            style: TextStyle(color: colorScheme.onSurfaceVariant),
                          ),
                        ),
                      ],
                    ),
                  )
                else
                  ListView.builder(
                    shrinkWrap: true,
                    physics: const NeverScrollableScrollPhysics(),
                    itemCount: _pools.length,
                    itemBuilder: (context, index) {
                      final pool = _pools[index];
                      final poolId = pool['pool_id'] as String? ?? '';
                      final coinA = pool['coin_type_a']?.toString() ?? '';
                      final coinB = pool['coin_type_b']?.toString() ?? '';
                      
                      return Card(
                        margin: const EdgeInsets.only(bottom: 8),
                        child: ListTile(
                          leading: CircleAvatar(
                            backgroundColor: colorScheme.primaryContainer,
                            child: Icon(
                              Icons.pool,
                              color: colorScheme.onPrimaryContainer,
                            ),
                          ),
                          title: Text(
                            '${coinA.split('::').last} / ${coinB.split('::').last}',
                            style: const TextStyle(fontWeight: FontWeight.bold),
                          ),
                          subtitle: Text(
                            poolId.length > 20 ? '${poolId.substring(0, 20)}...' : poolId,
                            style: const TextStyle(fontFamily: 'monospace', fontSize: 11),
                          ),
                          trailing: IconButton(
                            icon: const Icon(Icons.arrow_forward_ios, size: 16),
                            onPressed: () {
                              setState(() {
                                _selectedPoolId = poolId;
                                _selectedCoinTypeA = pool['coin_type_a'];
                                _selectedCoinTypeB = pool['coin_type_b'];
                                _tabController.animateTo(1);
                              });
                            },
                          ),
                        ),
                      );
                    },
                  ),
                const SizedBox(height: 12),
                _buildPrimaryButton(
                  onPressed: _isLoading ? null : _createPool,
                  icon: Icons.add,
                  label: 'Create New Pool',
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildLiquidityTab() {
    final colorScheme = Theme.of(context).colorScheme;

    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _buildBanner(
            title: 'Add Liquidity',
            subtitle: 'Provide liquidity to earn fees',
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const SizedBox(height: 12),
                DropdownButtonFormField<String>(
                  value: _selectedPoolId,
                  hint: const Text('Select a pool'),
                  decoration: const InputDecoration(
                    prefixIcon: Icon(Icons.pool_outlined),
                  ),
                  items: _pools.map((pool) {
                    final poolId = pool['pool_id'] as String?;
                    final coinA = pool['coin_type_a']?.toString() ?? '';
                    final coinB = pool['coin_type_b']?.toString() ?? '';
                    return DropdownMenuItem<String>(
                      value: poolId,
                      child: Text(
                        '${coinA.split('::').last} / ${coinB.split('::').last}',
                      ),
                    );
                  }).toList(),
                  onChanged: (value) {
                    setState(() {
                      _selectedPoolId = value;
                      final selectedPool = _pools.firstWhere(
                        (p) => p['pool_id'] == value,
                      );
                      _selectedCoinTypeA = selectedPool['coin_type_a'];
                      _selectedCoinTypeB = selectedPool['coin_type_b'];
                    });
                  },
                ),
                const SizedBox(height: 16),
                if (_selectedPoolId != null) ...[
                  TextFormField(
                    controller: _amountAController,
                    decoration: InputDecoration(
                      labelText: 'Amount ${_selectedCoinTypeA?.split('::').last}',
                      prefixIcon: const Icon(Icons.currency_bitcoin),
                    ),
                    keyboardType: TextInputType.number,
                  ),
                  const SizedBox(height: 12),
                  TextFormField(
                    controller: _amountBController,
                    decoration: InputDecoration(
                      labelText: 'Amount ${_selectedCoinTypeB?.split('::').last}',
                      prefixIcon: const Icon(Icons.currency_bitcoin),
                    ),
                    keyboardType: TextInputType.number,
                  ),
                  const SizedBox(height: 16),
                  _buildPrimaryButton(
                    onPressed: _isLoading ? null : _addLiquidity,
                    icon: Icons.add_circle,
                    label: 'Add Liquidity',
                  ),
                ] else
                  Container(
                    padding: const EdgeInsets.all(16),
                    decoration: BoxDecoration(
                      color: colorScheme.surfaceVariant.withOpacity(0.3),
                      borderRadius: BorderRadius.circular(12),
                    ),
                    child: Row(
                      children: [
                        Icon(Icons.info_outline, color: colorScheme.onSurfaceVariant),
                        const SizedBox(width: 12),
                        Expanded(
                          child: Text(
                            'Please select a pool first',
                            style: TextStyle(color: colorScheme.onSurfaceVariant),
                          ),
                        ),
                      ],
                    ),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildSwapTab() {
    final colorScheme = Theme.of(context).colorScheme;

    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _buildBanner(
            title: 'Swap Tokens',
            subtitle: 'Exchange tokens instantly',
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                const SizedBox(height: 12),
                DropdownButtonFormField<String>(
                  value: _selectedPoolId,
                  hint: const Text('Select a pool'),
                  decoration: const InputDecoration(
                    prefixIcon: Icon(Icons.pool_outlined),
                  ),
                  items: _pools.map((pool) {
                    final poolId = pool['pool_id'] as String?;
                    final coinA = pool['coin_type_a']?.toString() ?? '';
                    final coinB = pool['coin_type_b']?.toString() ?? '';
                    return DropdownMenuItem<String>(
                      value: poolId,
                      child: Text(
                        '${coinA.split('::').last} / ${coinB.split('::').last}',
                      ),
                    );
                  }).toList(),
                  onChanged: (value) {
                    setState(() {
                      _selectedPoolId = value;
                      final selectedPool = _pools.firstWhere(
                        (p) => p['pool_id'] == value,
                      );
                      _selectedCoinTypeA = selectedPool['coin_type_a'];
                      _selectedCoinTypeB = selectedPool['coin_type_b'];
                    });
                  },
                ),
                const SizedBox(height: 16),
                if (_selectedPoolId != null) ...[
                  TextFormField(
                    controller: _swapAmountController,
                    decoration: const InputDecoration(
                      labelText: 'Amount to swap',
                      prefixIcon: Icon(Icons.currency_bitcoin),
                    ),
                    keyboardType: TextInputType.number,
                  ),
                  const SizedBox(height: 16),
                  Row(
                    children: [
                      Expanded(
                        child: _buildOutlinedButton(
                          onPressed: _isLoading ? null : () => _swap('A'),
                          icon: Icons.arrow_forward,
                          label: '${_selectedCoinTypeA?.split('::').last} → ${_selectedCoinTypeB?.split('::').last}',
                        ),
                      ),
                      const SizedBox(width: 8),
                      Expanded(
                        child: _buildOutlinedButton(
                          onPressed: _isLoading ? null : () => _swap('B'),
                          icon: Icons.arrow_back,
                          label: '${_selectedCoinTypeB?.split('::').last} → ${_selectedCoinTypeA?.split('::').last}',
                        ),
                      ),
                    ],
                  ),
                ] else
                  Container(
                    padding: const EdgeInsets.all(16),
                    decoration: BoxDecoration(
                      color: colorScheme.surfaceVariant.withOpacity(0.3),
                      borderRadius: BorderRadius.circular(12),
                    ),
                    child: Row(
                      children: [
                        Icon(Icons.info_outline, color: colorScheme.onSurfaceVariant),
                        const SizedBox(width: 12),
                        Expanded(
                          child: Text(
                            'Please select a pool first',
                            style: TextStyle(color: colorScheme.onSurfaceVariant),
                          ),
                        ),
                      ],
                    ),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildBanner({
    required String title,
    required String subtitle,
    Widget? child,
  }) {
    return AppPanel(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          AppSectionTitle(title),
          const SizedBox(height: 8),
          Text(subtitle, style: Theme.of(context).textTheme.bodySmall),
          if (child != null) child,
        ],
      ),
    );
  }
}
