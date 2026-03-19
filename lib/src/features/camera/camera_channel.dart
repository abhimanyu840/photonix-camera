import 'dart:typed_data';
import 'package:flutter/services.dart';
import 'package:flutter/foundation.dart';

/// Dart client for the com.photonix/camera MethodChannel.
/// All camera hardware operations go through this class.
class CameraChannel {
  static const _channel = MethodChannel('com.photonix/camera');

  static final CameraChannel _instance = CameraChannel._internal();
  factory CameraChannel() => _instance;
  CameraChannel._internal();

  /// Initialise CameraX on the Android side.
  /// Must be called before any other method.
  Future<void> initCamera() async {
    try {
      await _channel.invokeMethod('initCamera');
      debugPrint('[CameraChannel] initCamera OK');
    } on PlatformException catch (e) {
      debugPrint('[CameraChannel] initCamera failed: ${e.message}');
      rethrow;
    }
  }

  /// Captures [frameCount] frames in rapid succession.
  /// Returns a list of JPEG byte arrays — one per frame.
  /// Frames have hardware NR and sharpening disabled.
  Future<List<Uint8List>> captureBurst(int frameCount) async {
    try {
      final result = await _channel.invokeMethod<List<dynamic>>(
        'captureBurst',
        {'frameCount': frameCount},
      );

      if (result == null) throw PlatformException(code: 'NULL_RESULT');

      // Convert List<dynamic> → List<Uint8List>
      return result
          .map((frame) => Uint8List.fromList(frame as List<int>))
          .toList();
    } on PlatformException catch (e) {
      debugPrint('[CameraChannel] captureBurst failed: ${e.message}');
      rethrow;
    }
  }

  /// Returns device camera capabilities (ISO range, exposure range, etc.)
  Future<CameraCapabilities> getCapabilities() async {
    try {
      final result = await _channel.invokeMethod<Map<dynamic, dynamic>>(
        'getCapabilities',
      );
      if (result == null) throw PlatformException(code: 'NULL_RESULT');
      return CameraCapabilities.fromMap(Map<String, dynamic>.from(result));
    } on PlatformException catch (e) {
      debugPrint('[CameraChannel] getCapabilities failed: ${e.message}');
      rethrow;
    }
  }

  /// Flips between front and back camera.
  Future<void> flipCamera() async {
    await _channel.invokeMethod('flipCamera');
  }

  /// Sets exposure compensation index.
  Future<void> setExposureCompensation(int index) async {
    await _channel.invokeMethod('setExposureCompensation', {'index': index});
  }

  /// Releases camera resources. Call when navigating away.
  Future<void> dispose() async {
    await _channel.invokeMethod('dispose');
  }
}

/// Camera hardware capabilities read from CameraCharacteristics.
class CameraCapabilities {
  final int minISO;
  final int maxISO;
  final int minExposureNs;
  final int maxExposureNs;
  final double focalLengthMm;
  final bool supportsRaw;
  final String cameraId;

  const CameraCapabilities({
    required this.minISO,
    required this.maxISO,
    required this.minExposureNs,
    required this.maxExposureNs,
    required this.focalLengthMm,
    required this.supportsRaw,
    required this.cameraId,
  });

  factory CameraCapabilities.fromMap(Map<String, dynamic> map) {
    return CameraCapabilities(
      minISO: map['minISO'] as int? ?? 0,
      maxISO: map['maxISO'] as int? ?? 3200,
      minExposureNs: (map['minExposureNs'] as int?) ?? 0,
      maxExposureNs: (map['maxExposureNs'] as int?) ?? 0,
      focalLengthMm: (map['focalLengthMm'] as num?)?.toDouble() ?? 4.0,
      supportsRaw: map['supportsRaw'] as bool? ?? false,
      cameraId: map['cameraId'] as String? ?? '0',
    );
  }

  /// Scene classifier hint based on focal length.
  /// <3mm = ultrawide, 3-6mm = main, >6mm = telephoto
  String get lensType {
    if (focalLengthMm < 3.0) return 'ultrawide';
    if (focalLengthMm > 6.0) return 'telephoto';
    return 'main';
  }
}
