import 'package:camera/camera.dart';
import 'package:flutter/material.dart';
import '../../shared/theme/app_theme.dart';
import 'camera_channel.dart';

class CameraPreviewWidget extends StatefulWidget {
  const CameraPreviewWidget({super.key});

  @override
  State<CameraPreviewWidget> createState() => _CameraPreviewWidgetState();
}

class _CameraPreviewWidgetState extends State<CameraPreviewWidget> {
  bool _ready = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _init();
  }

  Future<void> _init() async {
    try {
      if (!CameraChannel().isInitialized) {
        await CameraChannel().initCamera();
      }
      if (mounted) setState(() => _ready = true);
    } catch (e) {
      if (mounted) setState(() => _error = e.toString());
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_error != null) {
      return ColoredBox(
        color: Colors.black,
        child: Center(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Text(
              'Camera error: $_error',
              style: const TextStyle(color: PhotonixColors.error, fontSize: 12),
              textAlign: TextAlign.center,
            ),
          ),
        ),
      );
    }

    final controller = CameraChannel().controller;
    if (!_ready || controller == null || !controller.value.isInitialized) {
      return const ColoredBox(color: Colors.black);
    }

    // Correct aspect ratio — avoid stretch and over-zoom
    return SizedBox.expand(
      child: FittedBox(
        fit: BoxFit.cover,
        child: SizedBox(
          width: controller.value.previewSize?.height ?? 1920,
          height: controller.value.previewSize?.width ?? 1080,
          child: CameraPreview(controller),
        ),
      ),
    );
  }

  @override
  void dispose() {
    // Don't dispose CameraChannel here — it's a singleton
    // Disposal is handled by CameraScreen
    super.dispose();
  }
}
