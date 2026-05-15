import 'package:flutter/material.dart';

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

    // ตรวจสอบว่ามี logoUrl จาก API หรือไม่ (รวมถึง KANARI token)
    if (logoUrl != null && logoUrl!.isNotEmpty) {
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
            errorBuilder: (context, error, stackTrace) {
              // ถ้าโหลดรูปไม่ได้ สำหรับ KANARI แสดง hexagon, token อื่นๆ แสดงตัวอักษรย่อ
              if (tokenType == 'KANARI' || symbol.toUpperCase() == 'KANARI') {
                return Center(
                  child: Icon(
                    Icons.hexagon_rounded,
                    color: colorScheme.onSurface,
                    size: size * 0.5,
                  ),
                );
              } else {
                return Center(
                  child: Text(
                    symbol.length > 2
                        ? symbol.substring(0, 2).toUpperCase()
                        : symbol.toUpperCase(),
                    style: TextStyle(
                      color: colorScheme.onSurface,
                      fontWeight: FontWeight.bold,
                      fontSize: size * 0.35,
                    ),
                  ),
                );
              }
            },
          ),
        ),
      );
    }

    // Fallback: ไม่มี logoUrl
    // สำหรับ Kanari token ใช้ logo พิเศษ
    if (tokenType == 'KANARI' || symbol.toUpperCase() == 'KANARI') {
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
        child: Center(
          child: Icon(
            Icons.hexagon_rounded,
            color: colorScheme.onSurface,
            size: size * 0.5,
          ),
        ),
      );
    }

    // สำหรับ token อื่นๆ แสดงตัวอักษรย่อ
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
      child: Center(
        child: Text(
          symbol.length > 2
              ? symbol.substring(0, 2).toUpperCase()
              : symbol.toUpperCase(),
          style: TextStyle(
            color: colorScheme.onSurface,
            fontWeight: FontWeight.bold,
            fontSize: size * 0.35,
          ),
        ),
      ),
    );
  }
}
