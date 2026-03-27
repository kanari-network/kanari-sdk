import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:kanari_crypto/kanari_crypto.dart';
import 'src/providers/wallet_provider.dart';
import 'src/ui/screens/home_screen.dart';
import 'src/ui/screens/welcome_screen.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  try {
    await initKanariCrypto();
    debugPrint("✅ Kanari Crypto initialized");
  } catch (e) {
    debugPrint("❌ Kanari Crypto init error: $e");
  }

  runApp(
    MultiProvider(
      providers: [
        ChangeNotifierProvider(
          create: (_) => WalletState()..initialize(),
        ), // โหลดข้อมูลทันทีที่สร้าง
      ],
      child: const KanariApp(),
    ),
  );
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
    );
  }
}

class MainWrapper extends StatelessWidget {
  const MainWrapper({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<WalletState>();

    // 👉 เช็คว่า "ถ้าแอปถูกล็อกอยู่ (isUnlocked = false) หรือ ไม่มีกระเป๋า" ให้ไปหน้า WelcomeScreen
    if (!state.isUnlocked || state.wallet == null) {
      return const WelcomeScreen();
    }

    return const HomeScreen();
  }
}
