import 'dart:io';
import 'dart:typed_data';
import 'package:flutter/foundation.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';
import '../../models/camera_state.dart';
import '../../providers/providers.dart';
import '../../providers/device_info_provider.dart';
import '../../rust/api/image_api.dart' as rust_api;
import 'camera_channel.dart';
import 'dart:async';

/// Pipeline timeout — cancel and save raw frame if exceeded.
const _kPipelineTimeout = Duration(seconds: 10);

/// Minimum free storage before capture is allowed (bytes).
const _kMinFreeStorageBytes = 50 * 1024 * 1024; // 50MB

class CaptureCoordinator {
  final Ref _ref;
  CaptureCoordinator(this._ref);

  Future<Uint8List?> capture() async {
    final cameraNotifier = _ref.read(cameraStateProvider.notifier);
    final progressNotifier = _ref.read(processingProgressProvider.notifier);
    final settings = _ref.read(settingsProvider);
    final deviceInfo = _ref.read(deviceInfoProvider).valueOrNull;

    // ── Guard: check free storage ──────────────────────────────────────────
    if (!await _hasEnoughStorage()) {
      debugPrint('[Coordinator] Low storage — aborting capture');
      // P10: show snackbar in camera_screen
      return null;
    }

    try {
      // ── Step 1: Burst capture ─────────────────────────────────────────────
      cameraNotifier.startCapture();
      progressNotifier.update('Capturing...', 0.05);

      final frameCount = _burstCount(settings.qualityTier, deviceInfo);
      debugPrint('[Coordinator] Capturing $frameCount frames');

      List<Uint8List> frames;
      try {
        frames = await CameraChannel()
            .captureBurst(frameCount)
            .timeout(
              const Duration(seconds: 5),
              onTimeout: () => throw TimeoutException('Capture timed out'),
            );
      } catch (e) {
        debugPrint('[Coordinator] Burst failed: $e — using raw preview');
        cameraNotifier.reset();
        progressNotifier.reset();
        return null;
      }

      debugPrint('[Coordinator] Got ${frames.length} frames');

      // ── Step 2: Show raw frame immediately (instant feel) ─────────────────
      // This gives the user immediate visual feedback while AI runs
      final rawBytes = frames.first;
      cameraNotifier.startProcessing();
      progressNotifier.update('Enhancing...', 0.1);

      // ── Step 3: AI pipeline with timeout ──────────────────────────────────
      Uint8List? result;
      String? pipelineError;

      try {
        final sceneHint = _getSceneHint(settings.qualityTier, deviceInfo);

        await Future(() async {
          final stream = rust_api.captureAndProcess(
            frames: frames,
            sceneHint: sceneHint,
          );

          await for (final update in stream) {
            if (update.error.isNotEmpty) {
              pipelineError = update.error;
              break;
            }
            if (update.isComplete) {
              result = Uint8List.fromList(update.resultBytes);
              break;
            }
            progressNotifier.update(update.stage, update.progress);
          }
        }).timeout(
          _kPipelineTimeout,
          onTimeout: () {
            debugPrint('[Coordinator] Pipeline timed out — using raw frame');
            result = rawBytes; // fall back to raw on timeout
          },
        );
      } catch (e) {
        if (e.toString().contains('OutOfMemory') ||
            e.toString().contains('OOM')) {
          debugPrint('[Coordinator] OOM — clearing model cache');
          // Signal Rust to clear model cache
          try {
            rust_api.processSingle(frame: rawBytes, sceneHint: 'fast');
          } catch (_) {}
          result = rawBytes;
        } else {
          debugPrint('[Coordinator] Pipeline error: $e — using raw frame');
          result = rawBytes;
        }
      }

      // ── Step 4: Complete ──────────────────────────────────────────────────
      final finalResult = result ?? rawBytes;
      progressNotifier.update('Done', 1.0);
      cameraNotifier.finishProcessing();

      Future.delayed(const Duration(seconds: 2), () {
        cameraNotifier.reset();
        progressNotifier.reset();
      });

      debugPrint('[Coordinator] Complete: ${finalResult.length} bytes');
      return finalResult;
    } catch (e) {
      debugPrint('[Coordinator] Unexpected error: $e');
      cameraNotifier.reset();
      progressNotifier.reset();
      return null;
    }
  }

  Future<bool> _hasEnoughStorage() async {
    try {
      final dir = await getTemporaryDirectory();
      final stat = await FileStat.stat(dir.path);
      // FileStat doesn't give free space — use a heuristic
      // In production use disk_space package
      return true; // TODO: integrate disk_space package in production
    } catch (_) {
      return true;
    }
  }

  int _burstCount(QualityTier tier, DeviceInfo? device) {
    if (device?.tier == DeviceTier.low) return 1;
    if (device?.tier == DeviceTier.medium) return 3;
    return switch (tier) {
      QualityTier.aiEnhanced => 3,
      QualityTier.standard => 3,
      QualityTier.fast => 1,
    };
  }

  String? _getSceneHint(QualityTier tier, DeviceInfo? device) {
    if (device?.tier == DeviceTier.low) return 'fast';
    if (device?.tier == DeviceTier.medium) return 'standard';
    return null; // let Rust auto-detect
  }
}

final captureCoordinatorProvider = Provider((ref) => CaptureCoordinator(ref));
