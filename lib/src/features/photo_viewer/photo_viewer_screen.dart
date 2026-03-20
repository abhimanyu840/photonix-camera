import 'dart:typed_data';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../providers/gallery_provider.dart';
import '../../shared/theme/app_theme.dart';

/// Before/after comparison viewer with drag split line.
/// Displays processed photo on the left of the split, original on the right.
class PhotoViewerScreen extends ConsumerStatefulWidget {
  final String photoId;
  const PhotoViewerScreen({super.key, required this.photoId});

  @override
  ConsumerState<PhotoViewerScreen> createState() => _PhotoViewerScreenState();
}

class _PhotoViewerScreenState extends ConsumerState<PhotoViewerScreen>
    with SingleTickerProviderStateMixin {
  double _splitPosition = 0.5;
  late AnimationController _snapController;
  late Animation<double> _snapAnimation;
  double _lastSplitPosition = 0.5;

  @override
  void initState() {
    super.initState();
    _snapController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 250),
    );
    _snapAnimation =
        Tween<double>(begin: 0.5, end: 0.5).animate(
          CurvedAnimation(parent: _snapController, curve: Curves.easeOut),
        )..addListener(() {
          setState(() => _splitPosition = _snapAnimation.value);
        });
  }

  @override
  void dispose() {
    _snapController.dispose();
    super.dispose();
  }

  void _onDragUpdate(DragUpdateDetails d, double screenWidth) {
    setState(() {
      _splitPosition = (_splitPosition + d.delta.dx / screenWidth).clamp(
        0.02,
        0.98,
      );
    });
  }

  void _onDragEnd(DragEndDetails _) {
    // Snap to center if within 10% of middle
    if ((_splitPosition - 0.5).abs() < 0.1) {
      _snapAnimation = Tween<double>(begin: _splitPosition, end: 0.5).animate(
        CurvedAnimation(parent: _snapController, curve: Curves.easeOut),
      );
      _snapController.forward(from: 0.0);
    }
  }

  @override
  Widget build(BuildContext context) {
    final screenWidth = MediaQuery.of(context).size.width;

    return Scaffold(
      backgroundColor: Colors.black,
      appBar: AppBar(
        backgroundColor: Colors.black,
        foregroundColor: Colors.white,
        title: Text(
          'Photo ${widget.photoId}',
          style: const TextStyle(fontSize: 14),
        ),
        actions: [
          Semantics(
            label: 'Share photo',
            button: true,
            child: IconButton(
              icon: const Icon(Icons.share_outlined),
              onPressed: () {}, // P11
            ),
          ),
          Semantics(
            label: 'Delete photo',
            button: true,
            child: IconButton(
              icon: const Icon(Icons.delete_outline),
              onPressed: () {}, // P11
            ),
          ),
        ],
      ),
      body: Stack(
        fit: StackFit.expand,
        children: [
          // ── Photo display ────────────────────────────────────────────────
          _PhotoDisplay(splitPosition: _splitPosition, photoId: widget.photoId),

          // ── Drag handle ──────────────────────────────────────────────────
          Positioned(
            left: screenWidth * _splitPosition - 20,
            top: 0,
            bottom: 0,
            width: 40,
            child: GestureDetector(
              behavior: HitTestBehavior.translucent,
              onHorizontalDragUpdate: (d) => _onDragUpdate(d, screenWidth),
              onHorizontalDragEnd: _onDragEnd,
              child: Center(
                child: Semantics(
                  label: 'Drag to compare before and after',
                  child: Container(
                    width: 3,
                    color: Colors.white,
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        // Drag handle knob
                        Container(
                          width: 32,
                          height: 32,
                          decoration: BoxDecoration(
                            color: Colors.white,
                            shape: BoxShape.circle,
                            boxShadow: [
                              BoxShadow(
                                color: Colors.black.withOpacity(0.3),
                                blurRadius: 8,
                              ),
                            ],
                          ),
                          child: const Icon(
                            Icons.compare_arrows,
                            size: 18,
                            color: Colors.black87,
                          ),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),

          // ── Labels ───────────────────────────────────────────────────────
          Positioned(
            top: 12,
            left: 12,
            child: _SplitLabel(
              text: 'ENHANCED',
              visible: _splitPosition > 0.15,
            ),
          ),
          Positioned(
            top: 12,
            right: 12,
            child: _SplitLabel(
              text: 'ORIGINAL',
              visible: _splitPosition < 0.85,
            ),
          ),
        ],
      ),
    );
  }
}

class _PhotoDisplay extends StatelessWidget {
  final double splitPosition;
  final String photoId;
  const _PhotoDisplay({required this.splitPosition, required this.photoId});

  @override
  Widget build(BuildContext context) {
    return CustomPaint(
      painter: _SplitPainter(splitPosition: splitPosition),
      child: Container(color: PhotonixColors.surface),
    );
  }
}

class _SplitPainter extends CustomPainter {
  final double splitPosition;
  const _SplitPainter({required this.splitPosition});

  @override
  void paint(Canvas canvas, Size size) {
    final splitX = size.width * splitPosition;

    // Left: enhanced (teal tint placeholder)
    canvas.drawRect(
      Rect.fromLTWH(0, 0, splitX, size.height),
      Paint()..color = const Color(0xFF0F2B2B),
    );
    // Right: original (gray placeholder)
    canvas.drawRect(
      Rect.fromLTWH(splitX, 0, size.width - splitX, size.height),
      Paint()..color = const Color(0xFF1A1A1A),
    );

    // Split line
    canvas.drawLine(
      Offset(splitX, 0),
      Offset(splitX, size.height),
      Paint()
        ..color = Colors.white
        ..strokeWidth = 1.5,
    );
  }

  @override
  bool shouldRepaint(_SplitPainter old) => old.splitPosition != splitPosition;
}

class _SplitLabel extends StatelessWidget {
  final String text;
  final bool visible;
  const _SplitLabel({required this.text, required this.visible});

  @override
  Widget build(BuildContext context) {
    return AnimatedOpacity(
      opacity: visible ? 1.0 : 0.0,
      duration: const Duration(milliseconds: 150),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
        decoration: BoxDecoration(
          color: Colors.black54,
          borderRadius: BorderRadius.circular(4),
        ),
        child: Text(
          text,
          style: const TextStyle(
            color: Colors.white,
            fontSize: 9,
            fontWeight: FontWeight.w600,
            letterSpacing: 0.8,
          ),
        ),
      ),
    );
  }
}
