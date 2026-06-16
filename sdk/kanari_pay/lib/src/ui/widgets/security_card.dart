import 'package:flutter/material.dart';
import 'package:kanari_pay/src/providers/wallet_provider.dart';
import 'package:provider/provider.dart';

import 'app_ui.dart';

class SecurityCard extends StatefulWidget {
  const SecurityCard({super.key});

  @override
  State<SecurityCard> createState() => _SecurityCardState();
}

class _SecurityCardState extends State<SecurityCard> {
  bool _isExpanded = false;

  Future<void> _toggleExpanded(BuildContext context) async {
    if (_isExpanded) {
      setState(() => _isExpanded = false);
      return;
    }

    final state = context.read<WalletState>();
    final authorized = await showAppPinVerificationSheet(
      context: context,
      onVerify: state.verifyPin,
      lockRemaining: state.pinLockRemaining,
      title: 'Unlock Secrets',
      subtitle: 'Enter your 6-digit PIN before showing wallet secrets.',
    );

    if (!mounted || !authorized) return;
    setState(() => _isExpanded = true);
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<WalletState>();
    if (state.wallet == null) return const SizedBox.shrink();

    final theme = Theme.of(context);
    final screenWidth = MediaQuery.of(context).size.width;
    final isSmallScreen = screenWidth < 360;

    final containerPadding = isSmallScreen ? 14.0 : 20.0;
    final iconSize = isSmallScreen ? 20.0 : 24.0;
    final iconPadding = isSmallScreen ? 8.0 : 10.0;
    final titleSize = isSmallScreen ? 13.0 : 15.0;
    final warningPadding = isSmallScreen ? 10.0 : 14.0;

    return Container(
      padding: EdgeInsets.all(containerPadding),
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(
          color: theme.colorScheme.outline.withValues(alpha: 0.2),
          width: 1,
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          GestureDetector(
            onTap: () => _toggleExpanded(context),
            child: Row(
              children: [
                Container(
                  padding: EdgeInsets.all(iconPadding),
                  decoration: BoxDecoration(
                    color: theme.colorScheme.surfaceContainerHighest.withValues(
                      alpha: 0.3,
                    ),
                    borderRadius: BorderRadius.circular(14),
                  ),
                  child: Icon(
                    Icons.security_rounded,
                    color: theme.colorScheme.onSurface,
                    size: iconSize,
                  ),
                ),
                SizedBox(width: isSmallScreen ? 10 : 16),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'Security Credentials',
                        style: TextStyle(
                          fontSize: titleSize,
                          fontWeight: FontWeight.w600,
                          color: theme.colorScheme.onSurface,
                        ),
                      ),
                      SizedBox(height: isSmallScreen ? 2 : 4),
                      Text(
                        'Mnemonic & Private Key',
                        style: TextStyle(
                          fontSize: isSmallScreen ? 10 : 12,
                          color: theme.colorScheme.onSurface.withValues(
                            alpha: 0.5,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
                Icon(
                  _isExpanded
                      ? Icons.keyboard_arrow_up
                      : Icons.keyboard_arrow_down,
                  color: theme.colorScheme.onSurface.withValues(alpha: 0.4),
                  size: 24,
                ),
              ],
            ),
          ),
          if (_isExpanded) ...[
            SizedBox(height: isSmallScreen ? 14 : 20),
            const Divider(height: 1),
            SizedBox(height: isSmallScreen ? 14 : 20),
            _buildSecurityField(
              context,
              'Mnemonic Seed',
              state.wallet!.mnemonic ?? 'Not available for this curve',
            ),
            SizedBox(height: isSmallScreen ? 12 : 16),
            _buildSecurityField(
              context,
              'Private Key',
              state.wallet!.privateKey,
            ),
            SizedBox(height: isSmallScreen ? 12 : 16),
            Container(
              padding: EdgeInsets.all(warningPadding),
              decoration: BoxDecoration(
                color: theme.colorScheme.errorContainer.withValues(alpha: 0.3),
                borderRadius: BorderRadius.circular(16),
                border: Border.all(
                  color: theme.colorScheme.error.withValues(alpha: 0.3),
                ),
              ),
              child: Row(
                children: [
                  Icon(
                    Icons.warning_amber_rounded,
                    color: theme.colorScheme.error,
                    size: isSmallScreen ? 18 : 20,
                  ),
                  SizedBox(width: isSmallScreen ? 8 : 12),
                  Expanded(
                    child: Text(
                      'NEVER share these with anyone. They grant full access to your funds.',
                      style: TextStyle(
                        fontSize: isSmallScreen ? 10 : 12,
                        color: theme.colorScheme.onErrorContainer,
                        fontWeight: FontWeight.w500,
                        height: 1.4,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ],
      ),
    );
  }

  Widget _buildSecurityField(BuildContext context, String label, String value) {
    final theme = Theme.of(context);
    final screenWidth = MediaQuery.of(context).size.width;
    final isSmallScreen = screenWidth < 360;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          label,
          style: TextStyle(
            fontSize: isSmallScreen ? 10 : 11,
            color: theme.colorScheme.onSurface.withValues(alpha: 0.4),
            fontWeight: FontWeight.w600,
            letterSpacing: 0.8,
          ),
        ),
        SizedBox(height: isSmallScreen ? 6 : 8),
        Container(
          width: double.infinity,
          padding: EdgeInsets.all(isSmallScreen ? 10 : 14),
          decoration: BoxDecoration(
            color: theme.colorScheme.surface.withValues(alpha: 0.5),
            borderRadius: BorderRadius.circular(14),
            border: Border.all(
              color: theme.colorScheme.outline.withValues(alpha: 0.08),
            ),
          ),
          child: SelectableText(
            value,
            style: TextStyle(
              fontFamily: 'monospace',
              fontSize: isSmallScreen ? 10 : 11,
              color: theme.colorScheme.onSurface,
              fontWeight: FontWeight.w400,
              letterSpacing: 0.3,
              height: 1.5,
            ),
          ),
        ),
      ],
    );
  }
}
