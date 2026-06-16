// NEW FILE: c:\Users\Pukpuy\Desktop\kanari-sdk\example_move\kanari_kit\lib\src\ui\widgets\state_badge.dart

import 'package:flutter/material.dart';

import 'app_ui.dart';

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
    final (background, foreground) = switch (state) {
      1 => (Colors.orange.withValues(alpha: 0.16), Colors.orange.shade800),
      2 => (Colors.blue.withValues(alpha: 0.16), Colors.blue.shade800),
      3 => (Colors.green.withValues(alpha: 0.16), Colors.green.shade800),
      4 => (colorScheme.errorContainer, colorScheme.onErrorContainer),
      _ => (colorScheme.surfaceContainerHighest, colorScheme.onSurfaceVariant),
    };

    return Container(
      padding: const EdgeInsets.symmetric(
        horizontal: AppUiTokens.badgeHorizontalPadding,
        vertical: AppUiTokens.badgeVerticalPadding,
      ),
      decoration: BoxDecoration(
        color: background,
        borderRadius: BorderRadius.circular(AppUiTokens.pillRadius),
        border: Border.all(color: foreground.withValues(alpha: 0.2)),
      ),
      child: Text(
        _getStateName(),
        style: TextStyle(
          color: foreground,
          fontWeight: FontWeight.w700,
          fontSize: 12,
        ),
      ),
    );
  }
}
