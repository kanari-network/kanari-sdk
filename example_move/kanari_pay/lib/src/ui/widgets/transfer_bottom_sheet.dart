import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:mobile_scanner/mobile_scanner.dart';
import 'package:provider/provider.dart';

import '../../providers/wallet_provider.dart';
import '../../models/account.dart';
import 'token_logo.dart';

/// Transfer Bottom Sheet - UI สำหรับโอนเงิน
/// แยกออกมาจาก home_screen.dart เพื่อลดขนาดไฟล์และทำให้โค้ดเป็นระเบียบ
class TransferBottomSheet extends StatefulWidget {
  final String? prefilledAddress;

  const TransferBottomSheet({super.key, this.prefilledAddress});

  @override
  State<TransferBottomSheet> createState() => _TransferBottomSheetState();
}

class _TransferBottomSheetState extends State<TransferBottomSheet> {
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

  Future<String?> _scanQRCode(BuildContext context) async {
    final result = await Navigator.push<String>(
      context,
      MaterialPageRoute(builder: (context) => const QRScannerScreen()),
    );
    return result;
  }

  /// Helper to check if a token is KANARI
  bool _isKanariToken(TokenBalance token) {
    return token.tokenType == 'KANARI' ||
        token.tokenType.contains('::kanari::KANARI') ||
        token.symbol.toUpperCase() == 'KANARI';
  }

  /// Helper to build dropdown menu item for a token
  DropdownMenuItem<String> _buildTokenItem(TokenBalance token) {
    final formattedAmount = token.amount / math.pow(10, token.decimals);
    return DropdownMenuItem(
      value: token.tokenType,
      child: Row(
        children: [
          TokenLogo(
            tokenType: token.tokenType,
            symbol: token.symbol,
            size: 28,
            logoUrl: token.iconUrl,
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              '${token.symbol} (${formattedAmount.toStringAsFixed(4)})',
              overflow: TextOverflow.ellipsis,
            ),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final isSmallScreen = MediaQuery.of(context).size.width < 360;
    final walletState = context.watch<WalletState>();

    // Find KANARI token from balances list to get iconUrl and metadata
    final kanariToken = walletState.tokenBalances.firstWhere(
      _isKanariToken,
      orElse: () => TokenBalance(
        tokenType: 'KANARI',
        symbol: 'KANARI',
        amount: walletState.balance,
        decimals: 9,
        iconUrl: null,
      ),
    );

    // Build token list: KANARI first, then other tokens
    final tokenItems = [
      DropdownMenuItem(
        value: '',
        child: Row(
          children: [
            TokenLogo(
              tokenType: kanariToken.tokenType,
              symbol: kanariToken.symbol,
              size: 28,
              logoUrl: kanariToken.iconUrl,
            ),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                'KANARI (${(walletState.balance / 1000000000).toStringAsFixed(4)})',
                style: const TextStyle(fontWeight: FontWeight.bold),
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ],
        ),
      ),
      ...walletState.tokenBalances
          .where((token) => !_isKanariToken(token))
          .map(_buildTokenItem),
    ];

    return Material(
      borderRadius: const BorderRadius.vertical(top: Radius.circular(24)),
      color: colorScheme.surface,
      child: Padding(
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
                      setState(
                        () => _recipientController.text = scannedAddress,
                      );
                    }
                  },
                ),
              ),
            ),
            SizedBox(height: isSmallScreen ? 12 : 16),

            DropdownButtonFormField<String>(
              initialValue: _selectedTokenType,
              isExpanded: true,
              decoration: const InputDecoration(labelText: 'Asset to send'),
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
              keyboardType: const TextInputType.numberWithOptions(
                decimal: true,
              ),
              decoration: InputDecoration(
                labelText: 'Amount',
                prefixIcon: const Icon(Icons.account_balance_wallet_rounded),
                suffixIcon: TextButton(
                  onPressed: () {
                    if (_selectedTokenType.isEmpty) {
                      final balance = walletState.balance / 1000000000;
                      _amountController.text = balance.toStringAsFixed(6);
                    } else {
                      final token = walletState.tokenBalances.firstWhere(
                        (t) => t.tokenType == _selectedTokenType,
                      );
                      final maxAmount =
                          token.amount / math.pow(10, token.decimals);
                      _amountController.text = maxAmount.toStringAsFixed(
                        token.decimals < 6 ? token.decimals : 6,
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
              onPressed: () async => _handleTransfer(context, walletState),
              child: const Text(
                'Send Assets',
                style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
              ),
            ),
          ],
        ),
      ),
    );
  }

  /// Handle transfer logic
  Future<void> _handleTransfer(BuildContext context, WalletState ws) async {
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
          backgroundColor: Theme.of(context).colorScheme.error,
        ),
      );
      return;
    }

    Navigator.pop(context);

    String? result;
    if (_selectedTokenType.isEmpty) {
      final amountMist = (amountDouble * 1000000000).round();
      result = await ws.transfer(recipient, amountMist);
    } else {
      final selectedToken = ws.tokenBalances.firstWhere(
        (t) => t.tokenType == _selectedTokenType,
      );
      final amountBaseUnits =
          (amountDouble * math.pow(10, selectedToken.decimals)).round();
      result = await ws.transferToken(
        recipient,
        _selectedTokenType,
        amountBaseUnits,
      );
    }

    if (context.mounted) {
      final colorScheme = Theme.of(context).colorScheme;
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
  }
}

/// QR Scanner Screen - หน้าสแกน QR Code
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
            icon: const Icon(Icons.flashlight_on_rounded, color: Colors.white),
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
                  Navigator.pop(context, barcode.rawValue);
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
