import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../models/camera_state.dart';

/// Drives the entire capture UI.
/// Notifier (not StateNotifier) — Riverpod 2.x recommended API.
class CameraStateNotifier extends Notifier<CameraMode> {
  @override
  CameraMode build() => CameraMode.idle;

  void startCapture() {
    state = CameraMode.capturing;
  }

  void startProcessing() {
    state = CameraMode.processing;
  }

  void finishProcessing() {
    state = CameraMode.done;
  }

  void reset() {
    state = CameraMode.idle;
  }
}

final cameraStateProvider = NotifierProvider<CameraStateNotifier, CameraMode>(
  CameraStateNotifier.new,
);
