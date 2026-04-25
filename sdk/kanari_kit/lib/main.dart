import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:kanari_crypto/kanari_crypto.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'src/providers/wallet_provider.dart';
import 'src/auth_client.dart';
import 'src/ui/screens/home_screen.dart';
import 'src/ui/screens/welcome_screen.dart';
import 'src/ui/screens/login_screen.dart';
import 'src/ui/screens/register_screen.dart';

// Auth API base URL - change this to your run-auth server
const String AUTH_API_URL = 'http://localhost:3000';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  try {
    await initKanariCrypto();
    debugPrint("✅ Kanari Crypto initialized");
  } catch (e) {
    debugPrint("❌ Kanari Crypto init error: $e");
  }

  // Initialize auth client
  final authClient = KanariAuthClient(AUTH_API_URL);

  // Restore session if exists
  await _restoreSession(authClient);

  runApp(
    MultiProvider(
      providers: [
        ChangeNotifierProvider(create: (_) => WalletState()..initialize()),
        ChangeNotifierProvider.value(value: authClient),
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

    if (sessionId != null && userEmail != null && walletAddress != null) {
      authClient.setSession(
        sessionId: sessionId,
        userEmail: userEmail,
        walletAddress: walletAddress,
      );

      // Validate restored session
      final response = await authClient.validateSession();
      if (!response.success || !(response.data?.valid ?? false)) {
        debugPrint("⚠️ Restored session is invalid, clearing...");
        authClient.clearSession();
        await prefs.remove('session_id');
        await prefs.remove('user_email');
        await prefs.remove('wallet_address');
      } else {
        debugPrint("✅ Session restored successfully for $userEmail");
      }
    }
  } catch (e) {
    debugPrint("❌ Session restore error: $e");
  }
}

/// Save session to SharedPreferences
Future<void> _saveSession(KanariAuthClient authClient) async {
  try {
    final prefs = await SharedPreferences.getInstance();
    if (authClient.sessionId != null) {
      await prefs.setString('session_id', authClient.sessionId!);
      await prefs.setString('user_email', authClient.userEmail!);
      await prefs.setString('wallet_address', authClient.walletAddress!);
      debugPrint("💾 Session saved");
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

      home: const MainWrapper(),

      // Routes for navigation
      routes: {
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
      },
    );
  }
}

class MainWrapper extends StatelessWidget {
  const MainWrapper({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<WalletState>();
    final authClient = context.watch<KanariAuthClient>();

    // Check authentication first
    if (!authClient.isAuthenticated) {
      return KanariLoginScreen(
        authClient: authClient,
        onLoginSuccess: () async {
          await _saveSession(authClient);
          // No need to call setState - ChangeNotifier will trigger rebuild automatically
        },
      );
    }

    // If authenticated but wallet not loaded yet
    if (!state.isUnlocked || state.wallet == null) {
      return const WelcomeScreen();
    }

    return const HomeScreen();
  }
}
