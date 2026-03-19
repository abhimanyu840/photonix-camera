import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../models/camera_state.dart';
import '../../providers/providers.dart';
import '../../shared/theme/app_theme.dart';
import '../../shared/widgets/capture_button.dart';
import '../../shared/widgets/processing_overlay.dart';
// Add this import at the top of camera_screen.dart:
import 'camera_preview_widget.dart';
import 'capture_coordinator.dart';

/// Main camera screen — viewfinder + controls.
/// CameraX preview added in Phase 4.
/// Pipeline wired in Phase 7.
class CameraScreen extends ConsumerWidget {
  const CameraScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final mode = ref.watch(cameraStateProvider);
    final settings = ref.watch(settingsProvider);

    return Scaffold(
      backgroundColor: PhotonixColors.background,
      body: Stack(
        fit: StackFit.expand,
        children: [
          // ── Viewfinder placeholder (replaced with CameraX preview in P4) ──
          const CameraPreviewWidget(),

          // ── Processing overlay (fades in during AI pipeline) ───────────────
          const ProcessingOverlay(),

          // ── Top bar ───────────────────────────────────────────────────────
          SafeArea(
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  // Quality tier badge
                  _QualityBadge(tier: settings.qualityTier),
                  // Flash / settings (P4)
                  const Icon(
                    Icons.settings_outlined,
                    color: Colors.white,
                    size: 22,
                  ),
                ],
              ),
            ),
          ),

          // ── Bottom controls ───────────────────────────────────────────────
          Align(
            alignment: Alignment.bottomCenter,
            child: SafeArea(
              child: Padding(
                padding: const EdgeInsets.only(bottom: 32),
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                  crossAxisAlignment: CrossAxisAlignment.center,
                  children: [
                    // Gallery thumbnail (P4)
                    const SizedBox(width: 56),
                    // Shutter
                    CaptureButton(onPressed: () => _onShutter(ref)),
                    // Flip camera (P4)
                    const SizedBox(width: 56),
                  ],
                ),
              ),
            ),
          ),

          // ── Debug mode indicator ──────────────────────────────────────────
          if (settings.debugModeEnabled)
            Positioned(
              bottom: 120,
              left: 0,
              right: 0,
              child: Center(
                child: Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 10,
                    vertical: 4,
                  ),
                  decoration: BoxDecoration(
                    color: Colors.black54,
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text(
                    'MODE: ${mode.name.toUpperCase()}',
                    style: const TextStyle(
                      color: PhotonixColors.accent,
                      fontSize: 10,
                      fontFamily: 'monospace',
                    ),
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }

  void _onShutter(WidgetRef ref) {
    final mode = ref.read(cameraStateProvider);
    if (mode != CameraMode.idle) return; // ignore tap during processing

    final coordinator = ref.read(captureCoordinatorProvider);
    coordinator.capture().then((result) {
      if (result != null) {
        debugPrint('Capture complete: ${result.length} bytes');
        // P10: save to gallery
        Future.delayed(const Duration(seconds: 2), () {
          ref.read(cameraStateProvider.notifier).reset();
        });
      }
    });
  }
}

class _QualityBadge extends StatelessWidget {
  final QualityTier tier;
  const _QualityBadge({required this.tier});

  @override
  Widget build(BuildContext context) {
    final (label, color) = switch (tier) {
      QualityTier.aiEnhanced => ('AI', PhotonixColors.accent),
      QualityTier.standard => ('STD', PhotonixColors.textSecondary),
      QualityTier.fast => ('FAST', PhotonixColors.textTertiary),
    };
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
      decoration: BoxDecoration(
        color: color.withOpacity(0.15),
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: color.withOpacity(0.4)),
      ),
      child: Text(
        label,
        style: TextStyle(
          color: color,
          fontSize: 10,
          fontWeight: FontWeight.w600,
          letterSpacing: 0.8,
        ),
      ),
    );
  }
}
