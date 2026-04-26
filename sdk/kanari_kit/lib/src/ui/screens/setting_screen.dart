import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:shared_preferences/shared_preferences.dart';

import '../../auth_client.dart';
import '../../providers/theme_mode_provider.dart';
import '../../providers/wallet_provider.dart';
import '../widgets/app_ui.dart';

class SettingScreen extends StatelessWidget {
  const SettingScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return AppGradientScaffold(
      appBar: AppBar(title: const Text('Settings'), centerTitle: true),
      body: ListView(
        padding: const EdgeInsets.all(20),
        children: [
          const AppSectionTitle('Appearance'),
          const SizedBox(height: 12),
          _ThemeModeCard(),
          const SizedBox(height: 24),
          const AppSectionTitle('Security'),
          const SizedBox(height: 12),
          _SettingsTile(
            icon: Icons.pin_rounded,
            title: 'Change PIN',
            subtitle: 'Update the 6-digit PIN for this device',
            onTap: () => _showChangePinDialog(context),
          ),
          const SizedBox(height: 24),
          const AppSectionTitle('Session'),
          const SizedBox(height: 12),
          _SettingsTile(
            icon: Icons.logout_rounded,
            title: 'Logout',
            subtitle: 'Current session only',
            iconColor: colorScheme.error,
            onTap: () => _handleLogout(context, false),
          ),
          const SizedBox(height: 12),
          _SettingsTile(
            icon: Icons.phonelink_erase_rounded,
            title: 'Logout All Devices',
            subtitle: 'All active sessions',
            iconColor: colorScheme.error,
            onTap: () => _handleLogout(context, true),
          ),
        ],
      ),
    );
  }

  void _showChangePinDialog(BuildContext context) {
    final state = context.read<WalletState>();

    showDialog(
      context: context,
      builder: (_) => AppPinChangeDialog(
        onSubmit: (oldPin, newPin) => state.changePin(oldPin, newPin),
      ),
    );
  }

  Future<void> _handleLogout(BuildContext context, bool logoutAll) async {
    final authClient = context.read<KanariAuthClient>();
    final walletState = context.read<WalletState>();

    final confirmed = await showDialog<bool>(
      context: context,
      builder: (_) => AppConfirmationDialog(
        icon: logoutAll ? Icons.phonelink_erase_rounded : Icons.logout_rounded,
        title: logoutAll ? 'Logout All Devices?' : 'Logout?',
        content: logoutAll
            ? 'This will log out all active sessions on all devices.'
            : 'This will log out your current session.',
        confirmLabel: 'Logout',
        isDestructive: true,
      ),
    );

    if (confirmed != true || !context.mounted) return;

    if (logoutAll) {
      await authClient.logoutAll();
    } else {
      await authClient.logout();
    }

    final prefs = await SharedPreferences.getInstance();
    await prefs.remove('session_id');
    await prefs.remove('user_email');
    await prefs.remove('wallet_address');

    walletState.logout();

    if (!context.mounted) return;

    Navigator.of(context).pushNamedAndRemoveUntil('/', (route) => false);
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(
          logoutAll ? 'Logged out from all devices' : 'Logged out successfully',
        ),
      ),
    );
  }
}

class _ThemeModeCard extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final themeModeProvider = context.watch<ThemeModeProvider>();

    return AppPanel(
      padding: EdgeInsets.zero,
      child: Column(
        children: [
          RadioListTile<ThemeMode>(
            value: ThemeMode.system,
            groupValue: themeModeProvider.themeMode,
            title: const Text('System'),
            subtitle: const Text('Follow device appearance'),
            onChanged: (value) {
              if (value != null) {
                themeModeProvider.setThemeMode(value);
              }
            },
          ),
          const Divider(height: 1),
          RadioListTile<ThemeMode>(
            value: ThemeMode.light,
            groupValue: themeModeProvider.themeMode,
            title: const Text('Light'),
            subtitle: const Text('Always use the light theme'),
            onChanged: (value) {
              if (value != null) {
                themeModeProvider.setThemeMode(value);
              }
            },
          ),
          const Divider(height: 1),
          RadioListTile<ThemeMode>(
            value: ThemeMode.dark,
            groupValue: themeModeProvider.themeMode,
            title: const Text('Dark'),
            subtitle: const Text('Always use the dark theme'),
            onChanged: (value) {
              if (value != null) {
                themeModeProvider.setThemeMode(value);
              }
            },
          ),
        ],
      ),
    );
  }
}

class _SettingsTile extends StatelessWidget {
  final IconData icon;
  final String title;
  final String subtitle;
  final Color? iconColor;
  final VoidCallback onTap;

  const _SettingsTile({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.onTap,
    this.iconColor,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Material(
      color: colorScheme.surfaceContainerHigh,
      borderRadius: BorderRadius.circular(20),
      child: InkWell(
        borderRadius: BorderRadius.circular(20),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(18),
          child: Row(
            children: [
              Icon(icon, color: iconColor ?? colorScheme.primary),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      title,
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      subtitle,
                      style: Theme.of(context).textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
              Icon(
                Icons.chevron_right_rounded,
                color: colorScheme.onSurfaceVariant,
              ),
            ],
          ),
        ),
      ),
    );
  }
}
