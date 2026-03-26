import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:kanari_kit/src/providers/wallet_provider.dart';
import 'package:provider/provider.dart';
import 'package:kanari_kit/kanari_kit.dart';

class WelcomeScreen extends StatelessWidget {
  const WelcomeScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<WalletState>();
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Scaffold(
      body: Container(
        decoration: BoxDecoration(
          gradient: LinearGradient(
            begin: Alignment.topCenter,
            end: Alignment.bottomCenter,
            colors: [
              colorScheme.surface,
              colorScheme.primaryContainer.withOpacity(0.12),
            ],
          ),
        ),
        child: SafeArea(
          // 👇 แก้ไขตรงนี้: เพิ่ม CustomScrollView และ SliverFillRemaining
          child: CustomScrollView(
            slivers: [
              SliverFillRemaining(
                hasScrollBody: false,
                child: Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 24.0),
                  child: Column(
                    children: [
                      const Spacer(),
                      _buildLogo(theme),
                      const SizedBox(height: 32),
                      Text(
                        'Kanari Wallet',
                        style: theme.textTheme.displaySmall?.copyWith(
                          fontWeight: FontWeight.bold,
                          color: colorScheme.onSurface,
                          letterSpacing: -0.5,
                        ),
                      ),
                      const SizedBox(height: 12),
                      Text(
                        'Secure, Quantum-Safe Digital Wallet',
                        textAlign: TextAlign.center,
                        style: theme.textTheme.bodyLarge?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ),
                      const Spacer(),

                      // M3 Action Area
                      if (state.hasWallet) ...[
                        FilledButton.icon(
                          onPressed: () => _showUnlockSheet(context),
                          icon: const Icon(Icons.lock_open_rounded),
                          label: const Text('Unlock Saved Wallet'),
                          style: FilledButton.styleFrom(
                            minimumSize: const Size(double.infinity, 56),
                          ),
                        ),
                        const SizedBox(height: 12),
                        TextButton.icon(
                          onPressed: () => _showDeleteConfirm(context, state),
                          icon: const Icon(
                            Icons.delete_outline_rounded,
                            size: 20,
                          ),
                          label: const Text('Clear All Wallet Data'),
                          style: TextButton.styleFrom(
                            foregroundColor: colorScheme.error,
                          ),
                        ),
                        _buildDivider(),
                      ],

                      FilledButton.tonalIcon(
                        onPressed: () => _showCreateSheet(context),
                        icon: const Icon(Icons.add_rounded),
                        label: const Text('Create New Wallet'),
                        style: FilledButton.styleFrom(
                          minimumSize: const Size(double.infinity, 56),
                        ),
                      ),
                      const SizedBox(height: 12),
                      OutlinedButton.icon(
                        onPressed: () => _showImportDialog(context),
                        icon: const Icon(Icons.file_download_outlined),
                        label: const Text('Import Existing Wallet'),
                        style: OutlinedButton.styleFrom(
                          minimumSize: const Size(double.infinity, 56),
                        ),
                      ),
                      // ลดความสูงด้านล่างลงนิดหน่อยเพื่อประหยัดพื้นที่หน้าจอเล็ก
                      const SizedBox(height: 32),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildLogo(ThemeData theme) {
    return Container(
      padding: const EdgeInsets.all(28),
      decoration: BoxDecoration(
        color: theme.colorScheme.primaryContainer,
        borderRadius: BorderRadius.circular(28),
      ),
      child: Icon(
        Icons.blur_on_rounded,
        size: 72,
        color: theme.colorScheme.onPrimaryContainer,
      ),
    );
  }

  Widget _buildDivider() {
    return const Padding(
      padding: EdgeInsets.symmetric(vertical: 24),
      child: Row(
        children: [
          Expanded(child: Divider()),
          Padding(
            padding: EdgeInsets.symmetric(horizontal: 16),
            child: Text(
              'OR',
              style: TextStyle(
                fontSize: 12,
                fontWeight: FontWeight.w500,
                color: Colors.grey,
              ),
            ),
          ),
          Expanded(child: Divider()),
        ],
      ),
    );
  }

  // --- Dialogs & Sheets ---

  void _showUnlockSheet(BuildContext context) {
    final walletState = context.read<WalletState>();
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      backgroundColor: Theme.of(context).colorScheme.surface,
      builder: (context) => _PinEntryPage(
        title: 'Enter PIN',
        subtitle: 'Enter your 6-digit PIN to unlock your wallet.',
        onComplete: (pin) {
          walletState.unlockWallet(pin);
        },
      ),
    );
  }

  void _showCreateSheet(BuildContext context) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      backgroundColor: Theme.of(context).colorScheme.surface,
      builder: (context) => _PinEntryPage(
        title: 'Set PIN',
        subtitle: 'Set a 6-digit PIN to secure your wallet.',
        onComplete: (pin) {
          Future.delayed(const Duration(milliseconds: 150), () {
            if (context.mounted) {
              _showCurveSelectionSheet(context, pin);
            }
          });
        },
      ),
    );
  }

  void _showCurveSelectionSheet(BuildContext context, String pin) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      showDragHandle: true,
      backgroundColor: Theme.of(context).colorScheme.surface,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(32)),
      ),
      builder: (context) => _CurveSelectionSheet(pin: pin),
    );
  }

  // 👉 อัปเดต Import Flow ให้เป็น 2 สเต็ป
  void _showImportDialog(BuildContext context) {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      showDragHandle: true,
      builder: (context) => _M3ImportSheet(
        onContinue: (String data, KanariCurve curve, bool isMnemonic) {
          // พอรับข้อมูลมาแล้ว ให้เปิดหน้าใส่ PIN ทันที
          Future.delayed(const Duration(milliseconds: 150), () {
            if (context.mounted) {
              _showImportPinSheet(context, data, curve, isMnemonic);
            }
          });
        },
      ),
    );
  }

  // 👉 หน้าต่างใส่รหัส PIN สำหรับการ Import
  void _showImportPinSheet(
    BuildContext context,
    String data,
    KanariCurve curve,
    bool isMnemonic,
  ) {
    final walletState = context.read<WalletState>();
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      useSafeArea: true,
      backgroundColor: Theme.of(context).colorScheme.surface,
      builder: (context) => _PinEntryPage(
        title: 'Set PIN',
        subtitle: 'Set a 6-digit PIN to secure your imported wallet.',
        onComplete: (pin) {
          if (isMnemonic) {
            walletState.importFromMnemonic(data, curve: curve, pin: pin);
          } else {
            walletState.importFromPrivateKey(data, curve: curve, pin: pin);
          }
        },
      ),
    );
  }

  void _showDeleteConfirm(BuildContext context, WalletState state) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Delete All Data?'),
        content: const Text(
          'This action cannot be undone. Make sure you have backed up your seed phrases.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () {
              state.deleteAllWallets();
              Navigator.pop(context);
            },
            child: const Text(
              'Clear Everything',
              style: TextStyle(color: Colors.red),
            ),
          ),
        ],
      ),
    );
  }
}

// ============================================================================
// UI: Step 2 - หน้าต่างเลือก Curve หลังจากใส่ PIN เสร็จ (Create Wallet)
// ============================================================================
class _CurveSelectionSheet extends StatefulWidget {
  final String pin;
  const _CurveSelectionSheet({required this.pin});

  @override
  State<_CurveSelectionSheet> createState() => _CurveSelectionSheetState();
}

class _CurveSelectionSheetState extends State<_CurveSelectionSheet> {
  KanariCurve _selectedCurve = KanariCurve.ed25519;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

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
            'Wallet Options',
            style: theme.textTheme.headlineSmall?.copyWith(
              fontWeight: FontWeight.bold,
            ),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 8),
          const Text(
            'Select the cryptographic curve for your new wallet.',
            style: TextStyle(fontSize: 13, color: Colors.grey),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 24),
          DropdownButtonFormField<KanariCurve>(
            value: _selectedCurve,
            isExpanded: true,
            decoration: InputDecoration(
              labelText: 'Curve Type',
              border: OutlineInputBorder(
                borderRadius: BorderRadius.circular(16),
              ),
            ),
            items: KanariCurve.values.map((curve) {
              return DropdownMenuItem(
                value: curve,
                child: Text(curve.name, overflow: TextOverflow.ellipsis),
              );
            }).toList(),
            onChanged: (val) => setState(() => _selectedCurve = val!),
          ),
          const SizedBox(height: 32),
          FilledButton(
            style: FilledButton.styleFrom(
              minimumSize: const Size(double.infinity, 56),
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(20),
              ),
            ),
            onPressed: () {
              context.read<WalletState>().createNewWallet(
                curve: _selectedCurve,
                pin: widget.pin,
              );
              Navigator.pop(context);
            },
            child: const Text(
              'Generate Wallet',
              style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
            ),
          ),
        ],
      ),
    );
  }
}

// ============================================================================
// UI: หน้าจอใส่รหัส PIN (Custom Number Pad)
// ============================================================================
class _PinEntryPage extends StatefulWidget {
  final String title;
  final String subtitle;
  final Function(String pin) onComplete;

  const _PinEntryPage({
    required this.title,
    required this.subtitle,
    required this.onComplete,
  });

  @override
  State<_PinEntryPage> createState() => _PinEntryPageState();
}

class _PinEntryPageState extends State<_PinEntryPage> {
  String _enteredPin = "";

  void _handleNumberPressed(String number) {
    if (_enteredPin.length < 6) {
      setState(() {
        _enteredPin += number;
      });
      if (_enteredPin.length == 6) {
        Future.delayed(const Duration(milliseconds: 200), () {
          if (mounted) {
            Navigator.pop(context);
            widget.onComplete(_enteredPin);
          }
        });
      }
    }
  }

  void _handleBackspace() {
    if (_enteredPin.isNotEmpty) {
      setState(() {
        _enteredPin = _enteredPin.substring(0, _enteredPin.length - 1);
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Scaffold(
      backgroundColor: Colors.transparent,
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        elevation: 0,
        leading: CloseButton(color: colorScheme.onSurface),
      ),
      body: SafeArea(
        child: Column(
          children: [
            const SizedBox(height: 16),
            Text(
              widget.title,
              style: theme.textTheme.headlineMedium?.copyWith(
                fontWeight: FontWeight.bold,
                color: colorScheme.onSurface,
              ),
            ),
            const SizedBox(height: 12),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 32.0),
              child: Text(
                widget.subtitle,
                textAlign: TextAlign.center,
                style: theme.textTheme.bodyLarge?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
            ),

            const Spacer(),

            _PinCirclesDisplay(length: _enteredPin.length, totalLength: 6),

            const Spacer(),

            _CustomNumberPad(
              onNumberPressed: _handleNumberPressed,
              onBackspacePressed: _handleBackspace,
            ),
            const SizedBox(height: 16),
          ],
        ),
      ),
    );
  }
}

// ============================================================================
// Widget: จุดวงกลมแสดงสถานะ PIN
// ============================================================================
class _PinCirclesDisplay extends StatelessWidget {
  final int length;
  final int totalLength;

  const _PinCirclesDisplay({required this.length, required this.totalLength});

  @override
  Widget build(BuildContext context) {
    final primaryColor = Theme.of(context).colorScheme.primary;
    final outlineColor = Theme.of(context).colorScheme.outlineVariant;

    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: List.generate(totalLength, (index) {
        final isFilled = index < length;
        return Padding(
          padding: const EdgeInsets.symmetric(horizontal: 10.0),
          child: AnimatedContainer(
            duration: const Duration(milliseconds: 150),
            width: 20,
            height: 20,
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              color: isFilled ? primaryColor : Colors.transparent,
              border: Border.all(
                color: isFilled ? primaryColor : outlineColor,
                width: 2,
              ),
            ),
          ),
        );
      }),
    );
  }
}

// ============================================================================
// Widget: แป้นพิมพ์ตัวเลขแบบกำหนดเอง
// ============================================================================
class _CustomNumberPad extends StatelessWidget {
  final Function(String) onNumberPressed;
  final VoidCallback onBackspacePressed;

  const _CustomNumberPad({
    required this.onNumberPressed,
    required this.onBackspacePressed,
  });

  @override
  Widget build(BuildContext context) {
    final numbers = [
      ['1', '2', '3'],
      ['4', '5', '6'],
      ['7', '8', '9'],
    ];

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 32.0),
      child: Column(
        children: [
          for (var row in numbers) ...[
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceEvenly,
              children: row.map((number) {
                return _NumberButton(
                  number: number,
                  onPressed: () => onNumberPressed(number),
                );
              }).toList(),
            ),
            const SizedBox(height: 16),
          ],
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceEvenly,
            children: [
              const SizedBox(width: 80, height: 80),
              _NumberButton(number: '0', onPressed: () => onNumberPressed('0')),
              SizedBox(
                width: 80,
                height: 80,
                child: IconButton(
                  onPressed: onBackspacePressed,
                  icon: const Icon(Icons.backspace_outlined),
                  iconSize: 28,
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

// ============================================================================
// Widget ย่อย: ปุ่มตัวเลขวงกลม
// ============================================================================
class _NumberButton extends StatelessWidget {
  final String number;
  final VoidCallback onPressed;

  const _NumberButton({required this.number, required this.onPressed});

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final theme = Theme.of(context);

    return Container(
      width: 80,
      height: 80,
      decoration: BoxDecoration(
        color: colorScheme.surfaceVariant.withOpacity(0.3),
        shape: BoxShape.circle,
      ),
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: onPressed,
          customBorder: const CircleBorder(),
          child: Center(
            child: Text(
              number,
              style: theme.textTheme.headlineMedium?.copyWith(
                fontWeight: FontWeight.w600,
                color: colorScheme.onSurface,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

// ============================================================================
// 👉 UI: Import Wallet Sheet (Step 1: กรอกข้อมูล -> กด Continue)
// ============================================================================
class _M3ImportSheet extends StatefulWidget {
  final Function(String data, KanariCurve curve, bool isMnemonic) onContinue;

  const _M3ImportSheet({required this.onContinue});

  @override
  State<_M3ImportSheet> createState() => _M3ImportSheetState();
}

class _M3ImportSheetState extends State<_M3ImportSheet>
    with SingleTickerProviderStateMixin {
  late TabController _tabController;
  final _dataController = TextEditingController();
  KanariCurve _curve = KanariCurve.ed25519;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 2, vsync: this);
  }

  @override
  void dispose() {
    _tabController.dispose();
    _dataController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Padding(
      padding: EdgeInsets.fromLTRB(
        24,
        8,
        24,
        MediaQuery.of(context).viewInsets.bottom + 24,
      ),
      // 👇 หุ้มด้วย SingleChildScrollView เพื่อให้เลื่อนได้เวลาคีย์บอร์ดเด้งขึ้นมา
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              'Import Wallet',
              style: theme.textTheme.headlineSmall?.copyWith(
                fontWeight: FontWeight.bold,
              ),
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 16),
            TabBar(
              controller: _tabController,
              tabs: const [
                Tab(text: 'Private Key'),
                Tab(text: 'Mnemonic'),
              ],
            ),
            const SizedBox(height: 24),
            DropdownButtonFormField<KanariCurve>(
              value: _curve,
              decoration: InputDecoration(
                labelText: 'Curve Type',
                border: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(16),
                ),
              ),
              items: KanariCurve.values
                  .map((c) => DropdownMenuItem(value: c, child: Text(c.name)))
                  .toList(),
              onChanged: (v) => setState(() => _curve = v!),
            ),
            const SizedBox(height: 16),
            TextField(
              controller: _dataController,
              maxLines: 3,
              decoration: InputDecoration(
                hintText: 'Enter your key or 12 words',
                border: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(16),
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
              onPressed: () {
                if (_dataController.text.trim().isEmpty) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(
                      content: const Text('Please enter your key or mnemonic'),
                      backgroundColor: colorScheme.error,
                    ),
                  );
                  return;
                }

                final isMnemonic = _tabController.index == 1;
                Navigator.pop(context); // ปิดหน้าต่างนี้ลง
                // ส่งข้อมูลไปเปิดหน้าใส่รหัส PIN (Step 2)
                widget.onContinue(
                  _dataController.text.trim(),
                  _curve,
                  isMnemonic,
                );
              },
              child: const Text(
                'Continue',
                style: TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
