// NEW FILE: c:\Users\Pukpuy\Desktop\kanari-sdk\example_move\kanari_kit\lib\src\ui\widgets\deal_card.dart

import 'package:flutter/material.dart';

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

    return Card(
      margin: const EdgeInsets.only(bottom: 12),
      color: isSelected ? colorScheme.primaryContainer.withOpacity(0.3) : null,
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Expanded(
                    child: Text(
                      'Deal ID',
                      style: Theme.of(context).textTheme.titleSmall?.copyWith(
                            fontWeight: FontWeight.bold,
                            color: isSelected ? colorScheme.primary : null,
                          ),
                    ),
                  ),
                  if (isSelected)
                    Icon(
                      Icons.check_circle,
                      color: colorScheme.primary,
                      size: 20,
                    ),
                ],
              ),
              const SizedBox(height: 8),
              _buildRow('ID', _truncate(dealId, 20)),
              const SizedBox(height: 4),
              _buildRow('Amount', '$amount $coinName'),
              const SizedBox(height: 4),
              _buildRow('Buyer', _truncate(buyer, 20)),
              const SizedBox(height: 4),
              _buildRow('Seller', _truncate(seller, 20)),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildRow(String label, String value) {
    return Row(
      mainAxisAlignment: MainAxisAlignment.spaceBetween,
      children: [
        Text(
          label,
          style: const TextStyle(
            fontSize: 12,
            fontWeight: FontWeight.w500,
          ),
        ),
        Text(
          value,
          style: const TextStyle(
            fontSize: 12,
            fontFamily: 'monospace',
          ),
        ),
      ],
    );
  }
}
