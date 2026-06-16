// NEW FILE: c:\Users\Pukpuy\Desktop\kanari-sdk\example_move\kanari_kit\lib\src\ui\widgets\deal_card.dart

import 'package:flutter/material.dart';

import 'app_ui.dart';

/// Reusable widget สำหรับแสดง Deal card
class DealCard extends StatelessWidget {
  final Map<String, dynamic> deal;
  final bool isSelected;
  final VoidCallback? onTap;
  final ColorScheme colorScheme;

  const DealCard({
    super.key,
    required this.deal,
    this.isSelected = false,
    this.onTap,
    required this.colorScheme,
  });

  String _truncate(String value, int length) {
    return value.length > length ? '${value.substring(0, length)}...' : value;
  }

  @override
  Widget build(BuildContext context) {
    final dealId = deal['deal_id'] as String? ?? 'N/A';
    final amount = deal['amount'] as int? ?? 0;
    final coinType = deal['coin_type'] as String? ?? '';
    final buyer = deal['buyer'] as String? ?? 'N/A';
    final seller = deal['seller'] as String? ?? 'N/A';
    final coinName = coinType.split('::').lastOrNull ?? 'COIN';
    final theme = Theme.of(context);

    return AnimatedContainer(
      duration: const Duration(milliseconds: 180),
      margin: const EdgeInsets.only(bottom: AppUiTokens.sectionSpacing),
      decoration: BoxDecoration(
        color: isSelected
            ? colorScheme.secondaryContainer.withValues(
                alpha: AppUiTokens.selectedFillAlpha,
              )
            : colorScheme.surfaceContainerLowest,
        borderRadius: BorderRadius.circular(AppUiTokens.cardRadius),
        border: Border.all(
          color: isSelected
              ? colorScheme.primary.withValues(
                  alpha: AppUiTokens.selectedBorderAlpha,
                )
              : colorScheme.outline.withValues(
                  alpha: AppUiTokens.subtleBorderAlpha,
                ),
        ),
      ),
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: onTap,
          borderRadius: BorderRadius.circular(AppUiTokens.cardRadius),
          child: Padding(
            padding: const EdgeInsets.all(AppUiTokens.cardPadding),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Expanded(
                      child: Text(
                        'Deal ID',
                        style: theme.textTheme.titleSmall?.copyWith(
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ),
                    if (isSelected)
                      Icon(
                        Icons.check_circle_rounded,
                        color: colorScheme.primary,
                        size: 20,
                      ),
                  ],
                ),
                const SizedBox(height: AppUiTokens.contentSpacing),
                AppDetailRow(label: 'ID', value: _truncate(dealId, 20)),
                const SizedBox(height: AppUiTokens.compactSpacing),
                AppDetailRow(label: 'Amount', value: '$amount $coinName'),
                const SizedBox(height: AppUiTokens.compactSpacing),
                AppDetailRow(label: 'Buyer', value: _truncate(buyer, 20)),
                const SizedBox(height: AppUiTokens.compactSpacing),
                AppDetailRow(label: 'Seller', value: _truncate(seller, 20)),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
