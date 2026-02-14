import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../../main.dart';
import 'package:provider/provider.dart';

class WalletInfoCard extends StatelessWidget {
  const WalletInfoCard({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<WalletState>();
    if (state.wallet == null) return const SizedBox.shrink();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Padding(
          padding: EdgeInsets.only(left: 4, bottom: 8),
          child: Text('WALLET ADDRESS', style: TextStyle(fontSize: 12, fontWeight: FontWeight.bold, color: Colors.grey)),
        ),
        Container(
          padding: const EdgeInsets.all(16),
          decoration: BoxDecoration(
            color: Colors.white.withOpacity(0.05),
            borderRadius: BorderRadius.circular(16),
            border: Border.all(color: Colors.white10),
          ),
          child: Row(
            children: [
              Expanded(
                child: SelectableText(
                  state.wallet!.address,
                  style: const TextStyle(fontFamily: 'monospace', fontSize: 13, color: Colors.blueAccent),
                ),
              ),
              IconButton(
                icon: const Icon(Icons.copy, size: 18, color: Colors.grey),
                onPressed: () {
                  Clipboard.setData(ClipboardData(text: state.wallet!.address));
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(content: Text('Address copied to clipboard'), duration: Duration(seconds: 1)),
                  );
                },
              ),
            ],
          ),
        ),
      ],
    );
  }
}
