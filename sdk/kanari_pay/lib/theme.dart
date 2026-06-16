import 'package:flutter/material.dart';
import 'package:google_fonts/google_fonts.dart';

abstract final class KanariColors {
  static const ink = Color(0xFF111B18);
  static const cream = Color(0xFFF7F4EB);
  static const paper = Color(0xFFFFFDF7);
  static const lime = Color(0xFFC8F43D);
  static const lavender = Color(0xFFCABDFF);
  static const purple = Color(0xFF7868DA);
  static const darkPaper = Color(0xFF17211E);
  static const darkSurface = Color(0xFF0E1513);
}

abstract final class AppSpacing {
  static const xs = 4.0;
  static const sm = 8.0;
  static const md = 16.0;
  static const lg = 24.0;
  static const xl = 32.0;
  static const xxl = 48.0;
}

abstract final class AppBorderRadius {
  static const sm = 8.0;
  static const md = 12.0;
  static const lg = 18.0;
  static const xl = 24.0;
}

TextTheme createTextTheme(
  BuildContext context,
  String bodyFontString,
  String displayFontString,
) {
  final base = Theme.of(context).textTheme;
  final body = GoogleFonts.getTextTheme(bodyFontString, base);
  final display = GoogleFonts.getTextTheme(displayFontString, base);

  return body.copyWith(
    displayLarge: display.displayLarge?.copyWith(
      fontSize: 54,
      height: 0.94,
      fontWeight: FontWeight.w900,
      letterSpacing: 0,
    ),
    displayMedium: display.displayMedium?.copyWith(
      fontSize: 40,
      height: 0.98,
      fontWeight: FontWeight.w900,
      letterSpacing: 0,
    ),
    displaySmall: display.displaySmall?.copyWith(
      fontSize: 32,
      height: 1,
      fontWeight: FontWeight.w900,
      letterSpacing: 0,
    ),
    headlineLarge: display.headlineLarge?.copyWith(
      fontSize: 28,
      height: 1.05,
      fontWeight: FontWeight.w900,
      letterSpacing: 0,
    ),
    headlineMedium: display.headlineMedium?.copyWith(
      fontSize: 24,
      fontWeight: FontWeight.w800,
      letterSpacing: 0,
    ),
    headlineSmall: display.headlineSmall?.copyWith(
      fontSize: 20,
      fontWeight: FontWeight.w800,
      letterSpacing: 0,
    ),
    titleLarge: display.titleLarge?.copyWith(
      fontSize: 20,
      fontWeight: FontWeight.w800,
      letterSpacing: 0,
    ),
    titleMedium: display.titleMedium?.copyWith(
      fontSize: 16,
      fontWeight: FontWeight.w800,
      letterSpacing: 0,
    ),
    titleSmall: display.titleSmall?.copyWith(
      fontSize: 14,
      fontWeight: FontWeight.w700,
      letterSpacing: 0,
    ),
    bodyLarge: body.bodyLarge?.copyWith(
      fontSize: 16,
      height: 1.5,
      letterSpacing: 0,
    ),
    bodyMedium: body.bodyMedium?.copyWith(
      fontSize: 14,
      height: 1.45,
      letterSpacing: 0,
    ),
    bodySmall: body.bodySmall?.copyWith(
      fontSize: 12,
      height: 1.4,
      letterSpacing: 0,
    ),
    labelLarge: body.labelLarge?.copyWith(
      fontSize: 13,
      fontWeight: FontWeight.w800,
      letterSpacing: 0,
    ),
    labelMedium: body.labelMedium?.copyWith(
      fontSize: 12,
      fontWeight: FontWeight.w800,
      letterSpacing: 0,
    ),
  );
}

class MaterialTheme {
  final TextTheme textTheme;

  const MaterialTheme(this.textTheme);

  static const _lightScheme = ColorScheme(
    brightness: Brightness.light,
    primary: KanariColors.ink,
    onPrimary: KanariColors.cream,
    primaryContainer: KanariColors.lime,
    onPrimaryContainer: KanariColors.ink,
    secondary: KanariColors.purple,
    onSecondary: Colors.white,
    secondaryContainer: KanariColors.lavender,
    onSecondaryContainer: KanariColors.ink,
    tertiary: KanariColors.lime,
    onTertiary: KanariColors.ink,
    tertiaryContainer: Color(0xFFE8F9A8),
    onTertiaryContainer: KanariColors.ink,
    error: Color(0xFFB42318),
    onError: Colors.white,
    errorContainer: Color(0xFFFFDAD6),
    onErrorContainer: Color(0xFF410002),
    surface: KanariColors.cream,
    onSurface: KanariColors.ink,
    onSurfaceVariant: Color(0xFF59625F),
    outline: Color(0xFF8C9491),
    outlineVariant: Color(0xFFD5D5CC),
    shadow: KanariColors.ink,
    scrim: KanariColors.ink,
    inverseSurface: KanariColors.ink,
    onInverseSurface: KanariColors.cream,
    inversePrimary: KanariColors.lime,
    surfaceTint: Colors.transparent,
    surfaceDim: Color(0xFFE8E5DC),
    surfaceBright: KanariColors.paper,
    surfaceContainerLowest: KanariColors.paper,
    surfaceContainerLow: Color(0xFFFBF8EF),
    surfaceContainer: Color(0xFFF1EEE5),
    surfaceContainerHigh: Color(0xFFE9E6DD),
    surfaceContainerHighest: Color(0xFFDFDCD3),
  );

  static const _darkScheme = ColorScheme(
    brightness: Brightness.dark,
    primary: KanariColors.lime,
    onPrimary: KanariColors.ink,
    primaryContainer: KanariColors.lime,
    onPrimaryContainer: KanariColors.ink,
    secondary: KanariColors.lavender,
    onSecondary: KanariColors.ink,
    secondaryContainer: Color(0xFF4E4674),
    onSecondaryContainer: Color(0xFFF1ECFF),
    tertiary: KanariColors.lime,
    onTertiary: KanariColors.ink,
    tertiaryContainer: Color(0xFF405013),
    onTertiaryContainer: Color(0xFFE8F9A8),
    error: Color(0xFFFFB4AB),
    onError: Color(0xFF690005),
    errorContainer: Color(0xFF93000A),
    onErrorContainer: Color(0xFFFFDAD6),
    surface: KanariColors.darkSurface,
    onSurface: KanariColors.cream,
    onSurfaceVariant: Color(0xFFB8C0BC),
    outline: Color(0xFF858E8A),
    outlineVariant: Color(0xFF3C4642),
    shadow: Colors.black,
    scrim: Colors.black,
    inverseSurface: KanariColors.cream,
    onInverseSurface: KanariColors.ink,
    inversePrimary: KanariColors.purple,
    surfaceTint: Colors.transparent,
    surfaceDim: Color(0xFF09100E),
    surfaceBright: Color(0xFF2B3531),
    surfaceContainerLowest: Color(0xFF070C0B),
    surfaceContainerLow: Color(0xFF121A17),
    surfaceContainer: KanariColors.darkPaper,
    surfaceContainerHigh: Color(0xFF202B27),
    surfaceContainerHighest: Color(0xFF2A3531),
  );

  ThemeData light() => _theme(_lightScheme);

  ThemeData dark() => _theme(_darkScheme);

  ThemeData _theme(ColorScheme colors) {
    final border = BorderSide(color: colors.outlineVariant);
    final rounded = RoundedRectangleBorder(
      borderRadius: BorderRadius.circular(AppBorderRadius.lg),
    );

    return ThemeData(
      useMaterial3: true,
      colorScheme: colors,
      scaffoldBackgroundColor: colors.surface,
      canvasColor: colors.surface,
      textTheme: textTheme.apply(
        bodyColor: colors.onSurface,
        displayColor: colors.onSurface,
      ),
      dividerColor: colors.outlineVariant,
      appBarTheme: AppBarTheme(
        centerTitle: false,
        elevation: 0,
        scrolledUnderElevation: 0,
        backgroundColor: Colors.transparent,
        foregroundColor: colors.onSurface,
        titleTextStyle: textTheme.titleLarge?.copyWith(color: colors.onSurface),
      ),
      cardTheme: CardThemeData(
        elevation: 0,
        color: colors.surfaceContainerLow,
        margin: EdgeInsets.zero,
        shape: rounded.copyWith(side: border),
      ),
      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: colors.surfaceContainerLow,
        contentPadding: const EdgeInsets.symmetric(
          horizontal: 18,
          vertical: 17,
        ),
        border: OutlineInputBorder(
          borderRadius: BorderRadius.circular(AppBorderRadius.md),
          borderSide: border,
        ),
        enabledBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(AppBorderRadius.md),
          borderSide: border,
        ),
        focusedBorder: OutlineInputBorder(
          borderRadius: BorderRadius.circular(AppBorderRadius.md),
          borderSide: BorderSide(color: colors.secondary, width: 2),
        ),
        labelStyle: TextStyle(color: colors.onSurfaceVariant),
        hintStyle: TextStyle(color: colors.onSurfaceVariant),
      ),
      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(
          minimumSize: const Size(48, 54),
          padding: const EdgeInsets.symmetric(horizontal: 22, vertical: 16),
          backgroundColor: colors.primary,
          foregroundColor: colors.onPrimary,
          elevation: 0,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(AppBorderRadius.md),
          ),
          textStyle: textTheme.labelLarge,
        ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          minimumSize: const Size(48, 54),
          padding: const EdgeInsets.symmetric(horizontal: 22, vertical: 16),
          foregroundColor: colors.onSurface,
          side: border,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(AppBorderRadius.md),
          ),
          textStyle: textTheme.labelLarge,
        ),
      ),
      textButtonTheme: TextButtonThemeData(
        style: TextButton.styleFrom(
          foregroundColor: colors.onSurface,
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 13),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(AppBorderRadius.sm),
          ),
          textStyle: textTheme.labelLarge,
        ),
      ),
      iconButtonTheme: IconButtonThemeData(
        style: IconButton.styleFrom(
          foregroundColor: colors.onSurface,
          shape: const CircleBorder(),
        ),
      ),
      bottomSheetTheme: BottomSheetThemeData(
        backgroundColor: colors.surfaceContainerLowest,
        modalBackgroundColor: colors.surfaceContainerLowest,
        surfaceTintColor: Colors.transparent,
        showDragHandle: true,
        shape: const RoundedRectangleBorder(
          borderRadius: BorderRadius.vertical(top: Radius.circular(24)),
        ),
      ),
      dialogTheme: DialogThemeData(
        elevation: 0,
        backgroundColor: colors.surfaceContainerLowest,
        surfaceTintColor: Colors.transparent,
        shape: rounded.copyWith(side: border),
      ),
      snackBarTheme: SnackBarThemeData(
        behavior: SnackBarBehavior.floating,
        backgroundColor: colors.inverseSurface,
        contentTextStyle: TextStyle(color: colors.onInverseSurface),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(AppBorderRadius.md),
        ),
      ),
      progressIndicatorTheme: ProgressIndicatorThemeData(
        color: colors.tertiary,
      ),
    );
  }
}
