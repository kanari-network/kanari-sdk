import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../providers/wallet_provider.dart';
import 'package:provider/provider.dart';

class WalletInfoCard extends StatelessWidget {
  const WalletInfoCard({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<WalletState>();
    if (state.wallet == null) return const SizedBox.shrink();

    final theme = Theme.of(context);
    final screenWidth = MediaQuery.of(context).size.width;
    final isSmallScreen = screenWidth < 360;
    
    final iconSize = isSmallScreen ? 22.0 : 28.0;
    final iconPadding = isSmallScreen ? 8.0 : 12.0;
    final fontSize = isSmallScreen ? 11.0 : 12.0;
    final cardPadding = isSmallScreen ? 12.0 : 20.0;
    final spacing = isSmallScreen ? 10.0 : 16.0;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.only(left: 4, bottom: 12),
          child: Text(
            'Wallet Address',
            style: theme.textTheme.labelSmall?.copyWith(
              fontWeight: FontWeight.w600,
              color: theme.colorScheme.onSurface.withOpacity(0.4),
              letterSpacing: 1.5,
            ),
          ),
        ),
        Container(
          padding: EdgeInsets.all(cardPadding),
          decoration: BoxDecoration(
            color: theme.colorScheme.surfaceVariant.withOpacity(0.2),
            borderRadius: BorderRadius.circular(24),
            border: Border.all(
              color: theme.colorScheme.outline.withOpacity(0.08),
              width: 1,
            ),
          ),
          child: Row(
            children: [
              Container(
                padding: EdgeInsets.all(iconPadding),
                decoration: BoxDecoration(
                  color: theme.colorScheme.primary.withOpacity(0.08),
                  borderRadius: BorderRadius.circular(16),
                ),
                child: Icon(
                  Icons.account_circle_rounded,
                  color: theme.colorScheme.primary,
                  size: iconSize,
                ),
              ),
              SizedBox(width: spacing),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    SelectableText(
                      state.wallet!.address,
                      style: TextStyle(
                        fontFamily: 'monospace',
                        fontSize: fontSize,
                        color: theme.colorScheme.onSurface,
                        fontWeight: FontWeight.w400,
                        letterSpacing: 0.5,
                        height: 1.4,
                      ),
                    ),
                  ],
                ),
              ),
              SizedBox(width: isSmallScreen ? 2 : 4),
              Container(
                decoration: BoxDecoration(
                  color: theme.colorScheme.surface,
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(
                    color: theme.colorScheme.outline.withOpacity(0.1),
                  ),
                ),
                child: IconButton(
                  icon: const Icon(Icons.copy_rounded, size: 20),
                  onPressed: () {
                    Clipboard.setData(ClipboardData(text: state.wallet!.address));
                    ScaffoldMessenger.of(context).showSnackBar(
                      SnackBar(
                        content: const Text('Address copied'),
                        behavior: SnackBarBehavior.floating,
                        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
                        backgroundColor: theme.colorScheme.onSurface.withOpacity(0.8),
                      ),
                    );
                  },
                  tooltip: 'Copy Address',
                  color: theme.colorScheme.onSurfaceVariant,
                  padding: EdgeInsets.all(isSmallScreen ? 6 : 8),
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }
}
