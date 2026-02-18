import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:kanari_crypto/kanari_crypto.dart';
import 'src/providers/wallet_provider.dart';
import 'src/ui/screens/home_screen.dart';
import 'src/ui/screens/welcome_screen.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();
  try {
    // Initialize Kanari Crypto
    await initKanariCrypto();
    debugPrint("Kanari Crypto initialized");
  } catch (e) {
    debugPrint("Kanari Crypto init error: $e");
  }

  runApp(
    MultiProvider(
      providers: [ChangeNotifierProvider(create: (_) => WalletState())],
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
      theme: ThemeData(
        useMaterial3: true,
        colorScheme: ColorScheme.fromSeed(
          seedColor: Colors.blueAccent,
          brightness: Brightness.dark,
        ),
        appBarTheme: const AppBarTheme(
          centerTitle: false,
          elevation: 0,
          backgroundColor: Colors.transparent,
        ),
        cardTheme: CardThemeData(
          elevation: 0,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(20),
          ),
        ),
        inputDecorationTheme: InputDecorationTheme(
          border: OutlineInputBorder(borderRadius: BorderRadius.circular(12)),
          filled: true,
        ),
      ),
      home: const MainWrapper(),
    );
  }
}

class MainWrapper extends StatefulWidget {
  const MainWrapper({super.key});

  @override
  State<MainWrapper> createState() => _MainWrapperState();
}

class _MainWrapperState extends State<MainWrapper> {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.read<WalletState>().initialize();
    });
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<WalletState>();

    // Switch between screens based on wallet state
    if (state.wallet == null) {
      return const WelcomeScreen();
    }

    return const HomeScreen();
  }
}
