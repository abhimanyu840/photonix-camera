//! Flutter ↔ Rust bridge API surface.
//! All public functions here are auto-exposed to Dart by frb codegen.

use crate::frb_generated::StreamSink;
use crate::pipeline::orchestrator::{run_classical, ClassicalPipelineConfig, PipelineProgress};

/// Returned by benchmark_roundtrip.
pub struct RoundtripResult {
    pub buffer_size_bytes: u64,
    pub rust_processing_us: u64,
    pub passed: bool,
    pub message: String,
}

/// Configuration DTO passed from Dart.
/// Maps 1:1 to ClassicalPipelineConfig.
pub struct PipelineConfigDto {
    pub run_burst_stack: bool,
    pub run_hdr_merge: bool,
    pub run_exposure_lift: bool,
    pub exposure_lift_amount: f32,
    pub saturation: f32,
    pub tone_mapping: String, // "aces" | "reinhard" | "none"
    pub sharpen_amount: f32,
    pub jpeg_quality: u8,
}

impl Default for PipelineConfigDto {
    fn default() -> Self {
        Self {
            run_burst_stack: true,
            run_hdr_merge: false,
            run_exposure_lift: true,
            exposure_lift_amount: 0.1,
            saturation: 1.1,
            tone_mapping: "aces".to_string(),
            sharpen_amount: 0.4,
            jpeg_quality: 95,
        }
    }
}

/// Engine version string — confirms .so loaded correctly.
pub fn get_engine_version() -> String {
    format!(
        "Photonix Engine v{} (Rust 1.82+, frb 2.11)",
        env!("CARGO_PKG_VERSION")
    )
}

/// Process a single JPEG frame through the classical pipeline.
/// Returns processed JPEG bytes.
pub fn process_single(frame: Vec<u8>, config: PipelineConfigDto) -> Vec<u8> {
    let cfg = dto_to_config(config);
    run_classical(vec![frame], cfg, None).unwrap_or_else(|e| {
        log::error!("process_single failed: {e}");
        vec![]
    })
}

/// Process a burst of JPEG frames through the classical pipeline.
/// Returns processed JPEG bytes.
pub fn process_burst(frames: Vec<Vec<u8>>, config: PipelineConfigDto) -> Vec<u8> {
    let cfg = dto_to_config(config);
    run_classical(frames, cfg, None).unwrap_or_else(|e| {
        log::error!("process_burst failed: {e}");
        vec![]
    })
}

/// Process a burst with live progress updates streamed to Dart.
/// Returns Stream<PipelineProgress> in Dart.
pub fn process_burst_with_progress(
    frames: Vec<Vec<u8>>,
    config: PipelineConfigDto,
    sink: StreamSink<PipelineProgress>,
) {
    let cfg = dto_to_config(config);
    let _ = run_classical(frames, cfg, Some(sink));
}

/// P2 passthrough — zero-copy validation.
pub fn process_image_bytes(bytes: Vec<u8>) -> Vec<u8> {
    bytes
}

/// P2 benchmark — round-trip timing.
pub fn benchmark_roundtrip(bytes: Vec<u8>) -> RoundtripResult {
    use std::time::Instant;
    let size = bytes.len() as u64;
    let start = Instant::now();
    let _ = bytes;
    let elapsed = start.elapsed().as_micros() as u64;
    RoundtripResult {
        buffer_size_bytes: size,
        rust_processing_us: elapsed,
        passed: elapsed < 5000,
        message: format!(
            "{} — {}KB in {}µs",
            if elapsed < 5000 { "PASS" } else { "FAIL" },
            size / 1024,
            elapsed
        ),
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn dto_to_config(dto: PipelineConfigDto) -> ClassicalPipelineConfig {
    use crate::compute::color::WhiteBalanceMode;
    use crate::compute::tone_map::ToneMappingMode;

    ClassicalPipelineConfig {
        run_burst_stack: dto.run_burst_stack,
        run_hdr_merge: dto.run_hdr_merge,
        run_exposure_lift: dto.run_exposure_lift,
        exposure_lift_amount: dto.exposure_lift_amount,
        white_balance_mode: WhiteBalanceMode::GreyWorld,
        saturation: dto.saturation,
        tone_mapping: match dto.tone_mapping.as_str() {
            "reinhard" => ToneMappingMode::Reinhard,
            "none" => ToneMappingMode::None,
            _ => ToneMappingMode::AcesFilmic,
        },
        sharpen_amount: dto.sharpen_amount,
        jpeg_quality: dto.jpeg_quality,
    }
}
