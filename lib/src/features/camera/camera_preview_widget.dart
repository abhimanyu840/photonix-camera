import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'camera_channel.dart';
import '../../shared/theme/app_theme.dart';

/// Displays the live camera preview using an AndroidView.
///
/// AndroidView embeds the native PreviewView into the Flutter widget tree.
/// This is the correct approach for CameraX — the alternative (Texture widget)
/// requires more setup and doesn't support all CameraX features.
///
/// Phase 4: Shows live viewfinder.
/// Phase 7: Preview is frozen during AI processing, replaced with result.
class CameraPreviewWidget extends ConsumerStatefulWidget {
  const CameraPreviewWidget({super.key});

  @override
  ConsumerState<CameraPreviewWidget> createState() =>
      _CameraPreviewWidgetState();
}

class _CameraPreviewWidgetState extends ConsumerState<CameraPreviewWidget> {
  bool _cameraReady = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _initCamera();
  }

  Future<void> _initCamera() async {
    try {
      await CameraChannel().initCamera();
      if (mounted) {
        setState(() => _cameraReady = true);
      }
    } catch (e) {
      if (mounted) {
        setState(() => _error = e.toString());
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_error != null) {
      return Container(
        color: Colors.black,
        child: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(
                Icons.error_outline,
                color: PhotonixColors.error,
                size: 32,
              ),
              const SizedBox(height: 8),
              Text(
                'Camera error:\n$_error',
                textAlign: TextAlign.center,
                style: const TextStyle(
                  color: PhotonixColors.error,
                  fontSize: 12,
                ),
              ),
            ],
          ),
        ),
      );
    }

    if (!_cameraReady) {
      return const ColoredBox(
        color: Colors.black,
        child: Center(
          child: CircularProgressIndicator(
            color: PhotonixColors.accent,
            strokeWidth: 2,
          ),
        ),
      );
    }

    // AndroidView embeds the native CameraX PreviewView
    return AndroidView(
      viewType: 'com.photonix/preview',
      layoutDirection: TextDirection.ltr,
      creationParams: const <String, dynamic>{},
      creationParamsCodec: const StandardMessageCodec(),
    );
  }

  @override
  void dispose() {
    CameraChannel().dispose();
    super.dispose();
  }
}
