/// All possible states of the capture pipeline.
/// Drives the entire camera UI — button appearance, overlays, gestures.
enum CameraMode {
  /// Camera ready, viewfinder live, shutter available
  idle,

  /// Burst frames being captured from CameraX (added P4)
  capturing,

  /// Rust classical + AI pipeline running
  processing,

  /// Photo saved, showing result preview
  done,
}

/// Which AI quality tier the user selected.
enum QualityTier {
  /// All AI models enabled (DnCNN + Zero-DCE + Real-ESRGAN + MiDaS)
  aiEnhanced,

  /// Classical pipeline only (burst stack + HDR + tone map)
  standard,

  /// Single frame, minimal processing — fastest
  fast,
}

/// A single captured photo in the gallery.
class PhotoEntry {
  final String id;
  final String filePath;
  final DateTime capturedAt;
  final CameraMode capturedWith;
  final QualityTier quality;
  final int processingTimeMs;

  /// Scene type detected by MobileNetV3 (added P7)
  final String? sceneType;

  const PhotoEntry({
    required this.id,
    required this.filePath,
    required this.capturedAt,
    required this.capturedWith,
    required this.quality,
    required this.processingTimeMs,
    this.sceneType,
  });
}

/// Progress update sent from Rust pipeline to Dart UI.
class ProcessingProgress {
  /// Human-readable stage name shown in the overlay
  final String stageName;

  /// 0.0 to 1.0
  final double progress;

  const ProcessingProgress({required this.stageName, required this.progress});

  static const idle = ProcessingProgress(stageName: '', progress: 0.0);
}
