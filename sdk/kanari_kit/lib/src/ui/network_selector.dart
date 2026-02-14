import 'package:flutter/material.dart';
import '../../main.dart';
import 'package:provider/provider.dart';
import '../../kanari_kit.dart';

class NetworkSelector extends StatelessWidget {
  const NetworkSelector({super.key});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<WalletState>();
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
      decoration: BoxDecoration(
        color: Colors.white10,
        borderRadius: BorderRadius.circular(20),
        border: Border.all(color: Colors.white24),
      ),
      child: DropdownButtonHideUnderline(
        child: DropdownButton<KanariEnvironment>(
          value: state.environment,
          isDense: true,
          icon: const Icon(Icons.arrow_drop_down, color: Colors.blueAccent),
          items: KanariEnvironment.values.map((env) {
            return DropdownMenuItem(
              value: env,
              child: Text(
                env.name.toUpperCase(),
                style: const TextStyle(fontSize: 12, fontWeight: FontWeight.bold),
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
