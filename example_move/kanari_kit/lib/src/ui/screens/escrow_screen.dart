import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_spinkit/flutter_spinkit.dart';
import 'package:provider/provider.dart';

import '../../escrow_client.dart';
import '../../models/account.dart';
import '../../providers/wallet_provider.dart';

class _EscrowTokenOption {
  final String tokenType;
  final String label;
  final bool isSpendable;

  const _EscrowTokenOption({
    required this.tokenType,
    required this.label,
    required this.isSpendable,
  });
}

class EscrowScreen extends StatefulWidget {
  const EscrowScreen({super.key});

  @override
  State<EscrowScreen> createState() => _EscrowScreenState();
}

class _EscrowScreenState extends State<EscrowScreen>
    with SingleTickerProviderStateMixin {
  late final TabController _tabController;

  final _dealIdController = TextEditingController();
  final _sellerAddressController = TextEditingController();
  final _amountController = TextEditingController();
  final _descriptionController = TextEditingController();
  final _buyerAddressController = TextEditingController();

  bool _isLoading = false;
  bool _isLoadingTokens = false;
  String? _loadedWalletAddress;
  String? _selectedTokenType;
  String? _errorMessage;
  String? _successMessage;
  List<String> _spendableCoinTypes = const [];
  int? _currentDealState;
  int? _proofCount;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 3, vsync: this);
  }

  @override
  void dispose() {
    _tabController.dispose();
    _dealIdController.dispose();
    _sellerAddressController.dispose();
    _amountController.dispose();
    _descriptionController.dispose();
    _buyerAddressController.dispose();
    super.dispose();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final walletAddress = context.read<WalletState>().wallet?.address;
    if (walletAddress != null && walletAddress != _loadedWalletAddress) {
      _loadedWalletAddress = walletAddress;
      unawaited(_loadSpendableCoinTypes());
    }
  }

  EscrowClient? _escrowClient(WalletState state) {
    final client = state.client;
    if (client == null) return null;
    return EscrowClient(client);
  }

  Future<void> _loadSpendableCoinTypes() async {
    final walletState = context.read<WalletState>();
    final wallet = walletState.wallet;
    final escrow = _escrowClient(walletState);
    if (wallet == null || escrow == null) return;

    setState(() {
      _isLoadingTokens = true;
    });

    try {
      final coinTypes = await escrow.getSpendableCoinTypes(wallet.address);
      if (!mounted) return;
      setState(() {
        _spendableCoinTypes = coinTypes;
        _isLoadingTokens = false;
        if (_selectedTokenType == null ||
            !coinTypes.contains(_selectedTokenType)) {
          _selectedTokenType = coinTypes.isEmpty ? null : coinTypes.first;
        }
      });
    } catch (_) {
      if (!mounted) return;
      setState(() {
        _spendableCoinTypes = const [];
        _isLoadingTokens = false;
        _selectedTokenType = null;
      });
    }
  }

  List<_EscrowTokenOption> _buildTokenOptions(WalletState walletState) {
    final spendableTypes = _spendableCoinTypes.toSet();
    final optionsByType = <String, _EscrowTokenOption>{};

    for (final token in walletState.tokenBalances) {
      final isSpendable = spendableTypes.contains(token.tokenType);
      optionsByType[token.tokenType] = _EscrowTokenOption(
        tokenType: token.tokenType,
        label:
            '${token.symbol} (${_formatTokenAmount(token)})${isSpendable ? '' : ' - balance only'}',
        isSpendable: isSpendable,
      );
    }

    for (final tokenType in _spendableCoinTypes) {
      optionsByType.putIfAbsent(
        tokenType,
        () => _EscrowTokenOption(
          tokenType: tokenType,
          label: '$tokenType - spendable',
          isSpendable: true,
        ),
      );
    }

    final options = optionsByType.values.toList()
      ..sort((a, b) => a.label.compareTo(b.label));
    return options;
  }

  String _formatTokenAmount(TokenBalance token) {
    final divisor = token.decimals <= 0
        ? 1.0
        : math.pow(10, token.decimals).toDouble();
    return (token.amount / divisor).toStringAsFixed(4);
  }

  String _friendlyError(Object error) {
    final text = error.toString();

    if (text.contains('SocketException') ||
        text.contains('connection was forcibly closed') ||
        text.contains('errno = 10054')) {
      return 'Kanari node connection lost. Please check if the node is running and try again.';
    }
    if (text.contains('Timeout') || text.contains('timed out')) {
      return 'Request timed out. Please try again.';
    }
    if (text.contains('Only transfer or burn transactions are supported') ||
        text.contains('not supported')) {
      return 'This node does not support escrow Move calls yet.';
    }
    if (text.contains('Escrow objects were not found')) {
      return 'No escrow objects found for this buyer address yet.';
    }
    if (text.contains('No spendable Coin<')) {
      return 'This wallet does not have a spendable coin object for the selected token.';
    }
    return 'An unexpected error occurred: ${text.split('\n').first}';
  }

  Future<void> _runAction(
    Future<void> Function(EscrowClient escrow, WalletState walletState) action,
  ) async {
    final walletState = context.read<WalletState>();
    final escrow = _escrowClient(walletState);
    if (walletState.wallet == null || escrow == null) {
      setState(() {
        _errorMessage = 'Wallet or RPC client is not available.';
      });
      return;
    }

    setState(() {
      _isLoading = true;
      _errorMessage = null;
      _successMessage = null;
    });

    try {
      await action(escrow, walletState);
    } catch (error, stackTrace) {
      // Debug: Log full error and stack trace
      print('[ESCROW] Error occurred:');
      print('[ESCROW]   Error: $error');
      print('[ESCROW]   Stack: $stackTrace');

      if (!mounted) return;
      setState(() {
        _errorMessage = _friendlyError(error);
      });
    } finally {
      if (!mounted) return;
      setState(() {
        _isLoading = false;
      });
    }
  }

  Future<void> _createDeal() async {
    await _runAction((escrow, walletState) async {
      final wallet = walletState.wallet!;
      final tokenType = _selectedTokenType;

      if (_dealIdController.text.trim().isEmpty) {
        throw Exception('Deal ID is required');
      }
      if (_sellerAddressController.text.trim().isEmpty) {
        throw Exception('Seller address is required');
      }
      if (_amountController.text.trim().isEmpty) {
        throw Exception('Amount is required');
      }
      if (tokenType == null || tokenType.isEmpty) {
        throw Exception('Escrow token is required');
      }

      final result = await escrow
          .createDeal(
            wallet: wallet,
            dealId: _dealIdController.text.trim(),
            sellerAddress: _sellerAddressController.text.trim(),
            amount: int.parse(_amountController.text.trim()),
            description: _descriptionController.text.trim(),
            tokenType: tokenType,
          )
          .timeout(const Duration(seconds: 30));

      await walletState.refreshBalance();
      if (!mounted) return;
      setState(() {
        _successMessage =
            'Deal created successfully! Tx Hash: ${result.hash.substring(0, 16)}...';
      });
      _dealIdController.clear();
      _sellerAddressController.clear();
      _amountController.clear();
      _descriptionController.clear();
    });
  }

  Future<void> _confirmDelivery() async {
    await _runAction((escrow, walletState) async {
      final wallet = walletState.wallet!;
      final buyer = _buyerAddressController.text.trim();
      if (buyer.isEmpty) {
        throw Exception('Buyer address is required');
      }

      final result = await escrow
          .confirmDelivery(wallet: wallet, buyerAddress: buyer)
          .timeout(const Duration(seconds: 30));
      await walletState.refreshBalance();
      if (!mounted) return;
      setState(() {
        _successMessage =
            'Delivery confirmed! Tx Hash: ${result.hash.substring(0, 16)}...';
      });
    });
  }

  Future<void> _releaseFunds() async {
    await _runAction((escrow, walletState) async {
      final wallet = walletState.wallet!;
      final buyer = _buyerAddressController.text.trim();
      if (buyer.isEmpty) {
        throw Exception('Buyer address is required');
      }

      final result = await escrow
          .releaseFunds(wallet: wallet, buyerAddress: buyer)
          .timeout(const Duration(seconds: 30));
      await walletState.refreshBalance();
      if (!mounted) return;
      setState(() {
        _successMessage =
            'Funds released successfully! Tx Hash: ${result.hash.substring(0, 16)}...';
      });
    });
  }

  Future<void> _raiseDispute() async {
    await _runAction((escrow, walletState) async {
      final wallet = walletState.wallet!;
      final buyer = _buyerAddressController.text.trim();
      if (buyer.isEmpty) {
        throw Exception('Buyer address is required');
      }

      final result = await escrow
          .raiseDispute(wallet: wallet, buyerAddress: buyer)
          .timeout(const Duration(seconds: 30));
      await walletState.refreshBalance();
      if (!mounted) return;
      setState(() {
        _successMessage =
            'Dispute raised! Tx Hash: ${result.hash.substring(0, 16)}...';
      });
    });
  }

  Future<void> _checkDealState() async {
    await _runAction((escrow, _) async {
      final buyer = _buyerAddressController.text.trim();
      if (buyer.isEmpty) {
        throw Exception('Buyer address is required');
      }

      final state = await escrow
          .getDealState(buyer)
          .timeout(const Duration(seconds: 30));
      final proofCount = await escrow
          .getProofCount(buyer)
          .timeout(const Duration(seconds: 30));

      if (!mounted) return;
      setState(() {
        _currentDealState = state;
        _proofCount = proofCount;
        _successMessage = 'Deal state retrieved successfully!';
      });
    });
  }

  String _stateName(int state) {
    switch (state) {
      case 1:
        return 'Locked';
      case 2:
        return 'Delivered';
      case 3:
        return 'Completed';
      case 4:
        return 'Disputed';
      default:
        return 'Unknown';
    }
  }

  Color _stateColor(int state) {
    switch (state) {
      case 1:
        return Colors.orange;
      case 2:
        return Colors.blue;
      case 3:
        return Colors.green;
      case 4:
        return Colors.red;
      default:
        return Colors.grey;
    }
  }

  @override
  Widget build(BuildContext context) {
    final walletState = context.watch<WalletState>();
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    if (walletState.wallet == null) {
      return Scaffold(
        appBar: AppBar(title: const Text('Escrow')),
        body: Center(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Icon(
                  Icons.account_balance_wallet_outlined,
                  size: 72,
                  color: colorScheme.onSurface.withOpacity(0.5),
                ),
                const SizedBox(height: 20),
                Text(
                  'No Wallet Connected',
                  style: theme.textTheme.headlineSmall?.copyWith(
                    fontWeight: FontWeight.bold,
                  ),
                ),
                const SizedBox(height: 12),
                Text(
                  'Connect your wallet from the Home screen to use escrow features.',
                  textAlign: TextAlign.center,
                  style: theme.textTheme.bodyMedium?.copyWith(
                    color: colorScheme.onSurface.withOpacity(0.7),
                  ),
                ),
              ],
            ),
          ),
        ),
      );
    }

    return Scaffold(
      appBar: AppBar(
        title: const Text('Escrow'),
        bottom: TabBar(
          controller: _tabController,
          tabs: const [
            Tab(icon: Icon(Icons.add_circle_outline), text: 'Create'),
            Tab(icon: Icon(Icons.check_circle_outline), text: 'Actions'),
            Tab(icon: Icon(Icons.search), text: 'Check'),
          ],
        ),
      ),
      body: TabBarView(
        controller: _tabController,
        children: [
          _buildCreateTab(walletState, colorScheme),
          _buildActionsTab(colorScheme),
          _buildCheckTab(colorScheme),
        ],
      ),
    );
  }

  Widget _buildCreateTab(WalletState walletState, ColorScheme colorScheme) {
    final allOptions = _buildTokenOptions(walletState);
    final spendableOptions = allOptions
        .where((option) => option.isSpendable)
        .toList();
    final balanceOnlyOptions = allOptions
        .where((option) => !option.isSpendable)
        .toList();

    if (_selectedTokenType != null &&
        !spendableOptions.any(
          (option) => option.tokenType == _selectedTokenType,
        )) {
      _selectedTokenType = spendableOptions.isEmpty
          ? null
          : spendableOptions.first.tokenType;
    }

    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _buildBanner(
            title: 'Create New Escrow Deal',
            subtitle:
                'Create a new deal and lock funds using a spendable coin object.',
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                if (_isLoadingTokens)
                  const Padding(
                    padding: EdgeInsets.only(top: 12),
                    child: LinearProgressIndicator(),
                  )
                else if (spendableOptions.isEmpty)
                  Padding(
                    padding: const EdgeInsets.only(top: 12),
                    child: Text(
                      'No spendable escrow token was found in this wallet yet.',
                      style: TextStyle(color: colorScheme.error),
                    ),
                  )
                else
                  Padding(
                    padding: const EdgeInsets.only(top: 12),
                    child: DropdownButtonFormField<String>(
                      value: _selectedTokenType,
                      decoration: const InputDecoration(
                        labelText: 'Escrow Token',
                        prefixIcon: Icon(Icons.account_balance_wallet_outlined),
                      ),
                      items: spendableOptions
                          .map(
                            (option) => DropdownMenuItem<String>(
                              value: option.tokenType,
                              child: Text(option.label),
                            ),
                          )
                          .toList(),
                      onChanged: _isLoading
                          ? null
                          : (value) {
                              setState(() {
                                _selectedTokenType = value;
                              });
                            },
                    ),
                  ),
                if (balanceOnlyOptions.isNotEmpty)
                  Padding(
                    padding: const EdgeInsets.only(top: 12),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'Balances found but not selectable for escrow:',
                          style: TextStyle(color: colorScheme.error),
                        ),
                        const SizedBox(height: 8),
                        ...balanceOnlyOptions.map(
                          (option) => Text('- ${option.label}'),
                        ),
                      ],
                    ),
                  ),
              ],
            ),
          ),
          const SizedBox(height: 16),
          _buildFeedback(colorScheme),
          TextFormField(
            controller: _dealIdController,
            decoration: const InputDecoration(
              labelText: 'Deal ID',
              hintText: 'e.g. deal-001',
              prefixIcon: Icon(Icons.tag),
            ),
          ),
          const SizedBox(height: 12),
          TextFormField(
            controller: _sellerAddressController,
            decoration: const InputDecoration(
              labelText: 'Seller Address',
              hintText: '0x...',
              prefixIcon: Icon(Icons.person),
            ),
          ),
          const SizedBox(height: 12),
          TextFormField(
            controller: _amountController,
            keyboardType: TextInputType.number,
            decoration: const InputDecoration(
              labelText: 'Amount',
              hintText: 'Enter amount in smallest unit',
              prefixIcon: Icon(Icons.payments),
            ),
          ),
          const SizedBox(height: 12),
          TextFormField(
            controller: _descriptionController,
            maxLines: 3,
            decoration: const InputDecoration(
              labelText: 'Description',
              hintText: 'Describe the deal',
              prefixIcon: Icon(Icons.description),
            ),
          ),
          const SizedBox(height: 24),
          ElevatedButton.icon(
            onPressed: _isLoading || spendableOptions.isEmpty
                ? null
                : _createDeal,
            icon: _isLoading
                ? const SizedBox(
                    width: 20,
                    height: 20,
                    child: SpinKitFadingCircle(color: Colors.white, size: 20),
                  )
                : const Icon(Icons.lock_outline),
            label: Text(
              _isLoading ? 'Creating...' : 'Create Deal & Lock Funds',
            ),
            style: ElevatedButton.styleFrom(
              padding: const EdgeInsets.symmetric(vertical: 16),
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildActionsTab(ColorScheme colorScheme) {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _buildBanner(
            title: 'Deal Actions',
            subtitle:
                'Use the buyer address to operate on the latest escrow deal.',
          ),
          const SizedBox(height: 16),
          _buildFeedback(colorScheme),
          TextFormField(
            controller: _buyerAddressController,
            decoration: const InputDecoration(
              labelText: 'Buyer Address',
              hintText: '0x...',
              prefixIcon: Icon(Icons.account_balance_wallet),
            ),
          ),
          const SizedBox(height: 24),
          Text(
            'Seller Actions',
            style: Theme.of(
              context,
            ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
          ),
          const SizedBox(height: 8),
          OutlinedButton.icon(
            onPressed: _isLoading ? null : _confirmDelivery,
            icon: _buildInlineLoader(colorScheme.primary, Icons.local_shipping),
            label: Text(_isLoading ? 'Processing...' : 'Confirm Delivery'),
          ),
          const SizedBox(height: 24),
          Text(
            'Buyer Actions',
            style: Theme.of(
              context,
            ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.bold),
          ),
          const SizedBox(height: 8),
          ElevatedButton.icon(
            onPressed: _isLoading ? null : _releaseFunds,
            icon: _buildInlineLoader(Colors.white, Icons.payment),
            label: Text(_isLoading ? 'Processing...' : 'Release Funds'),
          ),
          const SizedBox(height: 12),
          OutlinedButton.icon(
            onPressed: _isLoading ? null : _raiseDispute,
            icon: _buildInlineLoader(colorScheme.error, Icons.gavel),
            label: Text(_isLoading ? 'Processing...' : 'Raise Dispute'),
            style: OutlinedButton.styleFrom(foregroundColor: colorScheme.error),
          ),
        ],
      ),
    );
  }

  Widget _buildCheckTab(ColorScheme colorScheme) {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _buildBanner(
            title: 'Check Deal Status',
            subtitle: 'Inspect the latest deal owned by a buyer address.',
          ),
          const SizedBox(height: 16),
          _buildFeedback(colorScheme),
          TextFormField(
            controller: _buyerAddressController,
            decoration: const InputDecoration(
              labelText: 'Buyer Address',
              hintText: '0x...',
              prefixIcon: Icon(Icons.search),
            ),
          ),
          const SizedBox(height: 16),
          ElevatedButton.icon(
            onPressed: _isLoading ? null : _checkDealState,
            icon: _buildInlineLoader(Colors.white, Icons.search),
            label: Text(_isLoading ? 'Checking...' : 'Check Status'),
          ),
          const SizedBox(height: 24),
          if (_currentDealState != null)
            Card(
              color: _stateColor(_currentDealState!).withOpacity(0.12),
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  children: [
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        const Text(
                          'Current State',
                          style: TextStyle(fontWeight: FontWeight.bold),
                        ),
                        Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 12,
                            vertical: 6,
                          ),
                          decoration: BoxDecoration(
                            color: _stateColor(_currentDealState!),
                            borderRadius: BorderRadius.circular(999),
                          ),
                          child: Text(
                            _stateName(_currentDealState!),
                            style: const TextStyle(
                              color: Colors.white,
                              fontWeight: FontWeight.bold,
                            ),
                          ),
                        ),
                      ],
                    ),
                    if (_proofCount != null) ...[
                      const Divider(height: 24),
                      Row(
                        mainAxisAlignment: MainAxisAlignment.spaceBetween,
                        children: [
                          const Text('Proof Entries'),
                          Text(
                            '$_proofCount',
                            style: Theme.of(context).textTheme.titleMedium,
                          ),
                        ],
                      ),
                    ],
                  ],
                ),
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
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              title,
              style: Theme.of(
                context,
              ).textTheme.titleLarge?.copyWith(fontWeight: FontWeight.bold),
            ),
            const SizedBox(height: 8),
            Text(subtitle, style: Theme.of(context).textTheme.bodySmall),
            if (child != null) child,
          ],
        ),
      ),
    );
  }

  Widget _buildFeedback(ColorScheme colorScheme) {
    if (_errorMessage == null && _successMessage == null) {
      return const SizedBox.shrink();
    }

    final isError = _errorMessage != null;
    final message = isError ? _errorMessage! : _successMessage!;
    final color = isError ? colorScheme.error : Colors.green;

    return Container(
      margin: const EdgeInsets.only(bottom: 16),
      padding: const EdgeInsets.all(12),
      decoration: BoxDecoration(
        color: color.withOpacity(0.1),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: color.withOpacity(0.3)),
      ),
      child: Row(
        children: [
          Icon(
            isError ? Icons.error_outline : Icons.check_circle_outline,
            color: color,
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Text(message, style: TextStyle(color: color)),
          ),
          IconButton(
            onPressed: () {
              setState(() {
                _errorMessage = null;
                _successMessage = null;
              });
            },
            icon: Icon(Icons.close, color: color),
          ),
        ],
      ),
    );
  }

  Widget _buildInlineLoader(Color color, IconData fallbackIcon) {
    if (_isLoading) {
      return SizedBox(
        width: 20,
        height: 20,
        child: SpinKitFadingCircle(color: color, size: 20),
      );
    }
    return Icon(fallbackIcon);
  }
}
