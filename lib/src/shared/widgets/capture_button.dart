import 'package:flutter/material.dart';
import '../theme/app_theme.dart';

class CaptureButton extends StatefulWidget {
  final VoidCallback? onPressed;
  final bool isPortrait;

  const CaptureButton({super.key, this.onPressed, this.isPortrait = false});

  @override
  State<CaptureButton> createState() => _CaptureButtonState();
}

class _CaptureButtonState extends State<CaptureButton>
    with SingleTickerProviderStateMixin {
  late AnimationController _ctrl;
  late Animation<double> _scale;

  @override
  void initState() {
    super.initState();
    _ctrl = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 100),
      reverseDuration: const Duration(milliseconds: 200),
    );
    _scale = Tween(
      begin: 1.0,
      end: 0.88,
    ).animate(CurvedAnimation(parent: _ctrl, curve: Curves.easeIn));
  }

  @override
  void dispose() {
    _ctrl.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final isEnabled = widget.onPressed != null;

    return GestureDetector(
      onTapDown: isEnabled ? (_) => _ctrl.forward() : null,
      onTapUp: isEnabled
          ? (_) {
              _ctrl.reverse();
              widget.onPressed?.call();
            }
          : null,
      onTapCancel: () => _ctrl.reverse(),
      child: ScaleTransition(
        scale: _scale,
        child: Container(
          width: 72,
          height: 72,
          decoration: BoxDecoration(
            shape: BoxShape.circle,
            border: Border.all(
              color: isEnabled ? Colors.white : Colors.white38,
              width: 3,
            ),
          ),
          child: Padding(
            padding: const EdgeInsets.all(4),
            child: Container(
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: isEnabled
                    ? (widget.isPortrait ? PhotonixColors.accent : Colors.white)
                    : Colors.white38,
              ),
            ),
          ),
        ),
      ),
    );
  }
}
