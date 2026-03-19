//! Classical pipeline orchestrator.
//!
//! Wires all compute stages in correct order.
//! AI slots are stubbed — replaced in Phase 7.
//!
//! Stage order (linear space unless noted):
//!   decode JPEG → sRGB decode → white balance → [AI denoise stub] →
//!   burst stack → [HDR merge if bracketed] → exposure → [AI enhance stub] →
//!   tone map → gamma encode → [AI super-res stub] → sharpen → encode JPEG

use anyhow::Result;
use image::{DynamicImage, ImageReader, RgbImage};
use std::io::Cursor;

use crate::compute::burst_stack::{stack_burst, Frame};
use crate::compute::color::{adjust_saturation, white_balance, WhiteBalanceMode};
use crate::compute::exposure::gamma_lift;
use crate::compute::hdr_merge::mertens_fusion;
use crate::compute::sharpen::unsharp_mask;
use crate::compute::tone_map::{srgb_to_linear, tone_map, ToneMappingMode};
use crate::frb_generated::StreamSink;

/// Pipeline stage names — sent to Dart UI for progress overlay.
pub const STAGE_DECODE: &str = "Decoding frames...";
pub const STAGE_WB: &str = "White balance...";
pub const STAGE_STACK: &str = "Stacking frames...";
pub const STAGE_HDR: &str = "HDR merge...";
pub const STAGE_EXPOSURE: &str = "Exposure correction...";
pub const STAGE_TONEMAP: &str = "Tone mapping...";
pub const STAGE_SHARPEN: &str = "Sharpening...";
pub const STAGE_ENCODE: &str = "Encoding...";

/// Progress event sent through the StreamSink to Dart.
pub struct PipelineProgress {
    pub stage: String,
    pub progress: f32, // 0.0 – 1.0
}

/// Configuration for the classical pipeline.
/// Boolean flags determine which stages run.
/// All fields have sensible defaults via Default impl.
pub struct ClassicalPipelineConfig {
    /// Run multi-frame burst stack (requires >1 frame)
    pub run_burst_stack: bool,
    /// Run Mertens HDR fusion (requires bracketed exposures)
    pub run_hdr_merge: bool,
    /// Run gamma lift for shadow recovery
    pub run_exposure_lift: bool,
    pub exposure_lift_amount: f32,
    /// White balance mode
    pub white_balance_mode: WhiteBalanceMode,
    /// Saturation boost (1.0 = unchanged)
    pub saturation: f32,
    /// Tone mapping mode
    pub tone_mapping: ToneMappingMode,
    /// Unsharp mask amount (0.0 = off)
    pub sharpen_amount: f32,
    /// JPEG output quality (0–100)
    pub jpeg_quality: u8,
}

impl Default for ClassicalPipelineConfig {
    fn default() -> Self {
        Self {
            run_burst_stack: true,
            run_hdr_merge: false,
            run_exposure_lift: true,
            exposure_lift_amount: 0.1,
            white_balance_mode: WhiteBalanceMode::GreyWorld,
            saturation: 1.1, // slight boost
            tone_mapping: ToneMappingMode::AcesFilmic,
            sharpen_amount: 0.4,
            jpeg_quality: 95,
        }
    }
}

/// Decode a JPEG byte slice into a linear f32 Frame.
pub fn decode_jpeg(jpeg_bytes: &[u8]) -> Result<Frame> {
    let reader = ImageReader::new(Cursor::new(jpeg_bytes)).with_guessed_format()?;
    let img: DynamicImage = reader.decode()?;
    let rgb: RgbImage = img.into_rgb8();
    let (w, h) = rgb.dimensions();

    // Convert u8 sRGB → linear f32
    let pixels: Vec<f32> = rgb
        .as_raw()
        .iter()
        .map(|&b| srgb_to_linear(b as f32 / 255.0))
        .collect();

    Ok(Frame::new(pixels, w, h))
}

/// Encode a linear f32 Frame to JPEG bytes.
/// Applies sRGB gamma encoding internally.
pub fn encode_jpeg(frame: &Frame, quality: u8) -> Result<Vec<u8>> {
    use crate::compute::tone_map::linear_to_srgb;
    use image::codecs::jpeg::JpegEncoder;

    let w = frame.width;
    let h = frame.height;

    let u8_pixels: Vec<u8> = frame
        .pixels
        .iter()
        .map(|&x| (linear_to_srgb(x.clamp(0.0, 1.0)) * 255.0) as u8)
        .collect();

    let rgb = RgbImage::from_raw(w, h, u8_pixels)
        .ok_or_else(|| anyhow::anyhow!("Failed to create RgbImage"))?;

    let mut buf = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut buf, quality);
    encoder.encode_image(&rgb)?;

    Ok(buf)
}

/// Run the classical pipeline on one or more JPEG frames.
///
/// `progress_sink`: optional StreamSink for live stage updates to Dart.
/// Returns the processed image as JPEG bytes.
pub fn run_classical(
    jpeg_frames: Vec<Vec<u8>>,
    config: ClassicalPipelineConfig,
    progress_sink: Option<StreamSink<PipelineProgress>>,
) -> Result<Vec<u8>> {
    macro_rules! report {
        ($stage:expr, $pct:expr) => {
            if let Some(ref sink) = progress_sink {
                let _ = sink.add(PipelineProgress {
                    stage: $stage.to_string(),
                    progress: $pct,
                });
            }
        };
    }

    // ── 1. Decode all frames ─────────────────────────────────────────────────
    report!(STAGE_DECODE, 0.05);
    let frames: Vec<Frame> = jpeg_frames
        .iter()
        .map(|bytes| decode_jpeg(bytes))
        .collect::<Result<Vec<_>>>()?;

    // ── 2. White balance ──────────────────────────────────────────────────────
    report!(STAGE_WB, 0.15);
    let frames: Vec<Frame> = frames
        .into_iter()
        .map(|f| white_balance(f, WhiteBalanceMode::GreyWorld))
        .collect();

    // ── 3. Burst stack ────────────────────────────────────────────────────────
    let frame = if config.run_burst_stack && frames.len() > 1 {
        report!(STAGE_STACK, 0.30);
        stack_burst(frames)?
    } else if config.run_hdr_merge && frames.len() > 1 {
        report!(STAGE_HDR, 0.30);
        mertens_fusion(&frames)
    } else {
        frames.into_iter().next().unwrap()
    };

    // ── 4. Saturation ─────────────────────────────────────────────────────────
    let frame = adjust_saturation(frame, config.saturation);

    // ── 5. Exposure lift ──────────────────────────────────────────────────────
    report!(STAGE_EXPOSURE, 0.50);
    let frame = if config.run_exposure_lift {
        gamma_lift(frame, config.exposure_lift_amount)
    } else {
        frame
    };

    // ── 6. [AI denoiser stub — Phase 6/7] ────────────────────────────────────
    // let frame = ai_denoise(frame); // replaced in P7

    // ── 7. Tone mapping ───────────────────────────────────────────────────────
    report!(STAGE_TONEMAP, 0.70);
    let frame = tone_map(frame, config.tone_mapping);

    // ── 8. Sharpen ────────────────────────────────────────────────────────────
    report!(STAGE_SHARPEN, 0.85);
    let frame = if config.sharpen_amount > 0.0 {
        unsharp_mask(frame, config.sharpen_amount)
    } else {
        frame
    };

    // ── 9. Encode JPEG ────────────────────────────────────────────────────────
    report!(STAGE_ENCODE, 0.95);
    let jpeg = encode_jpeg(&frame, config.jpeg_quality)?;

    report!("Done", 1.0);
    Ok(jpeg)
}
