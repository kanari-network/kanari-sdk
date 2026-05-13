import 'package:flutter/material.dart';

import '../../kanaricurve.dart';

class AppCurveSelectionSheet extends StatefulWidget {
  final String title;
  final String subtitle;
  final String confirmLabel;
  final ValueChanged<KanariCurve> onConfirm;

  const AppCurveSelectionSheet({
    super.key,
    required this.onConfirm,
    this.title = 'Wallet Options',
    this.subtitle = 'Select the cryptographic curve for your new wallet.',
    this.confirmLabel = 'Generate Wallet',
  });

  @override
  State<AppCurveSelectionSheet> createState() => _AppCurveSelectionSheetState();
}

class _AppCurveSelectionSheetState extends State<AppCurveSelectionSheet> {
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
            widget.title,
            style: theme.textTheme.headlineSmall?.copyWith(
              fontWeight: FontWeight.bold,
            ),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 8),
          Text(
            widget.subtitle,
            style: const TextStyle(fontSize: 13, color: Colors.grey),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 24),
          DropdownButtonFormField<KanariCurve>(
            initialValue: _selectedCurve,
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
            onPressed: () => widget.onConfirm(_selectedCurve),
            child: Text(
              widget.confirmLabel,
              style: const TextStyle(fontSize: 16, fontWeight: FontWeight.bold),
            ),
          ),
        ],
      ),
    );
  }
}

class AppImportWalletSheet extends StatefulWidget {
  final void Function(String data, KanariCurve curve, bool isMnemonic)
  onContinue;

  const AppImportWalletSheet({super.key, required this.onContinue});

  @override
  State<AppImportWalletSheet> createState() => _AppImportWalletSheetState();
}

class _AppImportWalletSheetState extends State<AppImportWalletSheet>
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
              initialValue: _curve,
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
                Navigator.pop(context);
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
