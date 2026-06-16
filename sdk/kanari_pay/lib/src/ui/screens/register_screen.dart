import 'package:flutter/material.dart';

import '../../client/auth_client.dart';
import '../widgets/app_ui.dart';

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
  bool _obscurePassword = true;
  bool _obscureConfirmPassword = true;
  String? _errorMessage;
  String _selectedCurveType = 'ed25519';

  final List<Map<String, String>> _curveTypes = [
    {'value': 'ed25519', 'label': 'Ed25519', 'desc': 'Fast and secure default'},
    {
      'value': 'k256',
      'label': 'K256',
      'desc': 'Bitcoin and Ethereum compatible',
    },
    {'value': 'p256', 'label': 'P256', 'desc': 'NIST enterprise standard'},
    {
      'value': 'ed25519dilithium3',
      'label': 'Ed25519 + Dilithium3',
      'desc': 'Hybrid classical and PQ',
    },
    {
      'value': 'k256dilithium3',
      'label': 'K256 + Dilithium3',
      'desc': 'Hybrid EVM and PQ',
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

      if (!mounted) return;

      setState(() {
        _isLoading = false;
      });

      if (response.success) {
        final walletAddr = response.data?.walletAddress;
        showAppSuccessSnackBar(
          context,
          walletAddr != null
              ? 'Registration successful! Wallet: ${walletAddr.substring(0, 10)}...'
              : 'Registration successful!',
        );
        widget.onRegistrationSuccess?.call();
      } else {
        setState(() {
          _errorMessage = response.error ?? 'Registration failed';
        });
      }
    } catch (e) {
      if (!mounted) return;

      setState(() {
        _isLoading = false;
        _errorMessage = 'Network error: $e';
      });
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
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final selectedCurve = _curveTypes.firstWhere(
      (curve) => curve['value'] == _selectedCurveType,
    );

    return AppGradientScaffold(
      appBar: AppBar(title: const Text('Register'), centerTitle: true),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(16),
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 520),
            child: Form(
              key: _formKey,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const SizedBox(height: 16),
                  const AuthHero(
                    icon: Icons.person_add_alt_1_rounded,
                    title: 'Create Your Account',
                    subtitle:
                        'Set up your Kanari account and choose the wallet cryptography that fits your use case.',
                  ),
                  const SizedBox(height: 16),
                  AppPanel(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        const AppSectionTitle('Account Details'),
                        const SizedBox(height: 12),
                        TextFormField(
                          controller: _emailController,
                          keyboardType: TextInputType.emailAddress,
                          enabled: !_isLoading,
                          decoration: const InputDecoration(
                            labelText: 'Email',
                            hintText: 'user@example.com',
                            prefixIcon: Icon(Icons.alternate_email_rounded),
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
                        ),
                        const SizedBox(height: 16),
                        TextFormField(
                          controller: _passwordController,
                          obscureText: _obscurePassword,
                          enabled: !_isLoading,
                          onChanged: (_) => setState(() {}),
                          decoration: InputDecoration(
                            labelText: 'Password',
                            hintText: 'Strong password required',
                            prefixIcon: const Icon(Icons.lock_rounded),
                            suffixIcon: IconButton(
                              icon: Icon(
                                _obscurePassword
                                    ? Icons.visibility_rounded
                                    : Icons.visibility_off_rounded,
                              ),
                              onPressed: () {
                                setState(() {
                                  _obscurePassword = !_obscurePassword;
                                });
                              },
                            ),
                          ),
                          validator: _validatePassword,
                        ),
                        const SizedBox(height: 12),
                        _RequirementsCard(password: _passwordController.text),
                        const SizedBox(height: 16),
                        TextFormField(
                          controller: _confirmPasswordController,
                          obscureText: _obscureConfirmPassword,
                          enabled: !_isLoading,
                          decoration: InputDecoration(
                            labelText: 'Confirm Password',
                            hintText: 'Re-enter your password',
                            prefixIcon: const Icon(Icons.verified_user_rounded),
                            suffixIcon: IconButton(
                              icon: Icon(
                                _obscureConfirmPassword
                                    ? Icons.visibility_rounded
                                    : Icons.visibility_off_rounded,
                              ),
                              onPressed: () {
                                setState(() {
                                  _obscureConfirmPassword =
                                      !_obscureConfirmPassword;
                                });
                              },
                            ),
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
                        ),
                      ],
                    ),
                  ),
                  const SizedBox(height: 18),
                  AppPanel(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: [
                        const AppSectionTitle('Wallet Cryptography'),
                        const SizedBox(height: 8),
                        Text(
                          selectedCurve['desc']!,
                          style: theme.textTheme.bodyMedium?.copyWith(
                            color: colorScheme.onSurfaceVariant,
                          ),
                        ),
                        const SizedBox(height: 16),
                        DropdownButtonFormField<String>(
                          initialValue: _selectedCurveType,
                          decoration: const InputDecoration(
                            labelText: 'Curve Type',
                            prefixIcon: Icon(Icons.security_rounded),
                          ),
                          items: _curveTypes.map((curve) {
                            return DropdownMenuItem<String>(
                              value: curve['value'],
                              child: Text(curve['label']!),
                            );
                          }).toList(),
                          onChanged: _isLoading
                              ? null
                              : (value) {
                                  if (value == null) return;
                                  setState(() {
                                    _selectedCurveType = value;
                                  });
                                },
                        ),
                      ],
                    ),
                  ),
                  if (_errorMessage != null) ...[
                    const SizedBox(height: 16),
                    AppErrorBanner(message: _errorMessage!),
                  ],
                  const SizedBox(height: 24),
                  AppWideButton(
                    onPressed: _isLoading ? null : _handleRegister,
                    icon: Icons.person_add_alt_1_rounded,
                    label: 'Create Account',
                    child: _isLoading
                        ? const SizedBox(
                            width: 20,
                            height: 20,
                            child: CircularProgressIndicator(strokeWidth: 2),
                          )
                        : const Text('Create Account'),
                  ),
                  const SizedBox(height: 12),
                  AppWideButton(
                    onPressed: _isLoading ? null : () => Navigator.pop(context),
                    icon: Icons.login_rounded,
                    label: 'Already have an account? Login',
                    style: AppWideButtonStyle.text,
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _RequirementsCard extends StatelessWidget {
  final String password;

  const _RequirementsCard({required this.password});

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest,
        borderRadius: BorderRadius.circular(12),
        border: Border.all(color: colorScheme.outline.withValues(alpha: 0.2)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'Password requirements',
            style: Theme.of(
              context,
            ).textTheme.labelLarge?.copyWith(fontWeight: FontWeight.w700),
          ),
          const SizedBox(height: 8),
          _RequirementRow(
            text: 'At least 8 characters',
            isValid: RegExp(r'.{8,}').hasMatch(password),
          ),
          _RequirementRow(
            text: 'One uppercase letter',
            isValid: RegExp(r'[A-Z]').hasMatch(password),
          ),
          _RequirementRow(
            text: 'One lowercase letter',
            isValid: RegExp(r'[a-z]').hasMatch(password),
          ),
          _RequirementRow(
            text: 'One digit',
            isValid: RegExp(r'[0-9]').hasMatch(password),
          ),
          _RequirementRow(
            text: 'One special character',
            isValid: RegExp(r'[!@#$%^&*(),.?":{}|<>]').hasMatch(password),
          ),
        ],
      ),
    );
  }
}

class _RequirementRow extends StatelessWidget {
  final String text;
  final bool isValid;

  const _RequirementRow({required this.text, required this.isValid});

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 3),
      child: Row(
        children: [
          Icon(
            isValid
                ? Icons.check_circle_rounded
                : Icons.radio_button_unchecked_rounded,
            size: 16,
            color: isValid ? Colors.green : colorScheme.onSurfaceVariant,
          ),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              text,
              style: TextStyle(
                fontSize: 12,
                color: isValid ? Colors.green : colorScheme.onSurfaceVariant,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
