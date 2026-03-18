import 'package:flutter/material.dart';

/// Photonix Camera color system.
/// Dark-first: the camera viewfinder is always visible,
/// so every UI element must be readable against a dark background.
class PhotonixColors {
  PhotonixColors._();

  // ── Base surfaces ──────────────────────────────────────────────────────────
  static const background = Color(0xFF080808); // near-black — behind viewfinder
  static const surface = Color(0xFF141414); // cards, bottom sheet
  static const surfaceElevated = Color(0xFF1E1E1E); // raised cards

  // ── Accent ────────────────────────────────────────────────────────────────
  static const accent = Color(0xFFFFCC00); // shutter ring, active state
  static const accentDim = Color(0x40FFCC00); // disabled shutter

  // ── Semantic ──────────────────────────────────────────────────────────────
  static const success = Color(0xFF22C55E);
  static const error = Color(0xFFEF4444);
  static const processing = Color(0xFF3B82F6);

  // ── Text ──────────────────────────────────────────────────────────────────
  static const textPrimary = Color(0xFFFFFFFF);
  static const textSecondary = Color(0xFF9CA3AF);
  static const textTertiary = Color(0xFF4B5563);

  // ── Border ────────────────────────────────────────────────────────────────
  static const border = Color(0xFF262626);
  static const borderFocus = Color(0xFF404040);
}

class AppTheme {
  AppTheme._();

  static ThemeData get dark => ThemeData(
    useMaterial3: true,
    brightness: Brightness.dark,
    scaffoldBackgroundColor: PhotonixColors.background,

    colorScheme: const ColorScheme.dark(
      primary: PhotonixColors.accent,
      secondary: PhotonixColors.processing,
      surface: PhotonixColors.surface,
      error: PhotonixColors.error,
      onPrimary: Colors.black,
      onSecondary: Colors.white,
      onSurface: PhotonixColors.textPrimary,
    ),

    appBarTheme: const AppBarTheme(
      backgroundColor: PhotonixColors.background,
      foregroundColor: PhotonixColors.textPrimary,
      elevation: 0,
      centerTitle: false,
      titleTextStyle: TextStyle(
        color: PhotonixColors.textPrimary,
        fontSize: 17,
        fontWeight: FontWeight.w500,
        letterSpacing: -0.3,
      ),
    ),

    textTheme: const TextTheme(
      // Display — app name, large headers
      displayLarge: TextStyle(
        fontSize: 32,
        fontWeight: FontWeight.w600,
        color: PhotonixColors.textPrimary,
        letterSpacing: -0.5,
      ),
      // Title — screen headers
      titleLarge: TextStyle(
        fontSize: 20,
        fontWeight: FontWeight.w500,
        color: PhotonixColors.textPrimary,
        letterSpacing: -0.3,
      ),
      titleMedium: TextStyle(
        fontSize: 16,
        fontWeight: FontWeight.w500,
        color: PhotonixColors.textPrimary,
      ),
      // Body
      bodyLarge: TextStyle(
        fontSize: 15,
        fontWeight: FontWeight.w400,
        color: PhotonixColors.textPrimary,
      ),
      bodyMedium: TextStyle(
        fontSize: 13,
        fontWeight: FontWeight.w400,
        color: PhotonixColors.textSecondary,
      ),
      bodySmall: TextStyle(
        fontSize: 11,
        fontWeight: FontWeight.w400,
        color: PhotonixColors.textTertiary,
      ),
      // Labels — buttons, tags
      labelLarge: TextStyle(
        fontSize: 14,
        fontWeight: FontWeight.w500,
        color: PhotonixColors.textPrimary,
      ),
      labelSmall: TextStyle(
        fontSize: 10,
        fontWeight: FontWeight.w500,
        color: PhotonixColors.textSecondary,
        letterSpacing: 0.5,
      ),
    ),

    cardTheme: CardThemeData(
      color: PhotonixColors.surface,
      elevation: 0,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: const BorderSide(color: PhotonixColors.border, width: 1),
      ),
    ),

    elevatedButtonTheme: ElevatedButtonThemeData(
      style: ElevatedButton.styleFrom(
        backgroundColor: PhotonixColors.accent,
        foregroundColor: Colors.black,
        elevation: 0,
        padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 14),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
        textStyle: const TextStyle(fontSize: 15, fontWeight: FontWeight.w600),
      ),
    ),

    dividerTheme: const DividerThemeData(
      color: PhotonixColors.border,
      thickness: 1,
      space: 1,
    ),
  );
}
