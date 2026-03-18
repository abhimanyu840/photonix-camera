import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../models/camera_state.dart';

/// Tracks which pipeline stage is running and how far along it is.
/// Updated by the Rust stream handler in P7.
class ProcessingProgressNotifier extends Notifier<ProcessingProgress> {
  @override
  ProcessingProgress build() => ProcessingProgress.idle;

  void update(String stageName, double progress) {
    state = ProcessingProgress(stageName: stageName, progress: progress.clamp(0.0, 1.0));
  }

  void reset() {
    state = ProcessingProgress.idle;
  }
}

final processingProgressProvider =
    NotifierProvider<ProcessingProgressNotifier, ProcessingProgress>(
  ProcessingProgressNotifier.new,
);