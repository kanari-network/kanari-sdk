import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:provider/provider.dart';
import 'package:qr_flutter/qr_flutter.dart';

import '../providers/wallet_provider.dart';
import 'widgets/app_ui.dart';

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
              color: theme.colorScheme.onSurface.withValues(alpha: 0.4),
              letterSpacing: 1.5,
            ),
          ),
        ),
        Container(
          padding: EdgeInsets.all(cardPadding),
          decoration: BoxDecoration(
            color: theme.colorScheme.surface,
            borderRadius: BorderRadius.circular(16),
            border: Border.all(
              color: theme.colorScheme.outline.withValues(alpha: 0.2),
              width: 1,
            ),
          ),
          child: Row(
            children: [
              Container(
                padding: EdgeInsets.all(iconPadding),
                decoration: BoxDecoration(
                  color: theme.colorScheme.surfaceContainerHighest.withValues(
                    alpha: 0.5,
                  ),
                  borderRadius: BorderRadius.circular(16),
                ),
                child: Icon(
                  Icons.account_circle_rounded,
                  color: theme.colorScheme.onSurface,
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
                    color: theme.colorScheme.outline.withValues(alpha: 0.1),
                  ),
                ),
                child: IconButton(
                  icon: const Icon(Icons.qr_code_rounded, size: 20),
                  onPressed: () {
                    _showQRCodeDialog(context, state.wallet!.address);
                  },
                  tooltip: 'Show QR Code',
                  color: theme.colorScheme.onSurfaceVariant,
                  padding: EdgeInsets.all(isSmallScreen ? 6 : 8),
                ),
              ),
              SizedBox(width: isSmallScreen ? 2 : 4),
              Container(
                decoration: BoxDecoration(
                  color: theme.colorScheme.surface,
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(
                    color: theme.colorScheme.outline.withValues(alpha: 0.1),
                  ),
                ),
                child: IconButton(
                  icon: const Icon(Icons.copy_rounded, size: 20),
                  onPressed: () {
                    Clipboard.setData(
                      ClipboardData(text: state.wallet!.address),
                    );
                    showAppInfoSnackBar(context, 'Address copied');
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

  void _showQRCodeDialog(BuildContext context, String address) {
    final theme = Theme.of(context);

    showDialog(
      context: context,
      builder: (dialogContext) => Dialog(
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(24)),
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  // 👈 1. หุ้ม Text ด้วย Expanded
                  Expanded(
                    child: Text(
                      'Wallet QR Code',
                      style: theme.textTheme.titleLarge?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                      maxLines: 1, // 👈 2. (Optional) จำกัด 1 บรรทัด
                      overflow: TextOverflow
                          .ellipsis, // 👈 3. (Optional) ถ้าล้นให้เป็นจุดไข่ปลา
                    ),
                  ),
                  IconButton(
                    icon: const Icon(Icons.close_rounded),
                    onPressed: () => Navigator.pop(dialogContext),
                    tooltip: 'Close',
                  ),
                ],
              ),
              const SizedBox(height: 24),
              Container(
                padding: const EdgeInsets.all(16),
                decoration: BoxDecoration(
                  color: Colors.white,
                  borderRadius: BorderRadius.circular(16),
                  border: Border.all(
                    color: theme.colorScheme.outline.withValues(alpha: 0.2),
                    width: 2,
                  ),
                ),
                child: QrImageView(
                  data: address,
                  version: QrVersions.auto,
                  size: 200.0,
                  backgroundColor: Colors.white,
                  embeddedImageStyle: QrEmbeddedImageStyle(
                    size: const Size(40, 40),
                  ),
                ),
              ),
              const SizedBox(height: 24),
              Text(
                'Scan to receive KANARI',
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: theme.colorScheme.onSurface.withValues(alpha: 0.6),
                ),
              ),
              const SizedBox(height: 16),
              Container(
                padding: const EdgeInsets.symmetric(
                  horizontal: 12,
                  vertical: 8,
                ),
                decoration: BoxDecoration(
                  color: theme.colorScheme.surfaceContainerHighest.withValues(
                    alpha: 0.1,
                  ),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Text(
                  address,
                  style: theme.textTheme.bodySmall?.copyWith(
                    fontFamily: 'monospace',
                    fontSize: 10,
                    color: theme.colorScheme.onSurface.withValues(alpha: 0.7),
                  ),
                  textAlign: TextAlign.center,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              const SizedBox(height: 20),
              SizedBox(
                width: double.infinity,
                child: ElevatedButton.icon(
                  onPressed: () {
                    Clipboard.setData(ClipboardData(text: address));
                    Navigator.pop(dialogContext);
                    showAppInfoSnackBar(context, 'Address copied to clipboard');
                  },
                  icon: const Icon(Icons.copy_rounded, size: 18),
                  label: const Text('Copy Address'),
                  style: ElevatedButton.styleFrom(
                    padding: const EdgeInsets.symmetric(vertical: 12),
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(12),
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
