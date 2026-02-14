import 'package:flutter/material.dart';
import '../../main.dart';
import 'package:provider/provider.dart';

class BalanceCard extends StatelessWidget {
  const BalanceCard({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<WalletState>();
    return Card(
      elevation: 8,
      shadowColor: Colors.blueAccent.withOpacity(0.3),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(20)),
      child: Container(
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(20),
          gradient: LinearGradient(
            colors: [
              Colors.blueAccent.withOpacity(0.2),
              Colors.purpleAccent.withOpacity(0.1),
            ],
            begin: Alignment.topLeft,
            end: Alignment.bottomRight,
          ),
        ),
        padding: const EdgeInsets.all(24.0),
        child: Column(
          children: [
            Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Icon(
                  Icons.account_balance_wallet_outlined,
                  color: Colors.blueAccent.shade100,
                  size: 20,
                ),
                const SizedBox(width: 8),
                const Text(
                  'Total Balance',
                  style: TextStyle(
                    fontSize: 14,
                    color: Colors.grey,
                    fontWeight: FontWeight.w500,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),
            Text(
              (state.balance / 1000000000).toStringAsFixed(9),
              style: const TextStyle(
                fontSize: 40,
                fontWeight: FontWeight.bold,
                letterSpacing: 1,
              ),
            ),
            const Text(
              'KANARI',
              style: TextStyle(
                fontSize: 16,
                color: Colors.blueAccent,
                fontWeight: FontWeight.bold,
              ),
            ),
          ],
        ),
      ),
    );
  }
}
