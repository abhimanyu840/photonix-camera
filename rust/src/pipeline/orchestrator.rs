//! Full pipeline orchestrator — classical + AI stages.
//!
//! Stage order:
//!   decode → scene classify → [classical: WB, stack, HDR, exposure] →
//!   [AI: denoise, enhance] → tone map → sharpen →
//!   [AI: super-res] → [AI: depth + bokeh] → encode
//!
//! Progress events are sent via StreamSink so the Flutter UI can show
//! live stage labels during the ~300ms processing window.

use anyhow::Result;
use std::sync::Arc;

use crate::ai::models::denoiser::run_denoiser;
use crate::ai::models::depth::run_depth;
use crate::ai::models::enhancer::run_enhancer;
use crate::ai::models::scene_cls::classify_scene;
use crate::ai::models::super_res::run_super_res;
use crate::compute::burst_stack::{stack_burst, Frame};
use crate::compute::color::{adjust_saturation, white_balance, WhiteBalanceMode};
use crate::compute::exposure::gamma_lift;
use crate::compute::hdr_merge::mertens_fusion;
use crate::compute::sharpen::unsharp_mask;
use crate::compute::tone_map::{srgb_to_linear, tone_map, ToneMappingMode};
use crate::pipeline::bokeh::apply_bokeh;
use crate::pipeline::scene::{PipelineConfig, Scene};

/// Progress event sent to Dart during processing.
pub struct PipelineProgress {
    pub stage: String,
    pub progress: f32,
}

/// Decode JPEG bytes to a linear f32 Frame.
pub fn decode_jpeg_to_frame(jpeg: &[u8]) -> Result<Frame> {
    use image::{DynamicImage, ImageReader, RgbImage};
    use std::io::Cursor;

    let reader = ImageReader::new(Cursor::new(jpeg)).with_guessed_format()?;
    let img: DynamicImage = reader.decode()?;
    let rgb: RgbImage = img.into_rgb8();
    let (w, h) = rgb.dimensions();

    let pixels: Vec<f32> = rgb
        .as_raw()
        .iter()
        .map(|&b| srgb_to_linear(b as f32 / 255.0))
        .collect();

    Ok(Frame::new(pixels, w, h))
}

/// Encode a linear f32 Frame to JPEG bytes.
pub fn encode_frame_to_jpeg(frame: &Frame, quality: u8) -> Result<Vec<u8>> {
    use crate::compute::tone_map::linear_to_srgb;
    use image::codecs::jpeg::JpegEncoder;
    use image::RgbImage;

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

/// Run the full pipeline on JPEG frames.
///
/// `progress_fn`: called with (stage_name, 0.0-1.0) at each stage.
///   Use this to update the StreamSink from image_api.rs.
pub fn run_full_pipeline(
    jpeg_frames: Vec<Vec<u8>>,
    config: &PipelineConfig,
    scene: Scene,
    mut progress_fn: impl FnMut(&str, f32),
) -> Result<Vec<u8>> {
    // ── 1. Decode all frames ─────────────────────────────────────────────────
    progress_fn("Decoding...", 0.05);
    let frames: Vec<Frame> = jpeg_frames
        .iter()
        .map(|b| decode_jpeg_to_frame(b))
        .collect::<Result<Vec<_>>>()?;

    let w = frames[0].width as usize;
    let h = frames[0].height as usize;

    // ── 2. White balance ──────────────────────────────────────────────────────
    progress_fn("White balance...", 0.10);
    let frames: Vec<Frame> = frames
        .into_iter()
        .map(|f| white_balance(f, WhiteBalanceMode::GreyWorld))
        .collect();

    // ── 3. Burst stack or HDR ─────────────────────────────────────────────────
    let frame = if config.run_hdr && frames.len() > 1 {
        progress_fn("HDR merge...", 0.20);
        mertens_fusion(&frames)
    } else if frames.len() > 1 {
        progress_fn("Stacking frames...", 0.20);
        stack_burst(frames)?
    } else {
        frames.into_iter().next().unwrap()
    };

    // ── 4. AI denoiser ────────────────────────────────────────────────────────
    let frame = if config.run_denoiser {
        progress_fn("Denoising...", 0.30);
        match run_denoiser(&frame.pixels, h, w) {
            Ok(pixels) => Frame::new(pixels, frame.width, frame.height),
            Err(e) => {
                log::warn!("Denoiser failed: {e} — skipping");
                frame
            }
        }
    } else {
        frame
    };

    // ── 5. AI low-light enhancer ──────────────────────────────────────────────
    let frame = if config.run_enhancer {
        progress_fn("Enhancing...", 0.40);
        match run_enhancer(&frame.pixels, h, w) {
            Ok(pixels) => Frame::new(pixels, frame.width, frame.height),
            Err(e) => {
                log::warn!("Enhancer failed: {e} — skipping");
                frame
            }
        }
    } else {
        frame
    };

    // ── 6. Saturation + exposure ──────────────────────────────────────────────
    let frame = adjust_saturation(frame, 1.1);
    let frame = gamma_lift(frame, 0.1);

    // ── 7. Tone mapping ───────────────────────────────────────────────────────
    progress_fn("Tone mapping...", 0.55);
    let tone_mode = match config.tone_mapping.as_str() {
        "reinhard" => ToneMappingMode::Reinhard,
        "none" => ToneMappingMode::None,
        _ => ToneMappingMode::AcesFilmic,
    };
    let frame = tone_map(frame, tone_mode);

    // ── 8. Sharpen ────────────────────────────────────────────────────────────
    progress_fn("Sharpening...", 0.65);
    let frame = unsharp_mask(frame, 0.4);

    // ── 9. AI super-resolution ────────────────────────────────────────────────
    let frame = if config.run_super_res {
        progress_fn("Enhancing detail...", 0.75);
        match run_super_res(&frame.pixels, h, w) {
            Ok(pixels) => {
                let new_h = frame.height * 2;
                let new_w = frame.width * 2;
                Frame::new(pixels, new_w, new_h)
            }
            Err(e) => {
                log::warn!("Super-res failed: {e} — skipping");
                frame
            }
        }
    } else {
        frame
    };

    // ── 10. Depth estimation + bokeh (portrait only) ──────────────────────────
    let frame = if config.run_depth {
        progress_fn("Applying bokeh...", 0.85);

        // Use original resolution for depth (super-res may have 2x'd the frame)
        let depth_h = if config.run_super_res {
            h
        } else {
            frame.height as usize
        };
        let depth_w = if config.run_super_res {
            w
        } else {
            frame.width as usize
        };

        // Sample down to original res for depth inference
        let depth_pixels = if config.run_super_res {
            // Downsample 2x frame back to original res for depth input
            use crate::ai::preprocess::normalize::resize_bilinear;
            resize_bilinear(
                &frame.pixels,
                frame.width as usize,
                frame.height as usize,
                depth_w,
                depth_h,
                3,
            )
        } else {
            frame.pixels.clone()
        };

        match run_depth(&depth_pixels, depth_h, depth_w) {
            Ok(disparity) => {
                // Upsample disparity to match current frame size if needed
                let disp_for_frame = if config.run_super_res {
                    use crate::ai::preprocess::normalize::resize_bilinear;
                    resize_bilinear(
                        &disparity,
                        depth_w,
                        depth_h,
                        frame.width as usize,
                        frame.height as usize,
                        1,
                    )
                } else {
                    disparity
                };
                apply_bokeh(frame, &disp_for_frame, 0.7, 8.0)
            }
            Err(e) => {
                log::warn!("Depth failed: {e} — skipping bokeh");
                frame
            }
        }
    } else {
        frame
    };

    // ── 11. Encode ────────────────────────────────────────────────────────────
    progress_fn("Saving...", 0.95);
    let jpeg = encode_frame_to_jpeg(&frame, 95)?;

    progress_fn("Done", 1.0);
    Ok(jpeg)
}

/// Detect scene from the first frame of a burst.
/// Returns Scene::Standard if classification fails.
pub fn detect_scene(first_frame_jpeg: &[u8]) -> Scene {
    match decode_jpeg_to_frame(first_frame_jpeg) {
        Ok(frame) => {
            let h = frame.height as usize;
            let w = frame.width as usize;
            match classify_scene(&frame.pixels, h, w) {
                Ok(ai_scene) => {
                    // Map from ai::Scene to pipeline::Scene
                    match ai_scene {
                        crate::ai::models::scene_cls::Scene::Night => Scene::Night,
                        crate::ai::models::scene_cls::Scene::Portrait => Scene::Portrait,
                        crate::ai::models::scene_cls::Scene::Landscape => Scene::Landscape,
                        crate::ai::models::scene_cls::Scene::Macro => Scene::Macro,
                        crate::ai::models::scene_cls::Scene::Standard => Scene::Standard,
                    }
                }
                Err(e) => {
                    log::warn!("Scene classification failed: {e}");
                    Scene::Standard
                }
            }
        }
        Err(e) => {
            log::warn!("Frame decode failed for scene detection: {e}");
            Scene::Standard
        }
    }
}
