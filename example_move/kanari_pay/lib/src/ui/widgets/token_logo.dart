import 'package:flutter/material.dart';
import '../../core/token_utils.dart' as token_utils;

/// Reusable widget สำหรับแสดง Token Logo
/// รองรับทั้ง Kanari token และ token อื่นๆ ด้วยการแสดงสัญลักษณ์หรือตัวอักษรย่อ
class TokenLogo extends StatelessWidget {
  final String tokenType;
  final String symbol;
  final double size;
  final String? logoUrl;

  const TokenLogo({
    super.key,
    required this.tokenType,
    required this.symbol,
    this.size = 40,
    this.logoUrl,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final isKanari = token_utils.isKanariLike(
      tokenType: tokenType,
      symbol: symbol,
    );
    final fallbackChild = isKanari
        ? Icon(
            Icons.hexagon_rounded,
            color: colorScheme.onSurface,
            size: size * 0.5,
          )
        : Text(
            token_utils.tokenInitials(symbol),
            style: TextStyle(
              color: colorScheme.onSurface,
              fontWeight: FontWeight.bold,
              fontSize: size * 0.35,
            ),
          );

    if (logoUrl != null && logoUrl!.isNotEmpty) {
      return _buildFrame(
        colorScheme,
        child: ClipOval(
          child: Image.network(
            logoUrl!,
            fit: BoxFit.cover,
            loadingBuilder: (context, child, loadingProgress) {
              if (loadingProgress == null) return child;
              return Center(
                child: SizedBox(
                  width: size * 0.3,
                  height: size * 0.3,
                  child: CircularProgressIndicator(
                    strokeWidth: 2,
                    value: loadingProgress.expectedTotalBytes != null
                        ? loadingProgress.cumulativeBytesLoaded /
                              loadingProgress.expectedTotalBytes!
                        : null,
                  ),
                ),
              );
            },
            errorBuilder: (context, error, stackTrace) =>
                Center(child: fallbackChild),
          ),
        ),
      );
    }

    return _buildFrame(colorScheme, child: Center(child: fallbackChild));
  }

  Widget _buildFrame(ColorScheme colorScheme, {required Widget child}) {
    return Container(
      width: size,
      height: size,
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest,
        shape: BoxShape.circle,
        border: Border.all(
          color: colorScheme.outline.withOpacity(0.2),
          width: 1.5,
        ),
      ),
      child: child,
    );
  }
}
