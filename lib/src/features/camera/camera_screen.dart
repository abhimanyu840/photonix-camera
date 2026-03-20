import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';
import '../../models/camera_state.dart';
import '../../providers/providers.dart';
import '../../shared/theme/app_theme.dart';
import '../../shared/widgets/capture_button.dart';
import '../../shared/widgets/processing_overlay.dart';
import '../../rust/api/image_api.dart' as rust_api;
import 'camera_preview_widget.dart';
import 'capture_coordinator.dart';

class CameraScreen extends ConsumerStatefulWidget {
  const CameraScreen({super.key});

  @override
  ConsumerState<CameraScreen> createState() => _CameraScreenState();
}

class _CameraScreenState extends ConsumerState<CameraScreen> {
  @override
  void initState() {
    super.initState();
    // Pre-warm scene classifier in background after first frame renders
    WidgetsBinding.instance.addPostFrameCallback((_) => _prewarmModels());
  }

  Future<void> _prewarmModels() async {
    try {
      final dir = await getApplicationDocumentsDirectory();
      final modelPath = '${dir.path}/models/mobilenet_scene_int8.onnx';
      rust_api.prewarmSceneClassifier(modelPath: modelPath);
    } catch (e) {
      debugPrint('[Camera] Prewarm skipped: $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    final mode = ref.watch(cameraStateProvider);
    final settings = ref.watch(settingsProvider);

    return Scaffold(
      backgroundColor: PhotonixColors.background,
      body: Stack(
        fit: StackFit.expand,
        children: [
          // ── Camera preview ─────────────────────────────────────────────────
          const CameraPreviewWidget(),

          // ── Processing overlay ─────────────────────────────────────────────
          const ProcessingOverlay(),

          // ── Top bar ────────────────────────────────────────────────────────
          SafeArea(
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 12),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  _QualityBadge(tier: settings.qualityTier),
                  const Icon(
                    Icons.settings_outlined,
                    color: Colors.white,
                    size: 22,
                  ),
                ],
              ),
            ),
          ),

          // ── Bottom controls ─────────────────────────────────────────────────
          Align(
            alignment: Alignment.bottomCenter,
            child: SafeArea(
              child: Padding(
                padding: const EdgeInsets.only(bottom: 32),
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                  crossAxisAlignment: CrossAxisAlignment.center,
                  children: [
                    const SizedBox(width: 56),
                    CaptureButton(onPressed: () => _onShutter()),
                    const SizedBox(width: 56),
                  ],
                ),
              ),
            ),
          ),

          // ── Debug mode overlay ──────────────────────────────────────────────
          if (settings.debugModeEnabled)
            Positioned(
              bottom: 120,
              left: 0,
              right: 0,
              child: _DebugOverlay(mode: mode),
            ),
        ],
      ),
    );
  }

  void _onShutter() {
    final mode = ref.read(cameraStateProvider);
    if (mode != CameraMode.idle) return;

    final coordinator = ref.read(captureCoordinatorProvider);
    coordinator.capture().then((result) {
      if (result != null) {
        debugPrint('[Camera] Capture complete: ${result.length} bytes');
        // P10: save to gallery
        Future.delayed(const Duration(seconds: 2), () {
          ref.read(cameraStateProvider.notifier).reset();
        });
      }
    });
  }
}

// ── Sub-widgets ───────────────────────────────────────────────────────────────

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

class _DebugOverlay extends StatelessWidget {
  final CameraMode mode;
  const _DebugOverlay({required this.mode});

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 4),
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
    );
  }
}
