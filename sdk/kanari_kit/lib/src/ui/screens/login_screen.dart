import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:cryptography/cryptography.dart';
import 'package:provider/provider.dart';
import '../../auth_client.dart';
import '../../kanaricurve.dart';
import '../../providers/wallet_provider.dart';

/// Login Screen Widget for Kanari Auth
///
/// Provides email/password login interface with validation and error handling.
class KanariLoginScreen extends StatefulWidget {
  final KanariAuthClient authClient;
  final VoidCallback? onLoginSuccess;

  const KanariLoginScreen({
    super.key,
    required this.authClient,
    this.onLoginSuccess,
  });

  @override
  State<KanariLoginScreen> createState() => _KanariLoginScreenState();
}

class _KanariLoginScreenState extends State<KanariLoginScreen> {
  static const int _defaultKdfIterations = 120000;
  final _formKey = GlobalKey<FormState>();
  final _emailController = TextEditingController();
  final _passwordController = TextEditingController();
  bool _isLoading = false;
  String? _errorMessage;
  bool _obscurePassword = true;

  @override
  void dispose() {
    _emailController.dispose();
    _passwordController.dispose();
    super.dispose();
  }

  Future<void> _handleLogin() async {
    if (!_formKey.currentState!.validate()) return;

    setState(() {
      _isLoading = true;
      _errorMessage = null;
    });

    try {
      final response = await widget.authClient.login(
        email: _emailController.text.trim(),
        password: _passwordController.text,
      );

      if (mounted) {
        setState(() {
          _isLoading = false;
        });

        if (response.success) {
          final walletAddress = response.data?.walletAddress;
          var matchedLocalWallet = false;
          if (walletAddress != null && walletAddress.isNotEmpty) {
            matchedLocalWallet = await context
                .read<WalletState>()
                .syncWalletWithAddress(
              walletAddress,
            );
          }

          if (!matchedLocalWallet &&
              response.data?.encryptedPrivateKey != null &&
              response.data!.encryptedPrivateKey!.isNotEmpty &&
              response.data?.curveType != null) {
            try {
              final privateKey = await _decryptPrivateKey(
                response.data!.encryptedPrivateKey!,
                _passwordController.text,
              );
              final curve = KanariCurve.fromString(response.data!.curveType!);
              await context.read<WalletState>().importFromPrivateKey(
                privateKey,
                curve: curve,
                pin: '',
              );

              if (walletAddress != null && walletAddress.isNotEmpty) {
                matchedLocalWallet = await context
                    .read<WalletState>()
                    .syncWalletWithAddress(walletAddress);
              }
            } catch (e) {
              if (mounted) {
                setState(() {
                  _errorMessage = 'Wallet import failed: $e';
                });
              }
            }
          }

          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text(
                matchedLocalWallet
                    ? 'Login successful!'
                    : 'Login successful, but this wallet is not stored on this device yet.',
              ),
              backgroundColor: matchedLocalWallet
                  ? Colors.green
                  : Colors.orange,
            ),
          );
          widget.onLoginSuccess?.call();
        } else {
          setState(() {
            _errorMessage = response.error ?? 'Login failed';
          });
        }
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _isLoading = false;
          _errorMessage = 'Network error: $e';
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Kanari Login'), centerTitle: true),
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24.0),
          child: Form(
            key: _formKey,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                const SizedBox(height: 32),

                // Logo or Icon
                Icon(
                  Icons.account_circle,
                  size: 80,
                  color: Theme.of(context).primaryColor,
                ),

                const SizedBox(height: 16),

                // Title
                Text(
                  'Welcome Back',
                  style: Theme.of(context).textTheme.headlineMedium,
                  textAlign: TextAlign.center,
                ),

                const SizedBox(height: 8),

                Text(
                  'Login to access your Kanari wallet',
                  style: Theme.of(
                    context,
                  ).textTheme.bodyLarge?.copyWith(color: Colors.grey[600]),
                  textAlign: TextAlign.center,
                ),

                const SizedBox(height: 32),

                // Email Field
                TextFormField(
                  controller: _emailController,
                  keyboardType: TextInputType.emailAddress,
                  decoration: const InputDecoration(
                    labelText: 'Email',
                    hintText: 'user@example.com',
                    prefixIcon: Icon(Icons.email),
                    border: OutlineInputBorder(),
                  ),
                  validator: (value) {
                    if (value == null || value.isEmpty) {
                      return 'Please enter your email';
                    }
                    if (!value.contains('@')) {
                      return 'Please enter a valid email';
                    }
                    return null;
                  },
                  enabled: !_isLoading,
                ),

                const SizedBox(height: 16),

                // Password Field
                TextFormField(
                  controller: _passwordController,
                  obscureText: _obscurePassword,
                  decoration: InputDecoration(
                    labelText: 'Password',
                    hintText: 'Enter your password',
                    prefixIcon: const Icon(Icons.lock),
                    suffixIcon: IconButton(
                      icon: Icon(
                        _obscurePassword
                            ? Icons.visibility
                            : Icons.visibility_off,
                      ),
                      onPressed: () {
                        setState(() {
                          _obscurePassword = !_obscurePassword;
                        });
                      },
                    ),
                    border: const OutlineInputBorder(),
                  ),
                  validator: (value) {
                    if (value == null || value.isEmpty) {
                      return 'Please enter your password';
                    }
                    if (value.length < 8) {
                      return 'Password must be at least 8 characters';
                    }
                    return null;
                  },
                  enabled: !_isLoading,
                ),

                if (_errorMessage != null) ...[
                  const SizedBox(height: 16),
                  Container(
                    padding: const EdgeInsets.all(12),
                    decoration: BoxDecoration(
                      color: Colors.red[50],
                      borderRadius: BorderRadius.circular(8),
                      border: Border.all(color: Colors.red[200]!),
                    ),
                    child: Row(
                      children: [
                        Icon(Icons.error_outline, color: Colors.red[700]),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            _errorMessage!,
                            style: TextStyle(color: Colors.red[700]),
                          ),
                        ),
                      ],
                    ),
                  ),
                ],

                const SizedBox(height: 24),

                // Login Button
                ElevatedButton(
                  onPressed: _isLoading ? null : _handleLogin,
                  style: ElevatedButton.styleFrom(
                    padding: const EdgeInsets.symmetric(vertical: 16),
                  ),
                  child: _isLoading
                      ? const SizedBox(
                          height: 20,
                          width: 20,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Text('Login', style: TextStyle(fontSize: 16)),
                ),

                const SizedBox(height: 16),

                // Register Link
                TextButton(
                  onPressed: _isLoading
                      ? null
                      : () {
                          Navigator.pushNamed(context, '/register');
                        },
                  child: const Text("Don't have an account? Register"),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  Future<String> _decryptPrivateKey(
    String encryptedPayload,
    String password,
  ) async {
    final payload = jsonDecode(encryptedPayload) as Map<String, dynamic>;
    final encryptedBytes = base64Decode(payload['ciphertext'] as String);
    final nonce = base64Decode(payload['nonce'] as String);
    final salt = base64Decode(payload['salt'] as String);
    final iterations =
        (payload['iterations'] as num?)?.toInt() ?? _defaultKdfIterations;
    if (encryptedBytes.length < 16) {
      throw Exception('Encrypted payload is invalid');
    }

    final keyBytes = await _deriveKey(password, salt, iterations);
    final algorithm = AesGcm.with256bits();
    final cipherText = encryptedBytes.sublist(0, encryptedBytes.length - 16);
    final macBytes = encryptedBytes.sublist(encryptedBytes.length - 16);
    final secretBox = SecretBox(
      cipherText,
      nonce: nonce,
      mac: Mac(macBytes),
    );

    final clearText = await algorithm.decrypt(
      secretBox,
      secretKey: SecretKey(keyBytes),
    );

    return utf8.decode(clearText);
  }

  Future<List<int>> _deriveKey(String password, List<int> salt, int iterations) async {
    final sha256 = Sha256();
    var block = await sha256.hash([...utf8.encode(password), ...salt]);

    for (var i = 1; i < iterations; i++) {
      block = await sha256.hash([
        ...block.bytes,
        ...utf8.encode(password),
        ...salt,
      ]);
    }

    return block.bytes;
  }
}
