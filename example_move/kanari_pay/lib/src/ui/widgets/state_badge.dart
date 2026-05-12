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

  Color _getStateColor(ColorScheme colorScheme) {
    switch (state) {
      case 1:
        return Colors.orange;
      case 2:
        return Colors.blue;
      case 3:
        return Colors.green;
      case 4:
        return Colors.red;
      default:
        return Colors.grey;
    }
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
      decoration: BoxDecoration(
        color: _getStateColor(colorScheme).withOpacity(0.2),
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: _getStateColor(colorScheme)),
      ),
      child: Text(
        _getStateName(),
        style: TextStyle(
          color: _getStateColor(colorScheme),
          fontWeight: FontWeight.bold,
          fontSize: 12,
        ),
      ),
    );
  }
}
