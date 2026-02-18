import 'package:flutter/material.dart';
import '../providers/wallet_provider.dart';
import 'package:provider/provider.dart';

class BalanceCard extends StatelessWidget {
  const BalanceCard({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<WalletState>();
    final theme = Theme.of(context);

    return Container(
      width: double.infinity,
      padding: const EdgeInsets.all(24.0),
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(24),
        gradient: LinearGradient(
          colors: [theme.colorScheme.primary, theme.colorScheme.secondary],
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
        ),
        boxShadow: [
          BoxShadow(
            color: theme.colorScheme.primary.withOpacity(0.3),
            blurRadius: 20,
            offset: const Offset(0, 10),
          ),
        ],
      ),
      child: Column(
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(
                Icons.account_balance_wallet_rounded,
                color: theme.colorScheme.onPrimary.withOpacity(0.8),
                size: 18,
              ),
              const SizedBox(width: 8),
              Text(
                'Total Balance',
                style: TextStyle(
                  fontSize: 14,
                  color: theme.colorScheme.onPrimary.withOpacity(0.8),
                  fontWeight: FontWeight.w500,
                  letterSpacing: 0.5,
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),
          FittedBox(
            fit: BoxFit.scaleDown,
            child: Text(
              (state.balance / 1000000000).toStringAsFixed(9),
              style: TextStyle(
                fontSize: 44,
                fontWeight: FontWeight.bold,
                color: theme.colorScheme.onPrimary,
                letterSpacing: -0.5,
              ),
            ),
          ),
          const SizedBox(height: 4),
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
            decoration: BoxDecoration(
              color: theme.colorScheme.onPrimary.withOpacity(0.15),
              borderRadius: BorderRadius.circular(20),
            ),
            child: Text(
              'KANARI',
              style: TextStyle(
                fontSize: 12,
                color: theme.colorScheme.onPrimary,
                fontWeight: FontWeight.bold,
                letterSpacing: 1.5,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
