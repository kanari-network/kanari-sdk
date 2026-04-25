import 'package:flutter/material.dart';
import '../../auth_client.dart';

/// Registration Screen Widget for Kanari Auth
///
/// Provides email/password registration interface with validation,
/// curve type selection, and error handling.
class KanariRegisterScreen extends StatefulWidget {
  final KanariAuthClient authClient;
  final VoidCallback? onRegistrationSuccess;

  const KanariRegisterScreen({
    super.key,
    required this.authClient,
    this.onRegistrationSuccess,
  });

  @override
  State<KanariRegisterScreen> createState() => _KanariRegisterScreenState();
}

class _KanariRegisterScreenState extends State<KanariRegisterScreen> {
  final _formKey = GlobalKey<FormState>();
  final _emailController = TextEditingController();
  final _passwordController = TextEditingController();
  final _confirmPasswordController = TextEditingController();
  bool _isLoading = false;
  String? _errorMessage;
  bool _obscurePassword = true;
  bool _obscureConfirmPassword = true;
  String _selectedCurveType = 'ed25519';

  // Available curve types with descriptions
  final List<Map<String, String>> _curveTypes = [
    {'value': 'ed25519', 'label': 'Ed25519', 'desc': 'Fast & secure (default)'},
    {
      'value': 'k256',
      'label': 'K256 (Secp256k1)',
      'desc': 'Bitcoin/Ethereum compatible',
    },
    {
      'value': 'p256',
      'label': 'P256 (NIST)',
      'desc': 'Enterprise standard (NIST P-256)',
    },
    {
      'value': 'dilithium2',
      'label': 'Dilithium2',
      'desc': 'Post-quantum, Level 2 security (~2.5KB)',
    },
    {
      'value': 'dilithium3',
      'label': 'Dilithium3',
      'desc': 'Post-quantum, Level 3 security (~4KB) - Recommended',
    },
    {
      'value': 'dilithium5',
      'label': 'Dilithium5',
      'desc': 'Post-quantum, Level 5 security (~5KB)',
    },
    {
      'value': 'sphincsplus',
      'label': 'Sphincs+ (SHA256)',
      'desc': 'Post-quantum, hash-based (~50KB)',
    },
    {
      'value': 'ed25519dilithium3',
      'label': 'Ed25519 + Dilithium3',
      'desc': 'Hybrid: Classical + Post-quantum',
    },
    {
      'value': 'k256dilithium3',
      'label': 'K256 + Dilithium3',
      'desc': 'Hybrid: Bitcoin-compatible + Post-quantum',
    },
  ];

  @override
  void dispose() {
    _emailController.dispose();
    _passwordController.dispose();
    _confirmPasswordController.dispose();
    super.dispose();
  }

  Future<void> _handleRegister() async {
    if (!_formKey.currentState!.validate()) return;

    setState(() {
      _isLoading = true;
      _errorMessage = null;
    });

    try {
      final response = await widget.authClient.register(
        email: _emailController.text.trim(),
        password: _passwordController.text,
        curveType: _selectedCurveType,
      );

      if (mounted) {
        setState(() {
          _isLoading = false;
        });

        if (response.success) {
          final walletAddr = response.data?.walletAddress;
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text(
                walletAddr != null
                    ? 'Registration successful! Wallet: ${walletAddr.substring(0, 10)}...'
                    : 'Registration successful!',
              ),
              backgroundColor: Colors.green,
              duration: const Duration(seconds: 4),
            ),
          );
          widget.onRegistrationSuccess?.call();
        } else {
          setState(() {
            _errorMessage = response.error ?? 'Registration failed';
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

  String? _validatePassword(String? value) {
    if (value == null || value.isEmpty) {
      return 'Please enter a password';
    }
    if (value.length < 8) {
      return 'Password must be at least 8 characters';
    }
    if (!value.contains(RegExp(r'[A-Z]'))) {
      return 'Password must contain at least one uppercase letter';
    }
    if (!value.contains(RegExp(r'[a-z]'))) {
      return 'Password must contain at least one lowercase letter';
    }
    if (!value.contains(RegExp(r'[0-9]'))) {
      return 'Password must contain at least one digit';
    }
    if (!value.contains(RegExp(r'[!@#$%^&*(),.?":{}|<>]'))) {
      return 'Password must contain at least one special character';
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Create Account'), centerTitle: true),
      body: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.all(24.0),
          child: Form(
            key: _formKey,
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                const SizedBox(height: 24),

                // Icon
                Icon(
                  Icons.person_add,
                  size: 80,
                  color: Theme.of(context).primaryColor,
                ),

                const SizedBox(height: 16),

                // Title
                Text(
                  'Join Kanari Network',
                  style: Theme.of(context).textTheme.headlineMedium,
                  textAlign: TextAlign.center,
                ),

                const SizedBox(height: 8),

                Text(
                  'Create your wallet and start transacting',
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
                    hintText: 'Min 8 chars, upper, lower, digit, special',
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
                    helperText: 'Must meet all requirements below',
                  ),
                  validator: _validatePassword,
                  enabled: !_isLoading,
                ),

                const SizedBox(height: 8),

                // Password Requirements
                Container(
                  padding: const EdgeInsets.all(12),
                  decoration: BoxDecoration(
                    color: Colors.blue[50],
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(color: Colors.blue[200]!),
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'Password Requirements:',
                        style: TextStyle(
                          fontWeight: FontWeight.bold,
                          color: Colors.blue[700],
                          fontSize: 12,
                        ),
                      ),
                      const SizedBox(height: 4),
                      _buildRequirement(
                        'At least 8 characters',
                        RegExp(r'.{8,}'),
                      ),
                      _buildRequirement(
                        'One uppercase letter',
                        RegExp(r'[A-Z]'),
                      ),
                      _buildRequirement(
                        'One lowercase letter',
                        RegExp(r'[a-z]'),
                      ),
                      _buildRequirement('One digit', RegExp(r'[0-9]')),
                      _buildRequirement(
                        'One special character',
                        RegExp(r'[!@#$%^&*(),.?":{}|<>]'),
                      ),
                    ],
                  ),
                ),

                const SizedBox(height: 16),

                // Confirm Password Field
                TextFormField(
                  controller: _confirmPasswordController,
                  obscureText: _obscureConfirmPassword,
                  decoration: InputDecoration(
                    labelText: 'Confirm Password',
                    hintText: 'Re-enter your password',
                    prefixIcon: const Icon(Icons.lock_outline),
                    suffixIcon: IconButton(
                      icon: Icon(
                        _obscureConfirmPassword
                            ? Icons.visibility
                            : Icons.visibility_off,
                      ),
                      onPressed: () {
                        setState(() {
                          _obscureConfirmPassword = !_obscureConfirmPassword;
                        });
                      },
                    ),
                    border: const OutlineInputBorder(),
                  ),
                  validator: (value) {
                    if (value == null || value.isEmpty) {
                      return 'Please confirm your password';
                    }
                    if (value != _passwordController.text) {
                      return 'Passwords do not match';
                    }
                    return null;
                  },
                  enabled: !_isLoading,
                ),

                const SizedBox(height: 24),

                // Curve Type Dropdown
                DropdownButtonFormField<String>(
                  value: _selectedCurveType,
                  decoration: const InputDecoration(
                    border: OutlineInputBorder(),
                    prefixIcon: Icon(Icons.security),
                    labelText: 'Cryptographic Curve Type',
                    helperText: 'Choose the algorithm for your wallet',
                  ),
                  items: _curveTypes.map((curve) {
                    return DropdownMenuItem<String>(
                      value: curve['value'],
                      child: Text(
                        curve['label']!,
                        style: const TextStyle(fontWeight: FontWeight.w500),
                      ),
                    );
                  }).toList(),
                  onChanged: _isLoading
                      ? null
                      : (value) {
                          setState(() {
                            _selectedCurveType = value!;
                          });
                        },
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

                // Register Button
                ElevatedButton(
                  onPressed: _isLoading ? null : _handleRegister,
                  style: ElevatedButton.styleFrom(
                    padding: const EdgeInsets.symmetric(vertical: 16),
                  ),
                  child: _isLoading
                      ? const SizedBox(
                          height: 20,
                          width: 20,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        )
                      : const Text(
                          'Create Account',
                          style: TextStyle(fontSize: 16),
                        ),
                ),

                const SizedBox(height: 16),

                // Login Link
                TextButton(
                  onPressed: _isLoading
                      ? null
                      : () {
                          Navigator.pop(context);
                        },
                  child: const Text('Already have an account? Login'),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildRequirement(String text, RegExp pattern) {
    final password = _passwordController.text;
    final isValid = password.isNotEmpty && pattern.hasMatch(password);

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        children: [
          Icon(
            isValid ? Icons.check_circle : Icons.radio_button_unchecked,
            size: 16,
            color: isValid ? Colors.green : Colors.grey,
          ),
          const SizedBox(width: 8),
          Text(
            text,
            style: TextStyle(
              fontSize: 11,
              color: isValid ? Colors.green[700] : Colors.grey[600],
            ),
          ),
        ],
      ),
    );
  }
}
