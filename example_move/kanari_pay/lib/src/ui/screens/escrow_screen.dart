import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter_spinkit/flutter_spinkit.dart';
import 'package:provider/provider.dart';

import '../../client/escrow_client.dart';
import '../../models/account.dart';
import '../../providers/wallet_provider.dart';
import '../widgets/app_ui.dart';
import '../widgets/escrow_widgets.dart';

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
  late ScrollController _scrollController;

  final _dealIdController = TextEditingController();
  final _sellerAddressController = TextEditingController();
  final _amountController = TextEditingController();
  final _descriptionController = TextEditingController();
  final _buyerAddressController = TextEditingController();
  final _disputeReasonController = TextEditingController();

  bool _isLoading = false;
  bool _isLoadingTokens = false;
  String? _loadedWalletAddress;
  String? _selectedTokenType;
  String? _errorMessage;
  String? _successMessage;
  List<String> _spendableCoinTypes = const [];
  int? _currentDealState;
  Map<String, dynamic>? _dealDetails;

  // สำหรับจัดการ multiple deals
  List<Map<String, dynamic>> _allDeals = [];
  String? _selectedDealId;
  Map<String, dynamic>? _selectedDeal;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 3, vsync: this);
    _scrollController = ScrollController();
    _dealIdController.text = _generateDealId();
  }

  @override
  void dispose() {
    _tabController.dispose();
    _scrollController.dispose();
    _dealIdController.dispose();
    _sellerAddressController.dispose();
    _amountController.dispose();
    _descriptionController.dispose();
    _buyerAddressController.dispose();
    _disputeReasonController.dispose();
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

  /// Convert human-readable amount to raw amount based on decimals
  int _toRawAmount(String humanAmount, int decimals) {
    try {
      final amount = double.parse(humanAmount);
      return (amount * math.pow(10, decimals)).toInt();
    } catch (e) {
      return 0;
    }
  }

  /// Convert raw amount to human-readable amount based on decimals
  String _toHumanAmount(int rawAmount, int decimals) {
    if (rawAmount == 0) return '0';
    final divisor = math.pow(10, decimals).toDouble();
    return (rawAmount / divisor).toStringAsFixed(6);
  }

  /// Get decimals for a token type
  int _getDecimalsForTokenType(String tokenType) {
    // USDC and USDT typically use 6 decimals
    if (tokenType.contains('USDC') || tokenType.contains('USDT')) {
      return 6;
    }
    // KANARI typically uses 9 decimals
    if (tokenType.contains('KANARI')) {
      return 9;
    }
    // Default to 6 decimals for other tokens
    return 6;
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
      final buyerAddress = wallet.address;
      final sellerAddress = _sellerAddressController.text.trim();

      if (_dealIdController.text.trim().isEmpty) {
        throw Exception('Deal ID is required');
      }
      if (sellerAddress.isEmpty) {
        throw Exception('Seller address is required');
      }
      if (_amountController.text.trim().isEmpty) {
        throw Exception('Amount is required');
      }
      if (tokenType == null || tokenType.isEmpty) {
        throw Exception('Escrow token is required');
      }

      // ตรวจสอบว่า buyer ≠ seller
      if (buyerAddress.toLowerCase() == sellerAddress.toLowerCase()) {
        throw Exception(
          'Buyer and seller cannot be the same address. '
          'Please enter a different seller address.',
        );
      }

      // Convert human-readable amount to raw amount based on token decimals
      final decimals = _getDecimalsForTokenType(tokenType);
      final rawAmount = _toRawAmount(_amountController.text.trim(), decimals);

      if (rawAmount <= 0) {
        throw Exception('Invalid amount. Please enter a valid number.');
      }

      print('[ESCROW UI] Creating deal:');
      print('[ESCROW UI]   Human amount: ${_amountController.text.trim()}');
      print('[ESCROW UI]   Token: $tokenType');
      print('[ESCROW UI]   Decimals: $decimals');
      print('[ESCROW UI]   Raw amount: $rawAmount');

      final result = await escrow
          .createDeal(
            wallet: wallet,
            dealId: _dealIdController.text.trim(),
            sellerAddress: sellerAddress,
            amount: rawAmount,
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

  /// Seller: Confirm delivery
  Future<void> _confirmDelivery() async {
    if (_selectedDeal == null) {
      setState(() {
        _errorMessage = 'Please select a deal first';
      });
      return;
    }

    final objectId = _selectedDeal!['object_id'] as String?;
    final coinType = _selectedDeal!['coin_type'] as String?;
    final proofId = _selectedDeal!['proof_id'] as String?;

    if (objectId == null || coinType == null) {
      setState(() {
        _errorMessage = 'Missing deal information';
      });
      return;
    }

    if (proofId == null) {
      setState(() {
        _errorMessage = 'Proof object not found for this deal';
      });
      return;
    }

    await _runAction((escrow, walletState) async {
      await escrow
          .confirmDeliveryByObjectId(
            wallet: walletState.wallet!,
            dealObjectId: objectId,
            coinType: coinType,
            proofObjectId: proofId,
          )
          .timeout(const Duration(seconds: 30));
    });
  }

  /// Buyer: Release funds
  Future<void> _releaseFunds() async {
    if (_selectedDeal == null) {
      setState(() {
        _errorMessage = 'Please select a deal first';
      });
      return;
    }

    final objectId = _selectedDeal!['object_id'] as String?;
    final coinType = _selectedDeal!['coin_type'] as String?;
    final proofId = _selectedDeal!['proof_id'] as String?;

    if (objectId == null || coinType == null) {
      setState(() {
        _errorMessage = 'Missing deal information';
      });
      return;
    }

    if (proofId == null) {
      setState(() {
        _errorMessage = 'Proof object not found for this deal';
      });
      return;
    }

    await _runAction((escrow, walletState) async {
      await escrow
          .releaseFundsByObjectId(
            wallet: walletState.wallet!,
            dealObjectId: objectId,
            coinType: coinType,
            proofObjectId: proofId,
          )
          .timeout(const Duration(seconds: 30));
    });
  }

  /// Buyer or Seller: Raise dispute
  Future<void> _raiseDispute() async {
    if (_selectedDeal == null) {
      setState(() {
        _errorMessage = 'Please select a deal first';
      });
      return;
    }

    final objectId = _selectedDeal!['object_id'] as String?;
    final coinType = _selectedDeal!['coin_type'] as String?;
    final proofId = _selectedDeal!['proof_id'] as String?;

    if (objectId == null || coinType == null) {
      setState(() {
        _errorMessage = 'Missing deal information';
      });
      return;
    }

    if (proofId == null) {
      setState(() {
        _errorMessage = 'Proof object not found for this deal';
      });
      return;
    }

    await _runAction((escrow, walletState) async {
      await escrow
          .raiseDisputeByObjectId(
            wallet: walletState.wallet!,
            dealObjectId: objectId,
            coinType: coinType,
            proofObjectId: proofId,
            reason: _disputeReasonController.text.trim().isEmpty
                ? 'No reason provided'
                : _disputeReasonController.text.trim(),
          )
          .timeout(const Duration(seconds: 30));
    });
  }

  Future<void> _checkDealState() async {
    await _runAction((escrow, walletState) async {
      final wallet = walletState.wallet!;
      final buyer = _buyerAddressController.text.trim();
      if (buyer.isEmpty) {
        throw Exception('Buyer address is required');
      }

      //  FIXED: Use selected deal if available, otherwise fetch latest
      if (_selectedDeal != null) {
        // User selected a specific deal from dropdown
        final objectId = _selectedDeal!['object_id'] as String?;
        final coinType = _selectedDeal!['coin_type'] as String?;
        final dealId = _selectedDeal!['deal_id'] as String?;

        if (objectId == null || coinType == null || dealId == null) {
          throw Exception('Invalid selected deal data');
        }

        print('[ESCROW] Loading selected deal: $dealId');

        final state = await escrow
            .getDealStateByObjectId(
              wallet: wallet,
              dealObjectId: objectId,
              coinType: coinType,
            )
            .timeout(const Duration(seconds: 30));

        // Parse details from selected deal
        final rawAmount = _selectedDeal!['amount'] as int? ?? 0;
        final buyerAddr = _selectedDeal!['buyer'] as String? ?? buyer;
        final sellerAddr = _selectedDeal!['seller'] as String? ?? 'N/A';

        final details = {
          'deal_id': dealId,
          'buyer': buyerAddr,
          'seller': sellerAddr,
          'amount': rawAmount,
          'coin_type': coinType,
        };

        if (!mounted) return;
        setState(() {
          _currentDealState = state;
          _dealDetails = details;
          _successMessage = 'Deal state retrieved successfully!';
        });
      } else {
        // Fallback: Get latest deal for buyer address
        print('[ESCROW] No deal selected, fetching latest for: $buyer');

        final state = await escrow
            .getDealState(wallet: wallet, buyerAddress: buyer)
            .timeout(const Duration(seconds: 30));

        final details = await escrow
            .getDealDetails(wallet: wallet, buyerAddress: buyer)
            .timeout(const Duration(seconds: 30));

        if (!mounted) return;
        setState(() {
          _currentDealState = state;
          _dealDetails = details;
          _successMessage = 'Deal state retrieved successfully!';
        });
      }
    });
  }

  /// Auto-check state when buyer address changes
  Future<void> _autoCheckDealState() async {
    final buyer = _buyerAddressController.text.trim();
    if (buyer.isEmpty) {
      setState(() {
        _currentDealState = null;
        _dealDetails = null;
        _allDeals = [];
        _selectedDealId = null;
        _selectedDeal = null;
      });
      return;
    }

    await _fetchAllDeals(buyer);
  }

  /// Fetch all deals for a buyer address
  Future<void> _fetchAllDeals(String buyerAddress) async {
    final walletState = context.read<WalletState>();
    final escrow = _escrowClient(walletState);
    if (walletState.wallet == null || escrow == null) return;

    try {
      final deals = await escrow
          .getAllDeals(wallet: walletState.wallet!, buyerAddress: buyerAddress)
          .timeout(const Duration(seconds: 15));

      if (!mounted) return;
      setState(() {
        _allDeals = deals;
        _selectedDealId = deals.isNotEmpty ? deals.first['deal_id'] : null;
        _selectedDeal = deals.isNotEmpty ? deals.first : null;
        _currentDealState = null;
        _dealDetails = null;
      });

      if (deals.isNotEmpty) {
        await _loadSelectedDeal();
      }
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _allDeals = [];
        _selectedDealId = null;
        _selectedDeal = null;
        _currentDealState = null;
        _dealDetails = null;
        _errorMessage = 'Failed to fetch deals: $e';
      });
    }
  }

  /// Load details for selected deal
  Future<void> _loadSelectedDeal() async {
    if (_selectedDeal == null) return;

    final walletState = context.read<WalletState>();
    final escrow = _escrowClient(walletState);
    if (walletState.wallet == null || escrow == null) return;

    try {
      final objectId = _selectedDeal!['object_id'] as String?;
      final coinType = _selectedDeal!['coin_type'] as String?;

      if (objectId == null || coinType == null) {
        throw Exception('Missing object_id or coin_type in deal data');
      }

      print('[ESCROW] Loading selected deal: $objectId');

      //  FIXED: Only query state, details already in _selectedDeal
      final state = await escrow
          .getDealStateByObjectId(
            wallet: walletState.wallet!,
            dealObjectId: objectId,
            coinType: coinType,
          )
          .timeout(const Duration(seconds: 10));

      // Extract details from selectedDeal (already have from getAllDeals)
      final dealId = _selectedDeal!['deal_id'] as String? ?? objectId;
      final buyer = _selectedDeal!['buyer'] as String? ?? 'N/A';
      final seller = _selectedDeal!['seller'] as String? ?? 'N/A';
      final amount = _selectedDeal!['amount'] as int? ?? 0;

      final details = {
        'deal_id': dealId,
        'buyer': buyer,
        'seller': seller,
        'amount': amount,
        'coin_type': coinType,
      };

      print('[ESCROW] Deal loaded successfully, state: $state');

      if (!mounted) return;
      setState(() {
        _currentDealState = state;
        _dealDetails = details;
        _errorMessage = null;
      });
    } catch (e, stack) {
      print('[ESCROW] Failed to load selected deal: $e');
      print('[ESCROW] Stack: $stack');
      if (!mounted) return;
      setState(() {
        _currentDealState = null;
        _dealDetails = null;
        _errorMessage = 'Failed to load deal: $e';
      });
    }
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
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Scaffold(
      backgroundColor: colorScheme.surface,
      body: _isLoadingTokens
          ? Center(child: SpinKitFadingCircle(color: colorScheme.primary))
          : RefreshIndicator(
              onRefresh: _loadSpendableCoinTypes,
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
                          Tab(icon: Icon(Icons.add_business), text: 'Create'),
                          Tab(icon: Icon(Icons.list), text: 'Deals'),
                          Tab(icon: Icon(Icons.history), text: 'History'),
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
                        _buildCreateDealTab(),
                        _buildManageDealsTab(),
                        _buildHistoryTab(),
                      ],
                    ),
                  ),
                ],
              ),
            ),
    );
  }

  Widget _buildCreateDealTab() {
    final walletState = context.watch<WalletState>();
    final colorScheme = Theme.of(context).colorScheme;

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

          // ใช้ reusable widgets
          _buildDealIdInput(),

          const SizedBox(height: 12),

          _buildAddressInput(
            controller: _sellerAddressController,
            label: 'Seller Address',
            hintText: '0x...',
            prefixIcon: Icons.person,
            onAutofill: _autofillSellerAddress,
            helperText: 'Address of the seller',
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
          // ใช้ AppWideButton แทน ElevatedButton
          _buildPrimaryButton(
            onPressed: _createDeal,
            icon: Icons.lock_outline,
            label: 'Create Deal & Lock Funds',
            isLoading: _isLoading || spendableOptions.isEmpty,
          ),
        ],
      ),
    );
  }

  Widget _buildManageDealsTab() {
    final colorScheme = Theme.of(context).colorScheme;

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

          // แสดง current deal state ถ้ามี
          if (_currentDealState != null) ...[
            AppPanel(
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Text(
                    'Current State:',
                    style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                      color: colorScheme.onSurface.withOpacity(0.7),
                    ),
                  ),
                  _buildStateBadge(_currentDealState!),
                ],
              ),
            ),
            const SizedBox(height: 16),
          ],

          // เพิ่ม dropdown สำหรับเลือก Deal ID ถ้ามีหลาย deal
          if (_allDeals.length > 1) ...[
            AppSectionTitle('Select Deal'),
            const SizedBox(height: 8),
            DropdownButtonFormField<String>(
              value: _selectedDealId,
              decoration: const InputDecoration(
                labelText: 'Deal ID',
                border: OutlineInputBorder(),
              ),
              items: _allDeals.map((deal) {
                final dealId = deal['deal_id'] as String? ?? 'Unknown';
                final rawAmount = deal['amount'] as int? ?? 0;
                final coinType = deal['coin_type'] as String? ?? '';
                final coinName = coinType.split('::').lastOrNull ?? '';

                // Convert raw amount to human-readable
                final decimals = _getDecimalsForTokenType(coinType);
                final humanAmount = _toHumanAmount(rawAmount, decimals);

                return DropdownMenuItem<String>(
                  value: dealId,
                  child: Text('$dealId ($humanAmount $coinName)'),
                );
              }).toList(),
              onChanged: (value) {
                if (value != null) {
                  setState(() {
                    _selectedDealId = value;
                    _selectedDeal = _allDeals.firstWhere(
                      (deal) => deal['deal_id'] == value,
                      orElse: () => {},
                    );
                  });
                  _loadSelectedDeal();
                }
              },
            ),
            const SizedBox(height: 16),
          ],

          // ใช้ reusable widget สำหรับ buyer address
          _buildAddressInput(
            controller: _buyerAddressController,
            label: 'Buyer Address',
            hintText: '0x...',
            prefixIcon: Icons.account_balance_wallet,
            onAutofill: () {
              _autofillBuyerAddress();
              _autoCheckDealState();
            },
            helperText: 'Address of the buyer who created the deal',
            onChanged: _autoCheckDealState,
          ),
          const SizedBox(height: 24),
          AppSectionTitle('Seller Actions'),
          const SizedBox(height: 8),
          _buildOutlinedButton(
            onPressed: _currentDealState == 1 ? _confirmDelivery : null,
            icon: Icons.local_shipping,
            label: _currentDealState == 1
                ? 'Confirm Delivery'
                : 'Deal is not in LOCKED state',
            color: colorScheme.primary,
          ),
          const SizedBox(height: 24),
          AppSectionTitle('Buyer Actions'),
          const SizedBox(height: 8),
          _buildPrimaryButton(
            onPressed: _currentDealState == 2 ? _releaseFunds : null,
            icon: Icons.payment,
            label: _currentDealState == 2
                ? 'Release Funds'
                : 'Deal is not in DELIVERED state',
            isLoading: false,
          ),
          const SizedBox(height: 12),
          // เพิ่ม input field สำหรับเหตุผล dispute
          TextFormField(
            controller: _disputeReasonController,
            maxLines: 2,
            decoration: const InputDecoration(
              labelText: 'Dispute Reason',
              hintText: 'Explain why you are raising a dispute',
              prefixIcon: Icon(Icons.gavel),
            ),
          ),
          const SizedBox(height: 12),
          _buildOutlinedButton(
            onPressed: (_currentDealState == null || _currentDealState == 3)
                ? null
                : _raiseDispute,
            icon: Icons.gavel,
            label: (_currentDealState == null || _currentDealState == 3)
                ? 'Deal is completed or not found'
                : 'Raise Dispute',
            color: colorScheme.error,
          ),
        ],
      ),
    );
  }

  Widget _buildHistoryTab() {
    final colorScheme = Theme.of(context).colorScheme;

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
          _buildAddressInput(
            controller: _buyerAddressController,
            label: 'Buyer Address',
            hintText: '0x...',
            prefixIcon: Icons.search,
            onAutofill: () {
              _autofillBuyerAddress();
              _autoCheckDealState();
            },
            helperText: 'Address to check deal status',
            onChanged: _autoCheckDealState,
          ),
          const SizedBox(height: 16),
          _buildPrimaryButton(
            onPressed: _checkDealState,
            icon: Icons.search,
            label: 'Check Status',
          ),
          const SizedBox(height: 24),

          // 🔥 NEW: แสดง Deal List ทั้งหมดเป็น card
          if (_allDeals.isNotEmpty) ...[
            AppSectionTitle('All Deals (${_allDeals.length})'),
            const SizedBox(height: 12),
            ListView.builder(
              shrinkWrap: true,
              physics: const NeverScrollableScrollPhysics(),
              itemCount: _allDeals.length,
              itemBuilder: (context, index) {
                final deal = _allDeals[index];
                final dealId = deal['deal_id'] as String? ?? 'N/A';
                final isSelected = _selectedDealId == dealId;

                return DealCard(
                  deal: deal,
                  isSelected: isSelected,
                  colorScheme: colorScheme,
                  onTap: () {
                    setState(() {
                      _selectedDealId = dealId;
                      _selectedDeal = deal;
                    });
                    _checkDealState();
                  },
                );
              },
            ),
            const SizedBox(height: 24),
          ],

          // แสดงสถานะ deal
          if (_currentDealState != null)
            AppPanel(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      AppSectionTitle('Current State'),
                      _buildStateBadge(_currentDealState!),
                    ],
                  ),
                  if (_dealDetails != null && _dealDetails!.isNotEmpty) ...[
                    const Divider(height: 24),
                    _buildDetailRow(
                      'Deal ID',
                      _dealDetails!['deal_id'] ?? 'N/A',
                    ),
                    const SizedBox(height: 8),
                    _buildDetailRow('Buyer', _dealDetails!['buyer'] ?? 'N/A'),
                    const SizedBox(height: 8),
                    _buildDetailRow('Seller', _dealDetails!['seller'] ?? 'N/A'),
                    const SizedBox(height: 8),
                    _buildDetailRow(
                      'Amount',
                      _dealDetails!['amount'] != null
                          ? '${_toHumanAmount(_dealDetails!['amount'] as int, _getDecimalsForTokenType(_dealDetails!['coin_type'] ?? ''))} ${(_dealDetails!['coin_type'] as String? ?? '').split('::').lastOrNull ?? 'units'}'
                          : 'N/A',
                    ),
                  ],
                ],
              ),
            ),
          const SizedBox(height: 16),
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
                  onPressed: () {
                    setState(() {
                      _errorMessage = null;
                      _successMessage = null;
                    });
                  },
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
    Color? color,
    bool isLoading = false,
  }) {
    return AppWideButton(
      onPressed: (isLoading || onPressed == null) ? null : onPressed,
      icon: icon,
      label: isLoading ? 'Processing...' : label,
      style: AppWideButtonStyle.outlined,
    );
  }

  /// สร้าง Deal ID อัตโนมัติในรูปแบบ deal-{timestamp}-{random}
  String _generateDealId() {
    final timestamp = DateTime.now().millisecondsSinceEpoch;
    final random = DateTime.now().microsecondsSinceEpoch % 10000;
    return 'deal-${timestamp}-${random.toString().padLeft(4, '0')}';
  }

  /// ดึง wallet address ปัจจุบัน
  String? _getCurrentWalletAddress() {
    try {
      final walletState = context.read<WalletState>();
      return walletState.wallet?.address;
    } catch (_) {
      return null;
    }
  }

  /// เติม Seller Address ด้วย wallet address ปัจจุบัน
  void _autofillSellerAddress() {
    final address = _getCurrentWalletAddress();
    if (address != null) {
      setState(() {
        _sellerAddressController.text = address;
      });
    }
  }

  /// เติม Buyer Address ด้วย wallet address ปัจจุบัน
  void _autofillBuyerAddress() {
    final address = _getCurrentWalletAddress();
    if (address != null) {
      setState(() {
        _buyerAddressController.text = address;
      });
    }
  }

  /// รีเฟรช Deal ID ใหม่
  void _refreshDealId() {
    setState(() {
      _dealIdController.text = _generateDealId();
    });
  }

  /// Widget สำหรับ Address input พร้อมปุ่ม autofill
  Widget _buildAddressInput({
    required TextEditingController controller,
    required String label,
    required String hintText,
    required IconData prefixIcon,
    required VoidCallback onAutofill,
    String? helperText,
    VoidCallback? onChanged,
  }) {
    return Row(
      children: [
        Expanded(
          child: TextFormField(
            controller: controller,
            onChanged: (value) {
              if (onChanged != null) {
                onChanged();
              }
            },
            decoration: InputDecoration(
              labelText: label,
              hintText: hintText,
              prefixIcon: Icon(prefixIcon, size: 20),
              prefixIconConstraints: const BoxConstraints(
                minWidth: 40,
                minHeight: 40,
              ),
              helperText: helperText,
              contentPadding: const EdgeInsets.symmetric(
                horizontal: 12,
                vertical: 16,
              ),
            ),
          ),
        ),
        const SizedBox(width: 4),
        SizedBox(
          width: 40,
          height: 40,
          child: IconButton(
            onPressed: _isLoading ? null : onAutofill,
            icon: const Icon(Icons.person_pin, size: 20),
            tooltip: 'Use my wallet address',
            padding: EdgeInsets.zero,
          ),
        ),
      ],
    );
  }

  /// Widget สำหรับ Deal ID input พร้อมปุ่ม refresh
  Widget _buildDealIdInput() {
    return Row(
      children: [
        Expanded(
          child: TextFormField(
            controller: _dealIdController,
            decoration: const InputDecoration(
              labelText: 'Deal ID',
              hintText: 'Auto-generated',
              prefixIcon: Icon(Icons.tag, size: 20),
              prefixIconConstraints: BoxConstraints(
                minWidth: 40,
                minHeight: 40,
              ),
              helperText: 'Auto-generated, but you can edit it',
              contentPadding: EdgeInsets.symmetric(
                horizontal: 12,
                vertical: 16,
              ),
            ),
          ),
        ),
        const SizedBox(width: 4),
        SizedBox(
          width: 40,
          height: 40,
          child: IconButton(
            onPressed: _isLoading ? null : _refreshDealId,
            icon: const Icon(Icons.refresh, size: 20),
            tooltip: 'Generate new Deal ID',
            padding: EdgeInsets.zero,
          ),
        ),
      ],
    );
  }

  /// Widget สำหรับแสดง state badge
  Widget _buildStateBadge(int state) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
      decoration: BoxDecoration(
        color: _stateColor(state),
        borderRadius: BorderRadius.circular(999),
      ),
      child: Text(
        _stateName(state),
        style: const TextStyle(
          color: Colors.white,
          fontWeight: FontWeight.bold,
        ),
      ),
    );
  }

  /// Widget สำหรับ detail row
  Widget _buildDetailRow(String label, String value) {
    return Row(
      children: [
        Text(
          '$label: ',
          style: const TextStyle(
            fontWeight: FontWeight.w500,
            color: Colors.grey,
          ),
        ),
        Expanded(
          child: Text(
            value,
            style: const TextStyle(fontWeight: FontWeight.bold),
            overflow: TextOverflow.ellipsis,
          ),
        ),
      ],
    );
  }
}
