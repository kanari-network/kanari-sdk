import 'package:flutter/material.dart';
import 'package:google_fonts/google_fonts.dart';

/// ============================================
/// Typography Constants for Kanari Pay
/// ============================================
class TextSize {
  static const double titleLarge = 28.0; // Main titles, balance amounts
  static const double titleMedium = 20.0; // Section headers, card titles
  static const double bodyLarge = 16.0; // Primary body text
  static const double bodyMedium = 14.0; // Secondary information
  static const double bodySmall = 12.0; // Metadata, labels
  static const double caption = 10.0; // Captions, hints
}

/// ============================================
/// Spacing Constants
/// ============================================
class AppSpacing {
  static const double xs = 4.0;
  static const double sm = 8.0;
  static const double md = 16.0;
  static const double lg = 24.0;
  static const double xl = 32.0;
  static const double xxl = 48.0;
}

/// ============================================
/// Border Radius Constants
/// ============================================
class AppBorderRadius {
  static const double sm = 8.0;
  static const double md = 12.0;
  static const double lg = 16.0;
  static const double xl = 24.0;
  static const double xxl = 32.0;
}

/// ============================================
/// Elevation Constants
/// ============================================
class AppElevation {
  static const double none = 0.0;
  static const double low = 2.0;
  static const double medium = 4.0;
  static const double high = 8.0;
}

/// ============================================
/// Create Custom Text Theme
/// ============================================
TextTheme createTextTheme(
  BuildContext context,
  String bodyFontString,
  String displayFontString,
) {
  TextTheme baseTextTheme = Theme.of(context).textTheme;
  TextTheme bodyTextTheme = GoogleFonts.getTextTheme(
    bodyFontString,
    baseTextTheme,
  );
  TextTheme displayTextTheme = GoogleFonts.getTextTheme(
    displayFontString,
    baseTextTheme,
  );

  return displayTextTheme.copyWith(
    // Display/Title styles
    displayLarge: displayTextTheme.displayLarge?.copyWith(
      fontSize: TextSize.titleLarge,
      fontWeight: FontWeight.w700,
      letterSpacing: -0.5,
    ),
    displayMedium: displayTextTheme.displayMedium?.copyWith(
      fontSize: 24.0,
      fontWeight: FontWeight.w700,
      letterSpacing: -0.5,
    ),
    displaySmall: displayTextTheme.displaySmall?.copyWith(
      fontSize: TextSize.titleMedium,
      fontWeight: FontWeight.w700,
      letterSpacing: -0.5,
    ),

    // Headline styles
    headlineLarge: displayTextTheme.headlineLarge?.copyWith(
      fontSize: 22.0,
      fontWeight: FontWeight.w600,
      letterSpacing: -0.25,
    ),
    headlineMedium: displayTextTheme.headlineMedium?.copyWith(
      fontSize: 20.0,
      fontWeight: FontWeight.w600,
      letterSpacing: -0.25,
    ),
    headlineSmall: displayTextTheme.headlineSmall?.copyWith(
      fontSize: 18.0,
      fontWeight: FontWeight.w600,
      letterSpacing: 0,
    ),

    // Title styles
    titleLarge: displayTextTheme.titleLarge?.copyWith(
      fontSize: TextSize.titleMedium,
      fontWeight: FontWeight.w600,
      letterSpacing: 0,
    ),
    titleMedium: displayTextTheme.titleMedium?.copyWith(
      fontSize: 16.0,
      fontWeight: FontWeight.w600,
      letterSpacing: 0.15,
    ),
    titleSmall: displayTextTheme.titleSmall?.copyWith(
      fontSize: 14.0,
      fontWeight: FontWeight.w600,
      letterSpacing: 0.1,
    ),

    // Body styles
    bodyLarge: bodyTextTheme.bodyLarge?.copyWith(
      fontSize: TextSize.bodyLarge,
      fontWeight: FontWeight.w400,
      letterSpacing: 0.5,
    ),
    bodyMedium: bodyTextTheme.bodyMedium?.copyWith(
      fontSize: TextSize.bodyMedium,
      fontWeight: FontWeight.w400,
      letterSpacing: 0.25,
    ),
    bodySmall: bodyTextTheme.bodySmall?.copyWith(
      fontSize: TextSize.bodySmall,
      fontWeight: FontWeight.w400,
      letterSpacing: 0.4,
    ),

    // Label styles
    labelLarge: bodyTextTheme.labelLarge?.copyWith(
      fontSize: 14.0,
      fontWeight: FontWeight.w600,
      letterSpacing: 0.1,
    ),
    labelMedium: bodyTextTheme.labelMedium?.copyWith(
      fontSize: 12.0,
      fontWeight: FontWeight.w500,
      letterSpacing: 0.5,
    ),
    labelSmall: bodyTextTheme.labelSmall?.copyWith(
      fontSize: TextSize.caption,
      fontWeight: FontWeight.w500,
      letterSpacing: 0.5,
    ),
  );
}

/// ============================================
/// Material Theme with Material Design 3 Color Scheme
/// ============================================
class MaterialTheme {
  final TextTheme textTheme;

  const MaterialTheme(this.textTheme);

  /// Light Color Scheme - Black & White Theme
  static ColorScheme lightScheme() {
    return const ColorScheme(
      brightness: Brightness.light,
      primary: Color(0xff000000),
      surfaceTint: Color(0xff000000),
      onPrimary: Color(0xffffffff),
      primaryContainer: Color(0xffe6e6e6),
      onPrimaryContainer: Color(0xff1a1a1a),
      secondary: Color(0xff4d4d4d),
      onSecondary: Color(0xffffffff),
      secondaryContainer: Color(0xfff2f2f2),
      onSecondaryContainer: Color(0xff333333),
      tertiary: Color(0xff666666),
      onTertiary: Color(0xffffffff),
      tertiaryContainer: Color(0xffe8e8e8),
      onTertiaryContainer: Color(0xff404040),
      error: Color(0xffba1a1a),
      onError: Color(0xffffffff),
      errorContainer: Color(0xffffdad6),
      onErrorContainer: Color(0xff93000a),
      surface: Color(0xffffffff),
      onSurface: Color(0xff000000),
      onSurfaceVariant: Color(0xff404040),
      outline: Color(0xff999999),
      outlineVariant: Color(0xffcccccc),
      shadow: Color(0xff000000),
      scrim: Color(0xff000000),
      inverseSurface: Color(0xff1a1a1a),
      inversePrimary: Color(0xffcccccc),
      primaryFixed: Color(0xffe6e6e6),
      onPrimaryFixed: Color(0xff000000),
      primaryFixedDim: Color(0xffcccccc),
      onPrimaryFixedVariant: Color(0xff1a1a1a),
      secondaryFixed: Color(0xfff2f2f2),
      onSecondaryFixed: Color(0xff1a1a1a),
      secondaryFixedDim: Color(0xffd9d9d9),
      onSecondaryFixedVariant: Color(0xff333333),
      tertiaryFixed: Color(0xffe8e8e8),
      onTertiaryFixed: Color(0xff1a1a1a),
      tertiaryFixedDim: Color(0xffcccccc),
      onTertiaryFixedVariant: Color(0xff404040),
      surfaceDim: Color(0xfff2f2f2),
      surfaceBright: Color(0xffffffff),
      surfaceContainerLowest: Color(0xffffffff),
      surfaceContainerLow: Color(0xfffafafa),
      surfaceContainer: Color(0xfff5f5f5),
      surfaceContainerHigh: Color(0xffeeeeee),
      surfaceContainerHighest: Color(0xffe6e6e6),
    );
  }

  ThemeData light() {
    return theme(lightScheme());
  }

  /// Dark Color Scheme - Black & White Theme
  static ColorScheme darkScheme() {
    return const ColorScheme(
      brightness: Brightness.dark,
      primary: Color(0xffffffff),
      surfaceTint: Color(0xffffffff),
      onPrimary: Color(0xff000000),
      primaryContainer: Color(0xff333333),
      onPrimaryContainer: Color(0xffe6e6e6),
      secondary: Color(0xffb3b3b3),
      onSecondary: Color(0xff000000),
      secondaryContainer: Color(0xff4d4d4d),
      onSecondaryContainer: Color(0xffe6e6e6),
      tertiary: Color(0xff999999),
      onTertiary: Color(0xff000000),
      tertiaryContainer: Color(0xff404040),
      onTertiaryContainer: Color(0xffe8e8e8),
      error: Color(0xffffb4ab),
      onError: Color(0xff690005),
      errorContainer: Color(0xff93000a),
      onErrorContainer: Color(0xffffdad6),
      surface: Color(0xff000000),
      onSurface: Color(0xffe6e6e6),
      onSurfaceVariant: Color(0xffa3a3a3),
      outline: Color(0xff707070),
      outlineVariant: Color(0xff404040),
      shadow: Color(0xff000000),
      scrim: Color(0xff000000),
      inverseSurface: Color(0xffe6e6e6),
      inversePrimary: Color(0xff333333),
      primaryFixed: Color(0xffe6e6e6),
      onPrimaryFixed: Color(0xff000000),
      primaryFixedDim: Color(0xffcccccc),
      onPrimaryFixedVariant: Color(0xff1a1a1a),
      secondaryFixed: Color(0xffe6e6e6),
      onSecondaryFixed: Color(0xff000000),
      secondaryFixedDim: Color(0xffcccccc),
      onSecondaryFixedVariant: Color(0xff333333),
      tertiaryFixed: Color(0xffe8e8e8),
      onTertiaryFixed: Color(0xff000000),
      tertiaryFixedDim: Color(0xffcccccc),
      onTertiaryFixedVariant: Color(0xff404040),
      surfaceDim: Color(0xff000000),
      surfaceBright: Color(0xff262626),
      surfaceContainerLowest: Color(0xff000000),
      surfaceContainerLow: Color(0xff0d0d0d),
      surfaceContainer: Color(0xff121212),
      surfaceContainerHigh: Color(0xff1c1c1c),
      surfaceContainerHighest: Color(0xff262626),
    );
  }

  ThemeData dark() {
    return theme(darkScheme());
  }

  /// Create ThemeData from ColorScheme
  ThemeData theme(ColorScheme colorScheme) => ThemeData(
    useMaterial3: true,
    brightness: colorScheme.brightness,
    colorScheme: colorScheme,
    textTheme: textTheme.apply(
      bodyColor: colorScheme.onSurface,
      displayColor: colorScheme.onSurface,
    ),
    scaffoldBackgroundColor: colorScheme.surface,
    canvasColor: colorScheme.surface,

    // AppBar Theme
    appBarTheme: AppBarTheme(
      centerTitle: true,
      scrolledUnderElevation: 0,
      backgroundColor: Colors.transparent,
      foregroundColor: colorScheme.onSurface,
      elevation: 0,
    ),

    // Card Theme
    cardTheme: CardThemeData(
      elevation: AppElevation.low,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(AppBorderRadius.xl),
      ),
      color: colorScheme.surfaceContainerLow,
    ),

    // Input Decoration Theme
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: colorScheme.surfaceContainerLow,
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(AppBorderRadius.lg),
        borderSide: BorderSide.none,
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(AppBorderRadius.lg),
        borderSide: BorderSide(color: colorScheme.outline.withOpacity(0.3)),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(AppBorderRadius.lg),
        borderSide: BorderSide(color: colorScheme.primary, width: 2),
      ),
      contentPadding: const EdgeInsets.symmetric(
        horizontal: AppSpacing.md,
        vertical: AppSpacing.md,
      ),
      labelStyle: TextStyle(
        color: colorScheme.onSurfaceVariant,
        fontSize: TextSize.bodyMedium,
      ),
      hintStyle: TextStyle(
        color: colorScheme.onSurface.withOpacity(0.4),
        fontSize: TextSize.bodyMedium,
      ),
    ),

    // Button Themes
    elevatedButtonTheme: ElevatedButtonThemeData(
      style: ElevatedButton.styleFrom(
        backgroundColor: colorScheme.primary,
        foregroundColor: colorScheme.onPrimary,
        elevation: AppElevation.low,
        padding: const EdgeInsets.symmetric(
          horizontal: AppSpacing.lg,
          vertical: AppSpacing.md,
        ),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(AppBorderRadius.lg),
        ),
        textStyle: TextStyle(
          fontSize: TextSize.bodyMedium,
          fontWeight: FontWeight.w600,
          letterSpacing: 0.5,
        ),
      ),
    ),

    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        backgroundColor: colorScheme.primary,
        foregroundColor: colorScheme.onPrimary,
        padding: const EdgeInsets.symmetric(
          horizontal: AppSpacing.lg,
          vertical: AppSpacing.md,
        ),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(AppBorderRadius.lg),
        ),
        textStyle: TextStyle(
          fontSize: TextSize.bodyMedium,
          fontWeight: FontWeight.w600,
          letterSpacing: 0.5,
        ),
      ),
    ),

    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        foregroundColor: colorScheme.primary,
        side: BorderSide(color: colorScheme.primary),
        padding: const EdgeInsets.symmetric(
          horizontal: AppSpacing.lg,
          vertical: AppSpacing.md,
        ),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(AppBorderRadius.lg),
        ),
        textStyle: TextStyle(
          fontSize: TextSize.bodyMedium,
          fontWeight: FontWeight.w600,
          letterSpacing: 0.5,
        ),
      ),
    ),

    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(
        foregroundColor: colorScheme.primary,
        padding: const EdgeInsets.symmetric(
          horizontal: AppSpacing.sm,
          vertical: AppSpacing.xs,
        ),
        textStyle: TextStyle(
          fontSize: TextSize.bodyMedium,
          fontWeight: FontWeight.w600,
          letterSpacing: 0.5,
        ),
      ),
    ),

    // Bottom Sheet Theme
    bottomSheetTheme: BottomSheetThemeData(
      backgroundColor: colorScheme.surface,
      modalBackgroundColor: colorScheme.surface,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(
          top: Radius.circular(AppBorderRadius.xl),
        ),
      ),
      elevation: AppElevation.high,
    ),

    // Dialog Theme
    dialogTheme: DialogThemeData(
      backgroundColor: colorScheme.surface,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(AppBorderRadius.xl),
      ),
      elevation: AppElevation.high,
    ),

    // Chip Theme
    chipTheme: ChipThemeData(
      backgroundColor: colorScheme.surfaceContainerHigh,
      selectedColor: colorScheme.primaryContainer,
      labelStyle: TextStyle(
        color: colorScheme.onSurface,
        fontSize: TextSize.bodySmall,
      ),
      padding: const EdgeInsets.symmetric(
        horizontal: AppSpacing.sm,
        vertical: AppSpacing.xs,
      ),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(AppBorderRadius.md),
      ),
    ),
  );

  List<ExtendedColor> get extendedColors => [];
}

class ExtendedColor {
  final Color seed, value;
  final ColorFamily light;
  final ColorFamily lightHighContrast;
  final ColorFamily lightMediumContrast;
  final ColorFamily dark;
  final ColorFamily darkHighContrast;
  final ColorFamily darkMediumContrast;

  const ExtendedColor({
    required this.seed,
    required this.value,
    required this.light,
    required this.lightHighContrast,
    required this.lightMediumContrast,
    required this.dark,
    required this.darkHighContrast,
    required this.darkMediumContrast,
  });
}

class ColorFamily {
  const ColorFamily({
    required this.color,
    required this.onColor,
    required this.colorContainer,
    required this.onColorContainer,
  });

  final Color color;
  final Color onColor;
  final Color colorContainer;
  final Color onColorContainer;
}
