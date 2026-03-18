import 'dart:typed_data';
import 'package:flutter/material.dart';
import '../../rust/api/image_api.dart';
import '../../rust/frb_generated.dart';

/// BridgeTestScreen — validates the Flutter ↔ Rust bridge.
///
/// Tests performed:
///   1. get_engine_version() — confirms .so loaded
///   2. process_image_bytes() — confirms byte passthrough works
///   3. benchmark_roundtrip() — 4MB buffer, must be < 5ms
class BridgeTestScreen extends StatefulWidget {
  const BridgeTestScreen({super.key});

  @override
  State<BridgeTestScreen> createState() => _BridgeTestScreenState();
}

class _BridgeTestScreenState extends State<BridgeTestScreen> {
  String _engineVersion = '—';
  String _passthroughResult = '—';
  String _benchmarkResult = '—';
  bool _benchmarkPassed = false;
  bool _isRunning = false;

  // 4MB buffer of zeros — simulates a raw image frame
  static const int _testBufferSize = 4 * 1024 * 1024; // 4MB

  Future<void> _runAllTests() async {
    setState(() {
      _isRunning = true;
      _engineVersion = 'Testing...';
      _passthroughResult = 'Testing...';
      _benchmarkResult = 'Testing...';
    });

    // ── Test 1: Engine version ──────────────────────────────────────────────
    try {
      final version = await getEngineVersion();
      setState(() => _engineVersion = version);
    } catch (e) {
      setState(() => _engineVersion = 'FAIL: $e');
    }

    // ── Test 2: Byte passthrough ────────────────────────────────────────────
    try {
      final testBytes = Uint8List(1024); // 1KB test
      // Fill with recognisable pattern
      for (int i = 0; i < testBytes.length; i++) {
        testBytes[i] = i % 256;
      }

      final sw = Stopwatch()..start();
      final result = await processImageBytes(bytes: testBytes);
      sw.stop();

      // Verify bytes survived the round trip intact
      bool intact = result.length == testBytes.length;
      if (intact) {
        for (int i = 0; i < result.length; i++) {
          if (result[i] != testBytes[i]) {
            intact = false;
            break;
          }
        }
      }

      setState(() {
        _passthroughResult = intact
            ? 'PASS — 1KB round trip in ${sw.elapsedMicroseconds}µs, bytes intact'
            : 'FAIL — bytes corrupted during transfer';
      });
    } catch (e) {
      setState(() => _passthroughResult = 'FAIL: $e');
    }

    // ── Test 3: 4MB benchmark ───────────────────────────────────────────────
    try {
      // Allocate 4MB buffer
      final bigBuffer = Uint8List(_testBufferSize);
      // Fill with pattern so we can verify on return
      for (int i = 0; i < bigBuffer.length; i++) {
        bigBuffer[i] = i % 256;
      }

      final result = await benchmarkRoundtrip(bytes: bigBuffer);

      setState(() {
        _benchmarkResult = result.message;
        _benchmarkPassed = result.passed;
      });
    } catch (e) {
      setState(() {
        _benchmarkResult = 'FAIL: $e';
        _benchmarkPassed = false;
      });
    }

    setState(() => _isRunning = false);
  }

  @override
  void initState() {
    super.initState();
    // Auto-run on screen open
    WidgetsBinding.instance.addPostFrameCallback((_) => _runAllTests());
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: const Color(0xFF0A0A0A),
      appBar: AppBar(
        backgroundColor: const Color(0xFF141414),
        title: const Text(
          'Bridge Validation — P2',
          style: TextStyle(color: Colors.white, fontSize: 16),
        ),
        iconTheme: const IconThemeData(color: Colors.white),
      ),
      body: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _TestCard(
              label: 'Test 1 — Engine version',
              description: 'Confirms .so loaded and RustLib.init() succeeded',
              result: _engineVersion,
              passed: _engineVersion.startsWith('Photonix'),
              running: _isRunning && _engineVersion == 'Testing...',
            ),
            const SizedBox(height: 12),
            _TestCard(
              label: 'Test 2 — Byte passthrough',
              description: '1KB buffer: Dart → Rust → Dart, bytes intact',
              result: _passthroughResult,
              passed: _passthroughResult.startsWith('PASS'),
              running: _isRunning && _passthroughResult == 'Testing...',
            ),
            const SizedBox(height: 12),
            _TestCard(
              label: 'Test 3 — 4MB round trip benchmark',
              description:
                  'Target: < 5000µs (5ms). Validates zero-copy transfer.',
              result: _benchmarkResult,
              passed: _benchmarkPassed,
              running: _isRunning && _benchmarkResult == 'Testing...',
            ),
            const Spacer(),
            SizedBox(
              width: double.infinity,
              child: ElevatedButton(
                onPressed: _isRunning ? null : _runAllTests,
                style: ElevatedButton.styleFrom(
                  backgroundColor: const Color(0xFF2563EB),
                  foregroundColor: Colors.white,
                  padding: const EdgeInsets.symmetric(vertical: 16),
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(8),
                  ),
                ),
                child: Text(
                  _isRunning ? 'Running...' : 'Run Tests Again',
                  style: const TextStyle(fontSize: 15),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _TestCard extends StatelessWidget {
  final String label;
  final String description;
  final String result;
  final bool passed;
  final bool running;

  const _TestCard({
    required this.label,
    required this.description,
    required this.result,
    required this.passed,
    required this.running,
  });

  @override
  Widget build(BuildContext context) {
    final Color statusColor = running
        ? Colors.amber
        : passed
        ? const Color(0xFF22C55E)
        : result == '—'
        ? Colors.grey
        : const Color(0xFFEF4444);

    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: const Color(0xFF141414),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: statusColor.withOpacity(0.4), width: 1),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Container(
                width: 8,
                height: 8,
                decoration: BoxDecoration(
                  color: statusColor,
                  shape: BoxShape.circle,
                ),
              ),
              const SizedBox(width: 8),
              Text(
                label,
                style: const TextStyle(
                  color: Colors.white,
                  fontSize: 13,
                  fontWeight: FontWeight.w500,
                ),
              ),
            ],
          ),
          const SizedBox(height: 4),
          Text(
            description,
            style: TextStyle(
              color: Colors.white.withOpacity(0.4),
              fontSize: 11,
            ),
          ),
          const SizedBox(height: 8),
          running
              ? const SizedBox(
                  height: 14,
                  width: 14,
                  child: CircularProgressIndicator(
                    strokeWidth: 2,
                    color: Colors.amber,
                  ),
                )
              : Text(
                  result,
                  style: TextStyle(
                    color: statusColor,
                    fontSize: 12,
                    fontFamily: 'monospace',
                  ),
                ),
        ],
      ),
    );
  }
}
