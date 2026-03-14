import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../providers/wallet_provider.dart';
import 'package:provider/provider.dart';

class BalanceCard extends StatelessWidget {
  const BalanceCard({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<WalletState>();
    final theme = Theme.of(context);

    // Responsive sizing based on screen width
    final screenWidth = MediaQuery.of(context).size.width;
    final isSmallScreen = screenWidth < 360;
    final isMediumScreen = screenWidth >= 360 && screenWidth < 600;

    final balanceFontSize = isSmallScreen
        ? 36.0
        : (isMediumScreen ? 44.0 : 56.0);
    final titleFontSize = isSmallScreen ? 11.0 : 13.0;
    final badgeFontSize = isSmallScreen ? 9.0 : 11.0;
    final verticalPadding = isSmallScreen
        ? 24.0
        : (isMediumScreen ? 32.0 : 40.0);
    final horizontalPadding = isSmallScreen ? 16.0 : 24.0;

    return Container(
      width: double.infinity,
      padding: EdgeInsets.symmetric(
        vertical: verticalPadding,
        horizontal: horizontalPadding,
      ),
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(32),
        gradient: LinearGradient(
          colors: [
            theme.colorScheme.primary,
            theme.colorScheme.primary.withOpacity(0.7),
          ],
          begin: Alignment.topCenter,
          end: Alignment.bottomCenter,
        ),
        boxShadow: [
          BoxShadow(
            color: theme.colorScheme.primary.withOpacity(0.2),
            blurRadius: 30,
            offset: const Offset(0, 15),
            spreadRadius: -5,
          ),
        ],
      ),
      child: Column(
        children: [
          Text(
            'Total Balance',
            style: TextStyle(
              fontSize: titleFontSize,
              color: theme.colorScheme.onPrimary.withOpacity(0.7),
              fontWeight: FontWeight.w500,
              letterSpacing: 1.5,
            ),
          ),
          SizedBox(height: isSmallScreen ? 10 : 16),
          FittedBox(
            fit: BoxFit.scaleDown,
            alignment: Alignment.center,
            child: Text(
              (state.balance / 1000000000).toStringAsFixed(6),
              style: TextStyle(
                fontSize: balanceFontSize,
                fontWeight: FontWeight.w300,
                color: theme.colorScheme.onPrimary,
                letterSpacing: -2,
                height: 1.0,
              ),
            ),
          ),
          SizedBox(height: isSmallScreen ? 8 : 12),
          Container(
            padding: EdgeInsets.symmetric(
              horizontal: isSmallScreen ? 12 : 16,
              vertical: isSmallScreen ? 4 : 6,
            ),
            decoration: BoxDecoration(
              color: theme.colorScheme.onPrimary.withOpacity(0.15),
              borderRadius: BorderRadius.circular(20),
              border: Border.all(
                color: theme.colorScheme.onPrimary.withOpacity(0.2),
                width: 1,
              ),
            ),
            child: Text(
              'KANARI',
              style: TextStyle(
                fontSize: badgeFontSize,
                color: theme.colorScheme.onPrimary,
                fontWeight: FontWeight.w600,
                letterSpacing: 2,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
