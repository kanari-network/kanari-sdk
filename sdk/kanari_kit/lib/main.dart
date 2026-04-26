import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:kanari_crypto/kanari_crypto.dart';
import 'src/providers/wallet_provider.dart';
import 'src/ui/screens/home_screen.dart';
import 'src/ui/screens/welcome_screen.dart';
import 'src/ui/screens/login_screen.dart';
import 'src/ui/screens/register_screen.dart';
import 'src/ui/screens/setting_screen.dart';
import 'src/auth_client.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  try {
    await initKanariCrypto();
    debugPrint("✅ Kanari Crypto initialized");
  } catch (e) {
    debugPrint("❌ Kanari Crypto init error: $e");
  }

  // สร้าง authClient instance พร้อม baseUrl
  final authClient = KanariAuthClient('http://localhost:3000');

  // Restore session ก่อนเริ่ม app
  await _restoreSession(authClient);

  runApp(
    MultiProvider(
      providers: [
        ChangeNotifierProvider(create: (_) => authClient),
        ChangeNotifierProvider(create: (_) => WalletState()..initialize()),
      ],
      child: const KanariApp(),
    ),
  );
}

/// Restore session from SharedPreferences
Future<void> _restoreSession(KanariAuthClient authClient) async {
  try {
    final prefs = await SharedPreferences.getInstance();

    final sessionId = prefs.getString('session_id');
    final userEmail = prefs.getString('user_email');
    final walletAddress = prefs.getString('wallet_address');

    if (sessionId != null &&
        sessionId.isNotEmpty &&
        userEmail != null &&
        userEmail.isNotEmpty &&
        walletAddress != null &&
        walletAddress.isNotEmpty) {
      debugPrint("🔄 Restoring session from SharedPreferences...");
      debugPrint("   - User: $userEmail");
      debugPrint("   - Wallet: $walletAddress");

      // Set session information WITHOUT validating with server yet
      // Validation will happen when user performs an action
      authClient.setSession(
        sessionId: sessionId,
        userEmail: userEmail,
        walletAddress: walletAddress,
      );

      debugPrint("✅ Session restored locally for $userEmail");

      // Optional: Try to validate in background (don't block app startup)
      // If validation fails, we'll handle it later when user interacts
      _validateSessionInBackground(authClient, prefs);
    } else {
      debugPrint("ℹ️ No saved session found in SharedPreferences");
    }
  } catch (e) {
    debugPrint("❌ Session restore error: $e");
    // Don't clear session on error - let user stay logged in
  }
}

/// Validate session in background (non-blocking)
Future<void> _validateSessionInBackground(
  KanariAuthClient authClient,
  SharedPreferences prefs,
) async {
  try {
    debugPrint("🔍 Validating session in background...");
    final response = await authClient.validateSession();

    if (!response.success || !(response.data?.valid ?? false)) {
      debugPrint("⚠️ Session validation failed - session may be expired");
      // Don't clear session immediately - let user know when they try to use it
      debugPrint(
        "   User can still access app, but may need to re-login for actions",
      );
    } else {
      debugPrint("✅ Session validated successfully");
    }
  } catch (e) {
    // Network error or server not available - keep session anyway
    debugPrint("⚠️ Background validation skipped (server may be offline): $e");
    debugPrint("   Session kept locally - will validate on next action");
  }
}

/// Save session to SharedPreferences
Future<void> _saveSession(KanariAuthClient authClient) async {
  try {
    final prefs = await SharedPreferences.getInstance();

    if (authClient.sessionId != null &&
        authClient.userEmail != null &&
        authClient.walletAddress != null) {
      await prefs.setString('session_id', authClient.sessionId!);
      await prefs.setString('user_email', authClient.userEmail!);
      await prefs.setString('wallet_address', authClient.walletAddress!);
      debugPrint("💾 Session saved for: ${authClient.userEmail}");
    } else {
      debugPrint("⚠️ Cannot save session: missing required fields");
    }
  } catch (e) {
    debugPrint("❌ Session save error: $e");
  }
}

class KanariApp extends StatelessWidget {
  const KanariApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Kanari Wallet',
      debugShowCheckedModeBanner: false,
      themeMode: ThemeMode.system, // รองรับทั้งโหมดมืดและสว่างตามระบบ
      // --- Light Theme ---
      theme: ThemeData(
        useMaterial3: true,
        colorScheme: ColorScheme.fromSeed(
          seedColor: Colors.blueAccent, // สีม่วงจะดูเป็น M3 มากกว่าน้ำเงินเดิม
          brightness: Brightness.light,
        ),
        appBarTheme: const AppBarTheme(
          centerTitle: true,
          scrolledUnderElevation: 0, // ป้องกันสี AppBar เปลี่ยนเมื่อไถหน้าจอ
          backgroundColor: Colors.transparent,
        ),
        cardTheme: CardThemeData(
          elevation: 0,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(24),
          ),
          color: const ColorScheme.light().surfaceVariant.withOpacity(
            0.3,
          ), // การใช้พื้นผิวแบบ M3
        ),
        inputDecorationTheme: InputDecorationTheme(
          filled: true,
          border: OutlineInputBorder(
            borderRadius: BorderRadius.circular(16),
            borderSide: BorderSide.none,
          ),
          contentPadding: const EdgeInsets.symmetric(
            horizontal: 16,
            vertical: 16,
          ),
        ),
      ),

      // --- Dark Theme ---
      darkTheme: ThemeData(
        useMaterial3: true,
        colorScheme: ColorScheme.fromSeed(
          seedColor: Colors.blueAccent,
          brightness: Brightness.dark,
        ),
        appBarTheme: const AppBarTheme(
          centerTitle: true,
          scrolledUnderElevation: 0,
          backgroundColor: Colors.transparent,
        ),
        cardTheme: CardThemeData(
          elevation: 0,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(24),
          ),
        ),
        inputDecorationTheme: InputDecorationTheme(
          filled: true,
          border: OutlineInputBorder(
            borderRadius: BorderRadius.circular(16),
            borderSide: BorderSide.none,
          ),
        ),
      ),

      initialRoute: '/',
      routes: {
        '/': (context) {
          final authClient = context.watch<KanariAuthClient>();
          final walletState = context.watch<WalletState>();
          final canEnterHome =
              authClient.isAuthenticated ||
              (walletState.isUnlocked && walletState.hasWallet);

          debugPrint(
            "🔄 Route '/' check: isAuthenticated=${authClient.isAuthenticated}, isUnlocked=${walletState.isUnlocked}, hasWallet=${walletState.hasWallet}",
          );

          // เข้า Home ได้ทั้งแบบ login แล้ว หรือปลดล็อก local wallet แล้ว
          if (canEnterHome) {
            debugPrint(
              authClient.isAuthenticated
                  ? "✅ Authenticated → Navigate to HomeScreen"
                  : "✅ Local wallet unlocked → Navigate to HomeScreen",
            );
            return const HomeScreen();
          }

          debugPrint("ℹ️ Stay on WelcomeScreen");
          return const WelcomeScreen();
        },
        '/login': (context) {
          final authClient = context.read<KanariAuthClient>();
          return KanariLoginScreen(
            authClient: authClient,
            onLoginSuccess: () async {
              await _saveSession(authClient);
              if (context.mounted) {
                Navigator.of(
                  context,
                ).pushNamedAndRemoveUntil('/', (route) => false);
              }
            },
          );
        },
        '/register': (context) {
          final authClient = context.read<KanariAuthClient>();
          return KanariRegisterScreen(
            authClient: authClient,
            onRegistrationSuccess: () async {
              await _saveSession(authClient);
              if (context.mounted) {
                Navigator.of(
                  context,
                ).pushNamedAndRemoveUntil('/', (route) => false);
              }
            },
          );
        },
        '/home': (context) {
          debugPrint("🏠 Navigating to HomeScreen");
          return const HomeScreen();
        },
        '/settings': (context) {
          debugPrint("Settings screen opened");
          return const SettingScreen();
        },
      },
    );
  }
}
