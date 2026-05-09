import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../escrow_client.dart';
import '../../models/account.dart';
import '../../providers/wallet_provider.dart';
import '../widgets/app_ui.dart'; // ✅ ใช้ shared widgets

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
  final _disputeReasonController =
      TextEditingController(); // เพิ่ม controller สำหรับเหตุผล dispute

  bool _isLoading = false;
  bool _isLoadingTokens = false;
  String? _loadedWalletAddress;
  String? _selectedTokenType;
  String? _errorMessage;
  String? _successMessage;
  List<String> _spendableCoinTypes = const [];
  int? _currentDealState;
  Map<String, dynamic>? _dealDetails; // เพิ่มสำหรับเก็บรายละเอียด deal
  bool? _stateMatchResult; // เพิ่มสำหรับเก็บผล check_deal_state

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 3, vsync: this);

    // สร้าง Deal ID อัตโนมัติเมื่อเริ่มหน้าจอ
    _dealIdController.text = _generateDealId();
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
      if (_disputeReasonController.text.trim().isEmpty) {
        throw Exception('Dispute reason is required');
      }

      final result = await escrow
          .raiseDispute(
            wallet: wallet,
            buyerAddress: buyer,
            reason: _disputeReasonController.text
                .trim(), // ส่ง reason parameter
          )
          .timeout(const Duration(seconds: 30));
      await walletState.refreshBalance();
      if (!mounted) return;
      setState(() {
        _successMessage =
            'Dispute raised! Tx Hash: ${result.hash.substring(0, 16)}...';
      });
      _disputeReasonController.clear(); // เคลียร์หลังจากสำเร็จ
    });
  }

  Future<void> _checkDealState() async {
    await _runAction((escrow, walletState) async {
      final wallet = walletState.wallet!;
      final buyer = _buyerAddressController.text.trim();
      if (buyer.isEmpty) {
        throw Exception('Buyer address is required');
      }

      // ใช้ view functions ใหม่ที่ efficient กว่า
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
    });
  }

  /// Check if deal matches expected state (new feature)
  Future<void> _verifyDealState(int expectedState) async {
    await _runAction((escrow, walletState) async {
      final wallet = walletState.wallet!;
      final buyer = _buyerAddressController.text.trim();
      if (buyer.isEmpty) {
        throw Exception('Buyer address is required');
      }

      final isMatch = await escrow
          .checkDealState(
            wallet: wallet,
            buyerAddress: buyer,
            expectedState: expectedState,
          )
          .timeout(const Duration(seconds: 30));

      if (!mounted) return;
      setState(() {
        _stateMatchResult = isMatch;
        _successMessage = isMatch
            ? '✅ Deal is in ${_stateName(expectedState)} state'
            : '❌ Deal is NOT in ${_stateName(expectedState)} state';
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

    return AppGradientScaffold(
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

          // ใช้ reusable widget สำหรับ buyer address
          _buildAddressInput(
            controller: _buyerAddressController,
            label: 'Buyer Address',
            hintText: '0x...',
            prefixIcon: Icons.account_balance_wallet,
            onAutofill: _autofillBuyerAddress,
            helperText: 'Address of the buyer who created the deal',
          ),
          const SizedBox(height: 24),
          AppSectionTitle('Seller Actions'),
          const SizedBox(height: 8),
          _buildOutlinedButton(
            onPressed: _confirmDelivery,
            icon: Icons.local_shipping,
            label: 'Confirm Delivery',
            color: colorScheme.primary,
          ),
          const SizedBox(height: 24),
          AppSectionTitle('Buyer Actions'),
          const SizedBox(height: 8),
          _buildPrimaryButton(
            onPressed: _releaseFunds,
            icon: Icons.payment,
            label: 'Release Funds',
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
            onPressed: _raiseDispute,
            icon: Icons.gavel,
            label: 'Raise Dispute',
            color: colorScheme.error,
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
          _buildAddressInput(
            controller: _buyerAddressController,
            label: 'Buyer Address',
            hintText: '0x...',
            prefixIcon: Icons.search,
            onAutofill: _autofillBuyerAddress,
            helperText: 'Address to check deal status',
          ),
          const SizedBox(height: 16),
          _buildPrimaryButton(
            onPressed: _checkDealState,
            icon: Icons.search,
            label: 'Check Status',
          ),
          const SizedBox(height: 24),

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
                    _buildDetailRow('Buyer', _dealDetails!['buyer'] ?? 'N/A'),
                    const SizedBox(height: 8),
                    _buildDetailRow('Seller', _dealDetails!['seller'] ?? 'N/A'),
                    const SizedBox(height: 8),
                    _buildDetailRow(
                      'Amount',
                      '${_dealDetails!['amount']} units',
                    ),
                  ],
                ],
              ),
            ),
          const SizedBox(height: 16),

          // เพิ่มปุ่มตรวจสอบสถานะเฉพาะ
          if (_currentDealState != null) ...[
            const SizedBox(height: 16),
            AppSectionTitle('Quick Verification'),
            const SizedBox(height: 8),

            // ใช้ reusable widgets สำหรับ verification buttons
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                _buildVerifyButton(
                  label: 'Is Locked?',
                  state: 1,
                  color: Colors.orange,
                  icon: Icons.lock_outline,
                ),
                _buildVerifyButton(
                  label: 'Is Delivered?',
                  state: 2,
                  color: Colors.blue,
                  icon: Icons.local_shipping,
                ),
                _buildVerifyButton(
                  label: 'Is Completed?',
                  state: 3,
                  color: Colors.green,
                  icon: Icons.check_circle_outline,
                ),
                _buildVerifyButton(
                  label: 'Is Disputed?',
                  state: 4,
                  color: Colors.red,
                  icon: Icons.gavel,
                ),
              ],
            ),

            // แสดงผลการ verify
            if (_stateMatchResult != null) ...[
              const SizedBox(height: 16),
              Container(
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: _stateMatchResult!
                      ? Colors.green.withOpacity(0.1)
                      : Colors.red.withOpacity(0.1),
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(
                    color: _stateMatchResult!
                        ? Colors.green.withOpacity(0.3)
                        : Colors.red.withOpacity(0.3),
                  ),
                ),
                child: Row(
                  children: [
                    Icon(
                      _stateMatchResult! ? Icons.check_circle : Icons.cancel,
                      color: _stateMatchResult! ? Colors.green : Colors.red,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        _stateMatchResult!
                            ? '✅ Verified: Deal is in expected state'
                            : '❌ Not verified: Deal is in different state',
                        style: TextStyle(
                          color: _stateMatchResult! ? Colors.green : Colors.red,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ],
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
    required VoidCallback onPressed,
    required IconData icon,
    required String label,
    bool isLoading = false,
  }) {
    return AppWideButton(
      onPressed: isLoading ? null : onPressed,
      icon: icon,
      label: isLoading ? 'Processing...' : label,
      style: AppWideButtonStyle.primary,
    );
  }

  Widget _buildOutlinedButton({
    required VoidCallback onPressed,
    required IconData icon,
    required String label,
    Color? color,
    bool isLoading = false,
  }) {
    return AppWideButton(
      onPressed: isLoading ? null : onPressed,
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
  }) {
    return Row(
      children: [
        Expanded(
          child: TextFormField(
            controller: controller,
            decoration: InputDecoration(
              labelText: label,
              hintText: hintText,
              prefixIcon: Icon(prefixIcon),
              helperText: helperText,
            ),
          ),
        ),
        const SizedBox(width: 8),
        IconButton(
          onPressed: _isLoading ? null : onAutofill,
          icon: const Icon(Icons.person_pin),
          tooltip: 'Use my wallet address',
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
              prefixIcon: Icon(Icons.tag),
              helperText: 'Auto-generated, but you can edit it',
            ),
          ),
        ),
        const SizedBox(width: 8),
        IconButton(
          onPressed: _isLoading ? null : _refreshDealId,
          icon: const Icon(Icons.refresh),
          tooltip: 'Generate new Deal ID',
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
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Text(
          label,
          style: const TextStyle(
            fontWeight: FontWeight.w500,
            color: Colors.grey,
          ),
        ),
        Flexible(
          child: Text(
            value,
            style: const TextStyle(fontWeight: FontWeight.bold),
            overflow: TextOverflow.ellipsis,
          ),
        ),
      ],
    );
  }

  /// Widget สำหรับ verification button
  Widget _buildVerifyButton({
    required String label,
    required int state,
    required Color color,
    required IconData icon,
  }) {
    return OutlinedButton.icon(
      onPressed: _isLoading ? null : () => _verifyDealState(state),
      icon: Icon(icon, size: 18),
      label: Text(label),
      style: OutlinedButton.styleFrom(
        foregroundColor: color,
        side: BorderSide(color: color.withOpacity(0.5)),
      ),
    );
  }
}
