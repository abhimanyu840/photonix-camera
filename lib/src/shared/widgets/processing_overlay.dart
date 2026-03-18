import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../models/camera_state.dart';
import '../../providers/providers.dart';
import '../../shared/theme/app_theme.dart';

/// Shown over the viewfinder while the Rust pipeline runs.
/// Non-blocking: the camera preview remains visible underneath.
/// Fades in/out with AnimatedOpacity.
class ProcessingOverlay extends ConsumerWidget {
  const ProcessingOverlay({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final mode = ref.watch(cameraStateProvider);
    final progress = ref.watch(processingProgressProvider);
    final isVisible = mode == CameraMode.processing;

    return AnimatedOpacity(
      opacity: isVisible ? 1.0 : 0.0,
      duration: const Duration(milliseconds: 200),
      child: IgnorePointer(
        ignoring: !isVisible,
        child: Container(
          decoration: const BoxDecoration(
            gradient: LinearGradient(
              begin: Alignment.topCenter,
              end: Alignment.bottomCenter,
              colors: [Colors.transparent, Color(0xCC000000)],
              stops: [0.5, 1.0],
            ),
          ),
          child: Align(
            alignment: Alignment.bottomCenter,
            child: Padding(
              padding: const EdgeInsets.fromLTRB(24, 0, 24, 120),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  // Stage label
                  AnimatedSwitcher(
                    duration: const Duration(milliseconds: 300),
                    child: Text(
                      progress.stageName.isEmpty
                          ? 'Processing...'
                          : progress.stageName,
                      key: ValueKey(progress.stageName),
                      style: const TextStyle(
                        color: Colors.white,
                        fontSize: 14,
                        fontWeight: FontWeight.w500,
                        letterSpacing: -0.2,
                      ),
                    ),
                  ),
                  const SizedBox(height: 10),
                  // Progress bar
                  ClipRRect(
                    borderRadius: BorderRadius.circular(2),
                    child: LinearProgressIndicator(
                      value: progress.progress == 0 ? null : progress.progress,
                      backgroundColor: Colors.white.withOpacity(0.2),
                      valueColor: const AlwaysStoppedAnimation<Color>(
                        PhotonixColors.processing,
                      ),
                      minHeight: 3,
                    ),
                  ),
                  const SizedBox(height: 6),
                  // Percentage
                  if (progress.progress > 0)
                    Text(
                      '${(progress.progress * 100).toInt()}%',
                      style: const TextStyle(
                        color: PhotonixColors.textSecondary,
                        fontSize: 11,
                      ),
                    ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
