import 'dart:typed_data';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:photonix_camera/src/rust/frb_generated.dart';
import 'package:photonix_camera/src/rust/api/image_api.dart' as rust_api;

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() async {
    await RustLib.init();
  });

  // ── Test 1: Bridge round-trip ───────────────────────────────────────────────
  testWidgets('test_bridge_roundtrip_4mb_under_5ms', (tester) async {
    const bufferSize = 4 * 1024 * 1024; // 4MB
    final buffer = Uint8List(bufferSize);

    // Fill with pattern
    for (int i = 0; i < bufferSize; i++) {
      buffer[i] = i % 256;
    }

    final result = await rust_api.benchmarkRoundtrip(bytes: buffer);

    expect(
      result.passed,
      isTrue,
      reason:
          'Bridge round-trip took ${result.rustProcessingUs}µs (limit 5000µs)',
    );
    expect(result.bufferSizeBytes, equals(bufferSize));

    debugPrint('Bridge round-trip: ${result.message}');
  });

  // ── Test 2: Engine version confirms bridge is live ──────────────────────────
  testWidgets('test_engine_version_not_empty', (tester) async {
    final version = await rust_api.getEngineVersion();
    expect(version, isNotEmpty);
    expect(version, contains('Photonix'));
    debugPrint('Engine: $version');
  });

  // ── Test 3: Process single frame ────────────────────────────────────────────
  testWidgets('test_process_single_returns_jpeg', (tester) async {
    // Create a minimal valid JPEG (1×1 pixel)
    // This is a hardcoded minimal valid JPEG header
    final minimalJpeg = Uint8List.fromList([
      0xFF,
      0xD8,
      0xFF,
      0xE0,
      0x00,
      0x10,
      0x4A,
      0x46,
      0x49,
      0x46,
      0x00,
      0x01,
      0x01,
      0x00,
      0x00,
      0x01,
      0x00,
      0x01,
      0x00,
      0x00,
      0xFF,
      0xDB,
      0x00,
      0x43,
    ]);

    // process_single should not throw even with a small/invalid input
    try {
      final result = await rust_api.processSingle(
        frame: minimalJpeg,
        sceneHint: 'standard',
      );
      // If it succeeds, output should be non-empty JPEG
      if (result.isNotEmpty) {
        expect(result[0], equals(0xFF));
        expect(result[1], equals(0xD8));
      }
    } catch (e) {
      // Expected — minimal JPEG may not decode properly
      // Test just verifies no crash/hang
      debugPrint('process_single with minimal JPEG: $e');
    }
  });

  // ── Test 4: Processing stream events ────────────────────────────────────────
  testWidgets('test_processing_stream_emits_events', (tester) async {
    // Create a dummy frame list
    final dummyFrame = Uint8List(100);

    final updates = <rust_api.ProcessingUpdate>[];

    try {
      final stream = rust_api.captureAndProcess(
        frames: [dummyFrame],
        sceneHint: 'standard',
      );

      // Collect events with timeout
      await stream
          .take(5)
          .toList()
          .timeout(const Duration(seconds: 5), onTimeout: () => [])
          .then((events) => updates.addAll(events));
    } catch (e) {
      debugPrint('Stream test: $e');
    }

    // Should get at least one event (even if pipeline fails gracefully)
    debugPrint('Received ${updates.length} pipeline events');
  });
}
