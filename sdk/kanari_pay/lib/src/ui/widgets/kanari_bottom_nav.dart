import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import 'package:kanari_pay/src/ui/screens/escrow_screen.dart';
import 'package:kanari_pay/src/ui/screens/home_screen.dart';
import 'package:kanari_pay/src/ui/screens/kanari_welcome_screen.dart';
import 'package:kanari_pay/src/ui/widgets/transfer_bottom_sheet.dart';
import 'package:kanari_pay/src/providers/wallet_provider.dart';
import 'package:kanari_pay/theme.dart';

/// Persistent Bottom Navigation Bar - แสดงทุกหน้า
class KanariBottomNav extends StatefulWidget {
  final int currentIndex;

  const KanariBottomNav({super.key, this.currentIndex = 0});

  @override
  State<KanariBottomNav> createState() => _KanariBottomNavState();
}

class _KanariBottomNavState extends State<KanariBottomNav> {
  int _currentIndex = 0;

  // มี 2 หน้า - Send เป็น popup ไม่ใช่อีกหน้า
  final List<Widget> _screens = [HomeScreen(), const EscrowScreen()];

  @override
  void initState() {
    super.initState();
    _currentIndex = widget.currentIndex;
  }

  @override
  Widget build(BuildContext context) {
    final walletState = context.watch<WalletState>();
    if (walletState.requiresUnlock) {
      return const KanariWelcomeScreen();
    }

    return Scaffold(
      body: SafeArea(
        bottom: false,
        child: IndexedStack(index: _currentIndex, children: _screens),
      ),
      bottomNavigationBar: SafeArea(
        minimum: const EdgeInsets.fromLTRB(16, 0, 16, 12),
        child: Container(
          decoration: BoxDecoration(
            color: Theme.of(context).colorScheme.surfaceContainerLowest,
            borderRadius: BorderRadius.circular(24),
            border: Border.all(
              color: Theme.of(context).colorScheme.outlineVariant,
            ),
          ),
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 8),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceEvenly,
            children: [
              _buildNavItem(
                context,
                Icons.home_rounded,
                Icons.home_outlined,
                'Home',
                _currentIndex == 0,
                () => _onTabTapped(0),
              ),
              _buildNavItem(
                context,
                Icons.send_rounded,
                Icons.send_outlined,
                'Send',
                false,
                () => _showSendSheet(),
              ),
              _buildNavItem(
                context,
                Icons.security_rounded,
                Icons.security_outlined,
                'Escrow',
                _currentIndex == 1,
                () => _onTabTapped(1),
              ),
            ],
          ),
        ),
      ),
    );
  }

  void _onTabTapped(int index) {
    setState(() {
      _currentIndex = index;
    });
  }

  void _showSendSheet() {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      backgroundColor: Colors.transparent,
      useRootNavigator: true,
      builder: (context) => const TransferBottomSheet(),
    );
  }

  Widget _buildNavItem(
    BuildContext context,
    IconData activeIcon,
    IconData inactiveIcon,
    String label,
    bool isActive,
    VoidCallback onTap,
  ) {
    return _KanariNavItem(
      activeIcon: activeIcon,
      inactiveIcon: inactiveIcon,
      label: label,
      isActive: isActive,
      onTap: onTap,
    );
  }
}

class _KanariNavItem extends StatefulWidget {
  final IconData activeIcon;
  final IconData inactiveIcon;
  final String label;
  final bool isActive;
  final VoidCallback onTap;

  const _KanariNavItem({
    required this.activeIcon,
    required this.inactiveIcon,
    required this.label,
    required this.isActive,
    required this.onTap,
  });

  @override
  State<_KanariNavItem> createState() => _KanariNavItemState();
}

class _KanariNavItemState extends State<_KanariNavItem> {
  bool _hovered = false;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final reduceMotion = MediaQuery.disableAnimationsOf(context);
    final duration = reduceMotion
        ? Duration.zero
        : const Duration(milliseconds: 220);

    return Expanded(
      child: MouseRegion(
        onEnter: (_) => setState(() => _hovered = true),
        onExit: (_) => setState(() => _hovered = false),
        child: AnimatedSlide(
          duration: duration,
          curve: Curves.easeOutCubic,
          offset: Offset(0, _hovered ? -.06 : 0),
          child: InkWell(
            onTap: widget.onTap,
            borderRadius: BorderRadius.circular(16),
            child: AnimatedContainer(
              duration: duration,
              curve: Curves.easeOutCubic,
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 12),
              decoration: BoxDecoration(
                color: widget.isActive
                    ? KanariColors.lime
                    : _hovered
                    ? colors.surfaceContainerHigh
                    : Colors.transparent,
                borderRadius: BorderRadius.circular(16),
              ),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  AnimatedScale(
                    duration: duration,
                    curve: Curves.easeOutBack,
                    scale: widget.isActive || _hovered ? 1.1 : 1,
                    child: AnimatedSwitcher(
                      duration: duration,
                      transitionBuilder: (child, animation) =>
                          RotationTransition(
                            turns: Tween<double>(
                              begin: -.08,
                              end: 0,
                            ).animate(animation),
                            child: ScaleTransition(
                              scale: animation,
                              child: child,
                            ),
                          ),
                      child: Icon(
                        widget.isActive
                            ? widget.activeIcon
                            : widget.inactiveIcon,
                        key: ValueKey(widget.isActive),
                        color: widget.isActive
                            ? KanariColors.ink
                            : colors.onSurfaceVariant,
                        size: 24,
                      ),
                    ),
                  ),
                  const SizedBox(height: 6),
                  AnimatedDefaultTextStyle(
                    duration: duration,
                    style: TextStyle(
                      fontSize: 12,
                      fontWeight: widget.isActive
                          ? FontWeight.w800
                          : FontWeight.w500,
                      color: widget.isActive
                          ? KanariColors.ink
                          : colors.onSurfaceVariant,
                    ),
                    child: Text(widget.label),
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
