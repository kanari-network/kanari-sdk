// NEW FILE: c:\Users\Pukpuy\Desktop\kanari-sdk\example_move\kanari_kit\lib\src\ui\widgets\state_badge.dart

import 'package:flutter/material.dart';

/// Reusable widget สำหรับแสดง State badge
class StateBadge extends StatelessWidget {
  final int state;

  const StateBadge({super.key, required this.state});

  String _getStateName() {
    switch (state) {
      case 1:
        return 'Locked';
      case 2:
        return 'Delivered';
      case 3:
        return 'Completed';
      case 4:
        return 'Disputed';
      default:
        return 'Unknown';
    }
  }


  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: colorScheme.outline.withOpacity(0.3)),
      ),
      child: Text(
        _getStateName(),
        style: TextStyle(
          color: colorScheme.onSurface,
          fontWeight: FontWeight.bold,
          fontSize: 12,
        ),
      ),
    );
  }
}
