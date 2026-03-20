import 'dart:typed_data';
import 'package:camera/camera.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/painting.dart'; // for Offset

enum CameraMode { ultraWide, photo, portrait, macro, pro, video }

class CameraChannel {
  static final CameraChannel _instance = CameraChannel._internal();
  factory CameraChannel() => _instance;
  CameraChannel._internal();

  CameraController? _controller;
  List<CameraDescription>? _cameras;
  bool _isInitialized = false;
  bool _isFront = false;

  double _minZoom = 1.0;
  double _maxZoom = 1.0;
  double _currentZoom = 1.0;
  double _minExposure = -4.0;
  double _maxExposure = 4.0;
  double _currentExposure = 0.0;
  FlashMode _flashMode = FlashMode.auto;
  CameraMode _cameraMode = CameraMode.photo;

  bool get isInitialized => _isInitialized;
  bool get isFront => _isFront;
  CameraController? get controller => _controller;
  double get minZoom => _minZoom;
  double get maxZoom => _maxZoom;
  double get currentZoom => _currentZoom;
  double get minExposure => _minExposure;
  double get maxExposure => _maxExposure;
  double get currentExposure => _currentExposure;
  FlashMode get flashMode => _flashMode;
  CameraMode get cameraMode => _cameraMode;

  Future<void> initCamera({bool front = false}) async {
    _cameras ??= await availableCameras();
    if (_cameras == null || _cameras!.isEmpty) {
      throw Exception('No cameras available');
    }

    final direction = front
        ? CameraLensDirection.front
        : CameraLensDirection.back;
    final camera = _cameras!.firstWhere(
      (c) => c.lensDirection == direction,
      orElse: () => _cameras!.first,
    );

    if (_controller == null || !_isInitialized) {
      _controller = CameraController(
        camera,
        ResolutionPreset.max,
        enableAudio: false,
        imageFormatGroup: ImageFormatGroup.jpeg,
      );
      await _controller!.initialize();
    } else {
      // Fast switch — no dispose needed
      await _controller!.setDescription(camera);
    }

    _isFront = camera.lensDirection == CameraLensDirection.front;
    await _loadCapabilities();
    await _applyDefaults();
    _isInitialized = true;
    debugPrint(
      '[CameraChannel] Ready: ${camera.name} '
      '(${_isFront ? "front" : "back"}, '
      'zoom: ${_minZoom.toStringAsFixed(1)}-${_maxZoom.toStringAsFixed(1)}x)',
    );
  }

  Future<void> _loadCapabilities() async {
    try {
      _minZoom = await _controller!.getMinZoomLevel();
      _maxZoom = await _controller!.getMaxZoomLevel();
      _minExposure = await _controller!.getMinExposureOffset();
      _maxExposure = await _controller!.getMaxExposureOffset();
    } catch (e) {
      debugPrint('[CameraChannel] Capabilities: $e');
    }
  }

  Future<void> _applyDefaults() async {
    try {
      _currentZoom = _minZoom;
      await _controller!.setZoomLevel(_currentZoom);
      await _controller!.setFlashMode(_flashMode);
      await _controller!.setFocusMode(FocusMode.auto);
      await _controller!.setExposureMode(ExposureMode.auto);
      _currentExposure = 0.0;
    } catch (e) {
      debugPrint('[CameraChannel] Defaults: $e');
    }
  }

  Future<void> switchCamera() async => initCamera(front: !_isFront);

  Future<void> setFocusPoint(Offset normalizedPoint) async {
    if (_controller == null || !_isInitialized) return;
    try {
      final point = _isFront
          ? Offset(1.0 - normalizedPoint.dx, normalizedPoint.dy)
          : normalizedPoint;
      await _controller!.setExposurePoint(point);
      await _controller!.setFocusPoint(point);
      await _controller!.setFocusMode(FocusMode.locked);
      Future.delayed(const Duration(seconds: 3), () async {
        try {
          await _controller?.setFocusMode(FocusMode.auto);
          await _controller?.setExposureMode(ExposureMode.auto);
        } catch (_) {}
      });
    } catch (e) {
      debugPrint('[CameraChannel] Focus: $e');
    }
  }

  Future<void> resetFocus() async {
    try {
      await _controller?.setFocusPoint(null);
      await _controller?.setExposurePoint(null);
      await _controller?.setFocusMode(FocusMode.auto);
      await _controller?.setExposureMode(ExposureMode.auto);
    } catch (_) {}
  }

  Future<void> setZoom(double zoom) async {
    if (_controller == null || !_isInitialized) return;
    _currentZoom = zoom.clamp(_minZoom, _maxZoom);
    try {
      await _controller!.setZoomLevel(_currentZoom);
    } catch (e) {
      debugPrint('[CameraChannel] Zoom: $e');
    }
  }

  Future<void> setExposure(double ev) async {
    if (_controller == null || !_isInitialized) return;
    _currentExposure = ev.clamp(_minExposure, _maxExposure);
    try {
      await _controller!.setExposureOffset(_currentExposure);
    } catch (e) {
      debugPrint('[CameraChannel] Exposure: $e');
    }
  }

  Future<void> cycleFlash() async {
    _flashMode = switch (_flashMode) {
      FlashMode.auto => FlashMode.always,
      FlashMode.always => FlashMode.off,
      FlashMode.off => FlashMode.torch,
      FlashMode.torch => FlashMode.auto,
    };
    try {
      await _controller?.setFlashMode(_flashMode);
    } catch (_) {}
  }

  void setCameraMode(CameraMode mode) {
    _cameraMode = mode;
    switch (mode) {
      case CameraMode.portrait:
        // Stock portrait behaviour: main lens at slight wide zoom
        // Gives more scene context than telephoto, AI bokeh handles depth
        setZoom(_minZoom); // 1× main lens — same as stock portrait
      case CameraMode.ultraWide:
        // Use minimum available zoom — if minZoom = 1.0, this is same as photo
        // but we apply a wider crop in the Rust pipeline
        setZoom(_minZoom);
      case CameraMode.macro:
        setZoom(_minZoom);
      case CameraMode.photo:
      case CameraMode.video:
        setZoom(_minZoom);
        resetFocus();
      case CameraMode.pro:
        break;
    }
  }

  Future<List<Uint8List>> captureBurst(int frameCount) async {
    if (_controller == null || !_isInitialized) {
      throw Exception('Camera not initialized');
    }
    final frames = <Uint8List>[];
    for (int i = 0; i < frameCount; i++) {
      debugPrint('[CameraChannel] Frame ${i + 1}/$frameCount...');
      final xFile = await _controller!.takePicture().timeout(
        const Duration(seconds: 8),
      );
      final bytes = await xFile.readAsBytes();
      frames.add(bytes);
      debugPrint('[CameraChannel] Frame ${i + 1}: ${bytes.length} bytes ✓');
    }
    return frames;
  }

  Future<void> dispose() async {
    _isInitialized = false;
    try {
      await _controller?.dispose();
    } catch (_) {}
    _controller = null;
  }
}
