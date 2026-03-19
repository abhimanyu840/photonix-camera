import 'dart:typed_data';
import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../models/camera_state.dart';
import '../../providers/providers.dart';
import '../../rust/api/image_api.dart' as rust_api;
import 'camera_channel.dart';

/// Orchestrates the full capture pipeline:
///   1. Determine burst frame count from scene/settings
///   2. Tell CameraX to capture N frames (via MethodChannel)
///   3. Pass each frame through the Rust bridge (zero-copy)
///   4. Update progress state so the UI shows stage labels
///   5. Return the final processed image bytes
///
/// Phase 4: captureBurst → raw frames returned (no Rust processing yet)
/// Phase 5: classical pipeline added in Rust
/// Phase 7: AI routing + scene classification added
class CaptureCoordinator {
  final Ref _ref;

  CaptureCoordinator(this._ref);

  Future<Uint8List?> capture() async {
    final cameraNotifier = _ref.read(cameraStateProvider.notifier);
    final progressNotifier = _ref.read(processingProgressProvider.notifier);
    final settings = _ref.read(settingsProvider);

    try {
      // ── Step 1: Capture burst frames ──────────────────────────────────────
      cameraNotifier.startCapture();
      progressNotifier.update('Capturing...', 0.05);

      final frameCount = _burstCount(settings.qualityTier);
      debugPrint('[Coordinator] Capturing $frameCount frames');

      final frames = await CameraChannel().captureBurst(frameCount);
      debugPrint('[Coordinator] Got ${frames.length} frames from CameraX');

      // ── Step 2: Pass first frame through Rust bridge (P4 validation) ──────
      // Phase 5 will replace this with the full classical pipeline
      // Phase 7 will add AI routing
      cameraNotifier.startProcessing();
      progressNotifier.update('Processing...', 0.3);

      final Uint8List firstFrame = frames.first;
      final dto = rust_api.PipelineConfigDto(
        runBurstStack: frameCount > 1,
        runHdrMerge: false,
        runExposureLift: true,
        exposureLiftAmount: 0.1,
        saturation: 1.1,
        toneMapping: 'aces',
        sharpenAmount: 0.4,
        jpegQuality: 95,
      );
      final Uint8List result = await rust_api.processBurst(
        frames: frames,
        config: dto,
      );

      progressNotifier.update('Saving...', 0.95);
      await Future.delayed(const Duration(milliseconds: 100));

      cameraNotifier.finishProcessing();
      progressNotifier.reset();

      debugPrint('[Coordinator] Pipeline complete — ${result.length} bytes');
      return result;
    } catch (e) {
      debugPrint('[Coordinator] Capture failed: $e');
      cameraNotifier.reset();
      progressNotifier.reset();
      return null;
    }
  }

  /// Burst frame count per quality tier.
  /// Night: 7 (√7 = 2.6× SNR improvement)
  /// Portrait/Standard: 3
  /// Fast: 1 (single frame)
  int _burstCount(QualityTier tier) => switch (tier) {
    QualityTier.aiEnhanced => 3,
    QualityTier.standard => 3,
    QualityTier.fast => 1,
  };
}

/// Provider so CaptureCoordinator can be accessed anywhere.
final captureCoordinatorProvider = Provider((ref) => CaptureCoordinator(ref));
