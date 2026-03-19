import 'dart:typed_data';
import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../models/camera_state.dart';
import '../../providers/providers.dart';
import '../../rust/api/image_api.dart' as rust_api;
import 'camera_channel.dart';

/// Orchestrates the full capture pipeline:
///   1. CameraX burst capture (via MethodChannel)
///   2. Scene detection + AI pipeline (via Rust bridge with StreamSink)
///   3. Live progress updates → ProcessingProgressNotifier
///   4. Atomic preview swap on completion
class CaptureCoordinator {
  final Ref _ref;

  CaptureCoordinator(this._ref);

  Future<Uint8List?> capture() async {
    final cameraNotifier = _ref.read(cameraStateProvider.notifier);
    final progressNotifier = _ref.read(processingProgressProvider.notifier);
    final settings = _ref.read(settingsProvider);

    try {
      // ── Step 1: Burst capture ─────────────────────────────────────────────
      cameraNotifier.startCapture();
      progressNotifier.update('Capturing...', 0.05);

      final frameCount = _burstCount(settings.qualityTier);
      debugPrint('[Coordinator] Capturing $frameCount frames');

      final frames = await CameraChannel().captureBurst(frameCount);
      debugPrint('[Coordinator] Got ${frames.length} frames from CameraX');

      // ── Step 2: Start AI pipeline with StreamSink progress ────────────────
      cameraNotifier.startProcessing();

      Uint8List? result;
      String? errorMsg;

      // captureAndProcess returns a Stream<ProcessingUpdate>
      final stream = rust_api.captureAndProcess(
        frames: frames,
        sceneHint: null, // let Rust detect scene automatically
      );

      await for (final update in stream) {
        if (update.error.isNotEmpty) {
          errorMsg = update.error;
          break;
        }

        if (update.isComplete) {
          result = Uint8List.fromList(update.resultBytes);
          progressNotifier.update('Done', 1.0);
          break;
        } else {
          // Live stage update → drives ProcessingOverlay widget
          progressNotifier.update(update.stage, update.progress);
        }
      }

      if (errorMsg != null) {
        debugPrint('[Coordinator] Pipeline error: $errorMsg');
        cameraNotifier.reset();
        progressNotifier.reset();
        return null;
      }

      // ── Step 3: Finish ────────────────────────────────────────────────────
      cameraNotifier.finishProcessing();

      // Reset to idle after 2 seconds so user can tap shutter again
      Future.delayed(const Duration(seconds: 2), () {
        cameraNotifier.reset();
        progressNotifier.reset();
      });

      debugPrint('[Coordinator] Complete — ${result?.length ?? 0} bytes');
      return result;
    } catch (e) {
      debugPrint('[Coordinator] Capture failed: $e');
      cameraNotifier.reset();
      progressNotifier.reset();
      return null;
    }
  }

  /// Burst frame count per quality tier.
  int _burstCount(QualityTier tier) => switch (tier) {
    QualityTier.aiEnhanced => 3,
    QualityTier.standard => 3,
    QualityTier.fast => 1,
  };
}

final captureCoordinatorProvider = Provider((ref) => CaptureCoordinator(ref));
