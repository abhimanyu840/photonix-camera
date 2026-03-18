import 'dart:typed_data';
import 'package:flutter/foundation.dart';
import '../rust/frb_generated.dart';
import '../rust/api/image_api.dart';

/// BridgeService wraps all Rust FFI calls with error handling and logging.
/// All methods are async — Rust runs on a background thread, Flutter UI never blocks.
class BridgeService {
  static final BridgeService _instance = BridgeService._internal();
  factory BridgeService() => _instance;
  BridgeService._internal();

  bool _initialized = false;

  /// Must be called once at app startup before any other bridge calls.
  /// Called automatically by main.dart via RustLib.init().
  Future<void> initialize() async {
    if (_initialized) return;
    await RustLib.init();
    _initialized = true;
    debugPrint('[Bridge] Initialized successfully');
  }

  /// Returns the Rust engine version string.
  /// Use to confirm the .so loaded and the bridge is live.
  Future<String> getEngineVersion() async {
    try {
      final version = await getEngineVersion();
      debugPrint('[Bridge] Engine version: $version');
      return version;
    } catch (e) {
      debugPrint('[Bridge] getEngineVersion failed: $e');
      return 'Error: $e';
    }
  }

  /// Passes image bytes through the Rust engine.
  /// Vec<u8> is zero-copied from Rust → Dart automatically in async mode.
  /// Returns the processed bytes as a Uint8List.
  Future<Uint8List> processImageBytes(Uint8List imageBytes) async {
    try {
      // Uint8List is passed directly — frb v2 handles the conversion
      final result = await processImageBytes(bytes: imageBytes);
      return result;
    } catch (e) {
      debugPrint('[Bridge] processImageBytes failed: $e');
      rethrow;
    }
  }

  /// Benchmarks the round trip for the given buffer size.
  /// Returns a RoundtripResult with timing details and pass/fail.
  Future<RoundtripResult> benchmarkRoundtrip(Uint8List buffer) async {
    try {
      final result = await benchmarkRoundtrip(bytes: buffer);
      debugPrint('[Bridge] Benchmark: ${result.message}');
      return result;
    } catch (e) {
      debugPrint('[Bridge] benchmarkRoundtrip failed: $e');
      rethrow;
    }
  }
}
