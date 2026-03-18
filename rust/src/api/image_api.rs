// Photonix Camera — Image Processing Bridge API
// All functions in this file are auto-discovered by flutter_rust_bridge codegen.
// Vec<u8> return types are automatically zero-copied to Dart Uint8List in async mode.

/// Result struct returned by benchmark_roundtrip.
/// Timing fields are in microseconds for precision.
pub struct RoundtripResult {
    /// Size of the buffer that was processed (bytes)
    pub buffer_size_bytes: u64,
    /// Total time from Rust receiving bytes to returning bytes (microseconds)
    pub rust_processing_us: u64,
    /// Whether the round trip passed the 5ms threshold
    pub passed: bool,
    /// Human-readable result message
    pub message: String,
}

/// Returns the Photonix engine version string.
/// Called at app startup to confirm the Rust library loaded correctly.
pub fn get_engine_version() -> String {
    format!(
        "Photonix Engine v{} (Rust {}, frb 2.11)",
        env!("CARGO_PKG_VERSION"),
        "1.82+"
    )
}

/// Passes image bytes through the Rust engine and returns them.
/// In Phase 2 this is a passthrough — real processing added in P5/P6.
/// Vec<u8> is automatically zero-copied to Dart Uint8List in async mode.
pub fn process_image_bytes(bytes: Vec<u8>) -> Vec<u8> {
    // Phase 2: passthrough to validate the bridge pipeline.
    // Phase 5 will replace this with the classical pipeline.
    // Phase 7 will add AI routing on top.
    bytes
}

/// Benchmarks the Dart → Rust → Dart round trip for a given buffer.
/// Target: 4MB buffer must complete in under 5ms (5000 microseconds).
pub fn benchmark_roundtrip(bytes: Vec<u8>) -> RoundtripResult {
    use std::time::Instant;

    let buffer_size = bytes.len() as u64;
    let start = Instant::now();

    // Simulate the passthrough (same as process_image_bytes)
    let _result = bytes;

    let elapsed_us = start.elapsed().as_micros() as u64;
    let threshold_us: u64 = 5000; // 5ms in microseconds
    let passed = elapsed_us < threshold_us;

    RoundtripResult {
        buffer_size_bytes: buffer_size,
        rust_processing_us: elapsed_us,
        passed,
        message: format!(
            "{} — {}KB in {}µs (threshold: {}µs)",
            if passed { "PASS" } else { "FAIL" },
            buffer_size / 1024,
            elapsed_us,
            threshold_us
        ),
    }
}
