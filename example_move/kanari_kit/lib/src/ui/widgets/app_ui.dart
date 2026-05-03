import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

Future<T?> showAppModalSheet<T>({
  required BuildContext context,
  required WidgetBuilder builder,
  bool isScrollControlled = true,
  bool useSafeArea = true,
  bool showDragHandle = false,
  bool isDismissible = true,
  bool enableDrag = true,
  ShapeBorder? shape,
}) {
  return showModalBottomSheet<T>(
    context: context,
    isScrollControlled: isScrollControlled,
    useSafeArea: useSafeArea,
    showDragHandle: showDragHandle,
    isDismissible: isDismissible,
    enableDrag: enableDrag,
    backgroundColor: Theme.of(context).colorScheme.surface,
    shape:
        shape ??
        (showDragHandle
            ? const RoundedRectangleBorder(
                borderRadius: BorderRadius.vertical(top: Radius.circular(32)),
              )
            : null),
    builder: builder,
  );
}

class AppGradientScaffold extends StatelessWidget {
  final PreferredSizeWidget? appBar;
  final Widget body;
  final Color? backgroundColor;

  const AppGradientScaffold({
    super.key,
    this.appBar,
    required this.body,
    this.backgroundColor,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Scaffold(
      backgroundColor: backgroundColor ?? colorScheme.surface,
      appBar: appBar,
      body: SizedBox.expand(
        child: DecoratedBox(
          decoration: BoxDecoration(
            gradient: LinearGradient(
              begin: Alignment.topCenter,
              end: Alignment.bottomCenter,
              colors: [
                colorScheme.surface,
                colorScheme.primaryContainer.withOpacity(0.12),
              ],
            ),
          ),
          child: SafeArea(child: body),
        ),
      ),
    );
  }
}

class AppPanel extends StatelessWidget {
  final Widget child;
  final EdgeInsetsGeometry padding;

  const AppPanel({
    super.key,
    required this.child,
    this.padding = const EdgeInsets.all(24),
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Container(
      padding: padding,
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest.withOpacity(0.55),
        borderRadius: BorderRadius.circular(28),
        border: Border.all(color: colorScheme.outline.withOpacity(0.12)),
      ),
      child: child,
    );
  }
}

class AppSectionTitle extends StatelessWidget {
  final String title;

  const AppSectionTitle(this.title, {super.key});

  @override
  Widget build(BuildContext context) {
    return Text(
      title,
      style: Theme.of(
        context,
      ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700),
    );
  }
}

class AuthHero extends StatelessWidget {
  final IconData icon;
  final String title;
  final String subtitle;

  const AuthHero({
    super.key,
    required this.icon,
    required this.title,
    required this.subtitle,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Column(
      children: [
        Container(
          padding: const EdgeInsets.all(24),
          decoration: BoxDecoration(
            color: colorScheme.primaryContainer,
            borderRadius: BorderRadius.circular(28),
          ),
          child: Icon(icon, size: 48, color: colorScheme.onPrimaryContainer),
        ),
        const SizedBox(height: 20),
        Text(
          title,
          style: theme.textTheme.headlineMedium?.copyWith(
            fontWeight: FontWeight.w800,
            letterSpacing: -0.5,
          ),
          textAlign: TextAlign.center,
        ),
        const SizedBox(height: 8),
        Text(
          subtitle,
          style: theme.textTheme.bodyLarge?.copyWith(
            color: colorScheme.onSurfaceVariant,
          ),
          textAlign: TextAlign.center,
        ),
      ],
    );
  }
}

class AppErrorBanner extends StatelessWidget {
  final String message;

  const AppErrorBanner({super.key, required this.message});

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Container(
      padding: const EdgeInsets.all(14),
      decoration: BoxDecoration(
        color: colorScheme.errorContainer.withOpacity(0.85),
        borderRadius: BorderRadius.circular(16),
      ),
      child: Row(
        children: [
          Icon(
            Icons.error_outline_rounded,
            color: colorScheme.onErrorContainer,
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Text(
              message,
              style: TextStyle(color: colorScheme.onErrorContainer),
            ),
          ),
        ],
      ),
    );
  }
}

class AppLabeledDivider extends StatelessWidget {
  final String label;

  const AppLabeledDivider({super.key, this.label = 'or'});

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return Row(
      children: [
        Expanded(child: Divider(color: colorScheme.outline.withOpacity(0.3))),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          child: Text(
            label,
            style: TextStyle(color: colorScheme.onSurfaceVariant, fontSize: 12),
          ),
        ),
        Expanded(child: Divider(color: colorScheme.outline.withOpacity(0.3))),
      ],
    );
  }
}

class AppAccountSummaryPanel extends StatelessWidget {
  final String title;
  final String? subtitle;
  final Widget? trailing;

  const AppAccountSummaryPanel({
    super.key,
    required this.title,
    this.subtitle,
    this.trailing,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return AppPanel(
      padding: const EdgeInsets.all(16),
      child: Row(
        children: [
          CircleAvatar(
            backgroundColor: colorScheme.primaryContainer,
            child: Icon(
              Icons.account_circle,
              color: colorScheme.onPrimaryContainer,
              size: 32,
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  style: theme.textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.bold,
                  ),
                  overflow: TextOverflow.ellipsis,
                ),
                if (subtitle != null) ...[
                  const SizedBox(height: 4),
                  Text(
                    subtitle!,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ],
            ),
          ),
          if (trailing != null) trailing!,
        ],
      ),
    );
  }
}

class AppWideButton extends StatelessWidget {
  final VoidCallback? onPressed;
  final IconData icon;
  final String label;
  final AppWideButtonStyle style;
  final Widget? child;

  const AppWideButton({
    super.key,
    required this.onPressed,
    required this.icon,
    required this.label,
    this.style = AppWideButtonStyle.primary,
    this.child,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final buttonChild =
        child ??
        Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              icon,
              color: style == AppWideButtonStyle.text
                  ? colorScheme.primary
                  : null,
            ),
            const SizedBox(width: 8),
            Text(
              label,
              style: style == AppWideButtonStyle.text
                  ? TextStyle(
                      color: colorScheme.primary,
                      fontWeight: FontWeight.bold,
                    )
                  : null,
            ),
          ],
        );

    switch (style) {
      case AppWideButtonStyle.primary:
        return child == null
            ? FilledButton.icon(
                onPressed: onPressed,
                icon: Icon(icon),
                label: Text(label),
                style: FilledButton.styleFrom(
                  minimumSize: const Size(double.infinity, 56),
                ),
              )
            : FilledButton(
                onPressed: onPressed,
                style: FilledButton.styleFrom(
                  minimumSize: const Size(double.infinity, 56),
                ),
                child: child,
              );
      case AppWideButtonStyle.tonal:
        return child == null
            ? FilledButton.tonalIcon(
                onPressed: onPressed,
                icon: Icon(icon),
                label: Text(label),
                style: FilledButton.styleFrom(
                  minimumSize: const Size(double.infinity, 56),
                ),
              )
            : FilledButton.tonal(
                onPressed: onPressed,
                style: FilledButton.styleFrom(
                  minimumSize: const Size(double.infinity, 56),
                ),
                child: child,
              );
      case AppWideButtonStyle.outlined:
        return child == null
            ? OutlinedButton.icon(
                onPressed: onPressed,
                icon: Icon(icon),
                label: Text(label),
                style: OutlinedButton.styleFrom(
                  minimumSize: const Size(double.infinity, 56),
                ),
              )
            : OutlinedButton(
                onPressed: onPressed,
                style: OutlinedButton.styleFrom(
                  minimumSize: const Size(double.infinity, 56),
                ),
                child: child,
              );
      case AppWideButtonStyle.text:
        return TextButton(
          onPressed: onPressed,
          style: TextButton.styleFrom(
            minimumSize: const Size(double.infinity, 56),
          ),
          child: buttonChild,
        );
    }
  }
}

enum AppWideButtonStyle { primary, tonal, outlined, text }

class AppConfirmationDialog extends StatelessWidget {
  final IconData? icon;
  final Color? iconColor;
  final String title;
  final String content;
  final String confirmLabel;
  final bool isDestructive;

  const AppConfirmationDialog({
    super.key,
    this.icon,
    this.iconColor,
    required this.title,
    required this.content,
    this.confirmLabel = 'Confirm',
    this.isDestructive = false,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return AlertDialog(
      icon: icon != null
          ? Icon(
              icon,
              color: iconColor ?? (isDestructive ? colorScheme.error : null),
            )
          : null,
      title: Text(title),
      content: Text(content),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, false),
          child: const Text('Cancel'),
        ),
        FilledButton(
          style: isDestructive
              ? FilledButton.styleFrom(backgroundColor: colorScheme.error)
              : null,
          onPressed: () => Navigator.pop(context, true),
          child: Text(confirmLabel),
        ),
      ],
    );
  }
}

class AppPinChangeDialog extends StatelessWidget {
  final Future<bool> Function(String oldPin, String newPin) onSubmit;
  final String title;
  final String successMessage;
  final String failureMessage;

  const AppPinChangeDialog({
    super.key,
    required this.onSubmit,
    this.title = 'Change PIN',
    this.successMessage = 'PIN changed successfully',
    this.failureMessage = 'Failed to change PIN (Old PIN incorrect?)',
  });

  @override
  Widget build(BuildContext context) {
    final oldPinController = TextEditingController();
    final newPinController = TextEditingController();
    final confirmPinController = TextEditingController();

    return AlertDialog(
      icon: const Icon(Icons.pin_rounded),
      title: Text(title),
      content: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            _pinField(oldPinController, 'Current PIN'),
            const SizedBox(height: 12),
            _pinField(newPinController, 'New PIN'),
            const SizedBox(height: 12),
            _pinField(confirmPinController, 'Confirm New PIN'),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: () async {
            final oldPin = oldPinController.text;
            final newPin = newPinController.text;
            final confirmPin = confirmPinController.text;

            if (oldPin.length != 6 ||
                newPin.length != 6 ||
                newPin != confirmPin) {
              ScaffoldMessenger.of(context).showSnackBar(
                SnackBar(
                  content: const Text('Invalid PIN or PINs do not match'),
                  backgroundColor: Theme.of(context).colorScheme.error,
                ),
              );
              return;
            }

            Navigator.pop(context);
            final success = await onSubmit(oldPin, newPin);

            if (!context.mounted) return;

            ScaffoldMessenger.of(context).showSnackBar(
              SnackBar(
                content: Text(success ? successMessage : failureMessage),
                backgroundColor: success
                    ? Theme.of(context).colorScheme.primary
                    : Theme.of(context).colorScheme.error,
              ),
            );
          },
          child: const Text('Update'),
        ),
      ],
    );
  }

  Widget _pinField(TextEditingController controller, String label) {
    return TextField(
      controller: controller,
      obscureText: true,
      keyboardType: TextInputType.number,
      inputFormatters: [FilteringTextInputFormatter.digitsOnly],
      maxLength: 6,
      decoration: InputDecoration(labelText: label),
    );
  }
}

class AppPinEntrySheet extends StatefulWidget {
  final String title;
  final String subtitle;
  final ValueChanged<String> onComplete;
  final int pinLength;

  const AppPinEntrySheet({
    super.key,
    required this.title,
    required this.subtitle,
    required this.onComplete,
    this.pinLength = 6,
  });

  @override
  State<AppPinEntrySheet> createState() => _AppPinEntrySheetState();
}

class _AppPinEntrySheetState extends State<AppPinEntrySheet> {
  String _enteredPin = '';

  void _handleNumberPressed(String number) {
    if (_enteredPin.length < widget.pinLength) {
      setState(() {
        _enteredPin += number;
      });
      if (_enteredPin.length == widget.pinLength) {
        Future.delayed(const Duration(milliseconds: 200), () {
          if (mounted) {
            Navigator.pop(context);
            widget.onComplete(_enteredPin);
          }
        });
      }
    }
  }

  void _handleBackspace() {
    if (_enteredPin.isNotEmpty) {
      setState(() {
        _enteredPin = _enteredPin.substring(0, _enteredPin.length - 1);
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;

    return Scaffold(
      backgroundColor: Colors.transparent,
      appBar: AppBar(
        backgroundColor: Colors.transparent,
        elevation: 0,
        leading: CloseButton(color: colorScheme.onSurface),
      ),
      body: SafeArea(
        child: Column(
          children: [
            const SizedBox(height: 16),
            Text(
              widget.title,
              style: theme.textTheme.headlineMedium?.copyWith(
                fontWeight: FontWeight.bold,
                color: colorScheme.onSurface,
              ),
            ),
            const SizedBox(height: 12),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 32.0),
              child: Text(
                widget.subtitle,
                textAlign: TextAlign.center,
                style: theme.textTheme.bodyLarge?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
            ),
            const Spacer(),
            _AppPinCirclesDisplay(
              length: _enteredPin.length,
              totalLength: widget.pinLength,
            ),
            const Spacer(),
            _AppCustomNumberPad(
              onNumberPressed: _handleNumberPressed,
              onBackspacePressed: _handleBackspace,
            ),
            const SizedBox(height: 16),
          ],
        ),
      ),
    );
  }
}

class _AppPinCirclesDisplay extends StatelessWidget {
  final int length;
  final int totalLength;

  const _AppPinCirclesDisplay({
    required this.length,
    required this.totalLength,
  });

  @override
  Widget build(BuildContext context) {
    final primaryColor = Theme.of(context).colorScheme.primary;
    final outlineColor = Theme.of(context).colorScheme.outlineVariant;

    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: List.generate(totalLength, (index) {
        final isFilled = index < length;
        return Padding(
          padding: const EdgeInsets.symmetric(horizontal: 10.0),
          child: AnimatedContainer(
            duration: const Duration(milliseconds: 150),
            width: 20,
            height: 20,
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              color: isFilled ? primaryColor : Colors.transparent,
              border: Border.all(
                color: isFilled ? primaryColor : outlineColor,
                width: 2,
              ),
            ),
          ),
        );
      }),
    );
  }
}

class _AppCustomNumberPad extends StatelessWidget {
  final ValueChanged<String> onNumberPressed;
  final VoidCallback onBackspacePressed;

  const _AppCustomNumberPad({
    required this.onNumberPressed,
    required this.onBackspacePressed,
  });

  @override
  Widget build(BuildContext context) {
    final numbers = [
      ['1', '2', '3'],
      ['4', '5', '6'],
      ['7', '8', '9'],
    ];

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 32.0),
      child: Column(
        children: [
          for (var row in numbers) ...[
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceEvenly,
              children: row
                  .map(
                    (number) => _AppNumberButton(
                      number: number,
                      onPressed: () => onNumberPressed(number),
                    ),
                  )
                  .toList(),
            ),
            const SizedBox(height: 16),
          ],
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceEvenly,
            children: [
              const SizedBox(width: 80, height: 80),
              _AppNumberButton(
                number: '0',
                onPressed: () => onNumberPressed('0'),
              ),
              SizedBox(
                width: 80,
                height: 80,
                child: IconButton(
                  onPressed: onBackspacePressed,
                  icon: const Icon(Icons.backspace_outlined),
                  iconSize: 28,
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _AppNumberButton extends StatelessWidget {
  final String number;
  final VoidCallback onPressed;

  const _AppNumberButton({required this.number, required this.onPressed});

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final theme = Theme.of(context);

    return Container(
      width: 80,
      height: 80,
      decoration: BoxDecoration(
        color: colorScheme.surfaceVariant.withOpacity(0.3),
        shape: BoxShape.circle,
      ),
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          onTap: onPressed,
          customBorder: const CircleBorder(),
          child: Center(
            child: Text(
              number,
              style: theme.textTheme.headlineMedium?.copyWith(
                fontWeight: FontWeight.w600,
                color: colorScheme.onSurface,
              ),
            ),
          ),
        ),
      ),
    );
  }
}
