import 'package:flutter/material.dart';
import '../providers/wallet_provider.dart';
import 'package:provider/provider.dart';
import '../../kanari_kit.dart';

class NetworkSelector extends StatelessWidget {
  const NetworkSelector({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<WalletState>();
    final theme = Theme.of(context);

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceVariant.withOpacity(0.5),
        borderRadius: BorderRadius.circular(20),
        border: Border.all(color: theme.colorScheme.outline.withOpacity(0.2)),
      ),
      child: DropdownButtonHideUnderline(
        child: DropdownButton<KanariEnvironment>(
          value: state.environment,
          isDense: true,
          icon: Icon(Icons.arrow_drop_down, color: theme.colorScheme.primary),
          items: KanariEnvironment.values.map((env) {
            return DropdownMenuItem(
              value: env,
              child: Text(
                env.name.toUpperCase(),
                style: theme.textTheme.labelSmall?.copyWith(
                  fontWeight: FontWeight.bold,
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
            );
          }).toList(),
          onChanged: (env) {
            if (env != null) state.setEnvironment(env);
          },
        ),
      ),
    );
  }
}
