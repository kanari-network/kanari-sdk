import 'package:flutter/material.dart';
import 'package:kanari_kit/src/ui/screens/escrow_screen.dart';
import 'package:kanari_kit/src/ui/screens/home_screen.dart';
import 'package:kanari_kit/src/ui/widgets/transfer_bottom_sheet.dart';

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
    return Scaffold(
      body: SafeArea(
        bottom: false,
        child: IndexedStack(index: _currentIndex, children: _screens),
      ),
      bottomNavigationBar: SafeArea(
        minimum: const EdgeInsets.only(top: 0),
        child: Container(
          decoration: BoxDecoration(
            color: Theme.of(context).colorScheme.surface,
            borderRadius: const BorderRadius.vertical(top: Radius.circular(24)),
            border: Border(
              top: BorderSide(
                color: Theme.of(context).colorScheme.outline.withOpacity(0.2),
                width: 1,
              ),
            ),
            boxShadow: [
              BoxShadow(
                color: Colors.black.withOpacity(0.05),
                blurRadius: 10,
                offset: const Offset(0, -2),
              ),
            ],
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
    final colorScheme = Theme.of(context).colorScheme;

    return Expanded(
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(24),
        child: AnimatedContainer(
          duration: const Duration(milliseconds: 200),
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 12),
          decoration: BoxDecoration(
            color: isActive
                ? colorScheme.primaryContainer.withOpacity(0.3)
                : Colors.transparent,
            borderRadius: BorderRadius.circular(24),
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                isActive ? activeIcon : inactiveIcon,
                color: isActive
                    ? colorScheme.primary
                    : colorScheme.onSurfaceVariant,
                size: 24,
              ),
              const SizedBox(height: 6),
              Text(
                label,
                style: TextStyle(
                  fontSize: 12,
                  fontWeight: isActive ? FontWeight.w600 : FontWeight.w500,
                  color: isActive
                      ? colorScheme.primary
                      : colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
