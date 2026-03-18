import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../shared/theme/app_theme.dart';

/// Single photo viewer with before/after swipe comparison.
/// Full implementation in Phase 10.
class PhotoViewerScreen extends ConsumerStatefulWidget {
  final String photoId;
  const PhotoViewerScreen({super.key, required this.photoId});

  @override
  ConsumerState<PhotoViewerScreen> createState() => _PhotoViewerScreenState();
}

class _PhotoViewerScreenState extends ConsumerState<PhotoViewerScreen> {
  double _splitPosition = 0.5; // 0.0 = full original, 1.0 = full processed

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.black,
      appBar: AppBar(
        backgroundColor: Colors.black,
        title: Text(
          'Photo ${widget.photoId}',
          style: const TextStyle(fontSize: 14),
        ),
        actions: [
          IconButton(
            icon: const Icon(Icons.share_outlined),
            onPressed: () {}, // P10
          ),
        ],
      ),
      body: Stack(
        fit: StackFit.expand,
        children: [
          // Before/after split view — full impl in P10
          GestureDetector(
            onHorizontalDragUpdate: (d) {
              setState(() {
                _splitPosition =
                    (_splitPosition +
                            d.delta.dx / MediaQuery.of(context).size.width)
                        .clamp(0.0, 1.0);
              });
            },
            child: Container(
              color: PhotonixColors.surface,
              child: Center(
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const Icon(
                      Icons.image_outlined,
                      color: PhotonixColors.textTertiary,
                      size: 48,
                    ),
                    const SizedBox(height: 12),
                    Text(
                      'Drag to compare before/after\nPhase 10',
                      textAlign: TextAlign.center,
                      style: const TextStyle(
                        color: PhotonixColors.textSecondary,
                        fontSize: 13,
                      ),
                    ),
                    const SizedBox(height: 8),
                    Text(
                      'Split: ${(_splitPosition * 100).toInt()}%',
                      style: const TextStyle(
                        color: PhotonixColors.textTertiary,
                        fontSize: 11,
                        fontFamily: 'monospace',
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),

          // Split line
          Positioned(
            left: MediaQuery.of(context).size.width * _splitPosition - 1,
            top: 0,
            bottom: 0,
            child: Container(width: 2, color: Colors.white.withOpacity(0.6)),
          ),
        ],
      ),
    );
  }
}
