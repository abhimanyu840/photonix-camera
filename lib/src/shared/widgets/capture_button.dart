import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../models/camera_state.dart';
import '../../providers/providers.dart';
import '../../shared/theme/app_theme.dart';

/// The main shutter button.
///
/// States:
///   idle       — white ring, white fill, tappable
///   capturing  — pulse animation, no tap
///   processing — progress ring animating, no tap
///   done       — green fill briefly before resetting
class CaptureButton extends ConsumerStatefulWidget {
  final VoidCallback? onPressed;
  final double size;

  const CaptureButton({super.key, this.onPressed, this.size = 76});

  @override
  ConsumerState<CaptureButton> createState() => _CaptureButtonState();
}

class _CaptureButtonState extends ConsumerState<CaptureButton>
    with TickerProviderStateMixin {
  late AnimationController _pulseController;
  late AnimationController _progressController;
  late Animation<double> _pulseAnimation;

  @override
  void initState() {
    super.initState();
    _pulseController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 600),
    )..repeat(reverse: true);

    _progressController = AnimationController(
      vsync: this,
      duration: const Duration(seconds: 2),
    )..repeat();

    _pulseAnimation = Tween<double>(begin: 0.95, end: 1.05).animate(
      CurvedAnimation(parent: _pulseController, curve: Curves.easeInOut),
    );
  }

  @override
  void dispose() {
    _pulseController.dispose();
    _progressController.dispose();
    super.dispose();
  }

  void _handleTap() {
    HapticFeedback.mediumImpact();
    widget.onPressed?.call();
  }

  @override
  Widget build(BuildContext context) {
    final mode = ref.watch(cameraStateProvider);
    final progress = ref.watch(
      processingProgressProvider.select((p) => p.progress),
    );

    final bool enabled = mode == CameraMode.idle || mode == CameraMode.done;

    return GestureDetector(
      onTap: enabled ? _handleTap : null,
      child: SizedBox(
        width: widget.size + 16,
        height: widget.size + 16,
        child: Stack(
          alignment: Alignment.center,
          children: [
            // ── Progress ring (visible during processing) ──────────────────
            if (mode == CameraMode.processing)
              SizedBox(
                width: widget.size + 12,
                height: widget.size + 12,
                child: AnimatedBuilder(
                  animation: _progressController,
                  builder: (_, __) => CustomPaint(
                    painter: _ProgressRingPainter(
                      progress: progress,
                      color: PhotonixColors.processing,
                    ),
                  ),
                ),
              ),

            // ── Button body ───────────────────────────────────────────────
            AnimatedBuilder(
              animation: _pulseAnimation,
              builder: (_, child) {
                final scale = mode == CameraMode.capturing
                    ? _pulseAnimation.value
                    : 1.0;
                return Transform.scale(scale: scale, child: child);
              },
              child: AnimatedContainer(
                duration: const Duration(milliseconds: 200),
                width: widget.size,
                height: widget.size,
                decoration: BoxDecoration(
                  shape: BoxShape.circle,
                  color: _fillColor(mode),
                  border: Border.all(
                    color: _ringColor(mode),
                    width: mode == CameraMode.idle ? 3 : 2,
                  ),
                ),
                child: _icon(mode),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Color _fillColor(CameraMode mode) => switch (mode) {
    CameraMode.idle => Colors.white,
    CameraMode.capturing => PhotonixColors.accentDim,
    CameraMode.processing => Colors.transparent,
    CameraMode.done => PhotonixColors.success,
  };

  Color _ringColor(CameraMode mode) => switch (mode) {
    CameraMode.idle => Colors.white,
    CameraMode.capturing => PhotonixColors.accent,
    CameraMode.processing => PhotonixColors.processing.withOpacity(0.4),
    CameraMode.done => PhotonixColors.success,
  };

  Widget? _icon(CameraMode mode) => switch (mode) {
    CameraMode.processing => const SizedBox.shrink(),
    CameraMode.done => const Icon(Icons.check, color: Colors.white, size: 28),
    _ => null,
  };
}

class _ProgressRingPainter extends CustomPainter {
  final double progress;
  final Color color;

  const _ProgressRingPainter({required this.progress, required this.color});

  @override
  void paint(Canvas canvas, Size size) {
    final center = Offset(size.width / 2, size.height / 2);
    final radius = (size.width - 4) / 2;

    // Track
    canvas.drawCircle(
      center,
      radius,
      Paint()
        ..color = color.withOpacity(0.15)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 2.5,
    );

    // Progress arc
    canvas.drawArc(
      Rect.fromCircle(center: center, radius: radius),
      -math.pi / 2,
      2 * math.pi * progress,
      false,
      Paint()
        ..color = color
        ..style = PaintingStyle.stroke
        ..strokeWidth = 2.5
        ..strokeCap = StrokeCap.round,
    );
  }

  @override
  bool shouldRepaint(_ProgressRingPainter old) =>
      old.progress != progress || old.color != color;
}
