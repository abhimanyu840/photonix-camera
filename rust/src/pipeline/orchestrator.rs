//! Full pipeline orchestrator — Phase 11 + 11.5 combined.
//!
//! AI-guided color science is integrated at stage 4.
//! All conditional routing from Phase 11 is preserved.

use anyhow::Result;

use crate::ai::models::denoiser::run_denoiser;
use crate::ai::models::depth::run_depth;
use crate::ai::models::enhancer::run_enhancer;
use crate::ai::models::super_res::run_super_res;
use crate::ai::preprocess::normalize::resize_bilinear;
use crate::compute::burst_stack::{stack_burst_detailed, Frame, MotionClass};
use crate::compute::color::{white_balance, WhiteBalanceMode};
use crate::compute::color_science::{
    apply_color_science_with_params, build_skin_mask, predict_params_or_fallback,
};
use crate::compute::exposure::gamma_lift;
use crate::compute::hdr_merge::mertens_fusion;
use crate::compute::sharpen::unsharp_mask;
use crate::compute::tone_map::{linear_to_srgb, srgb_to_linear, tone_map, ToneMappingMode};
use crate::pipeline::bokeh::apply_bokeh;
use crate::pipeline::depth_refine::{detect_focus_threshold, refine_depth};
use crate::pipeline::face::apply_face_pipeline;
use crate::pipeline::scene::classify_scene;
use crate::pipeline::scene::{compute_image_stats, PipelineConfig, Scene};

// ── Decode / Encode ───────────────────────────────────────────────────────────

pub fn decode_jpeg_to_frame(jpeg: &[u8]) -> Result<Frame> {
    use image::{DynamicImage, ImageReader, RgbImage};
    use std::io::Cursor;
    let reader = ImageReader::new(Cursor::new(jpeg)).with_guessed_format()?;
    let rgb: RgbImage = reader.decode()?.into_rgb8();
    let (w, h) = rgb.dimensions();
    let pixels: Vec<f32> = rgb
        .as_raw()
        .iter()
        .map(|&b| srgb_to_linear(b as f32 / 255.0))
        .collect();
    Ok(Frame::new(pixels, w, h))
}

pub fn encode_frame_to_jpeg(frame: &Frame, quality: u8) -> Result<Vec<u8>> {
    use image::codecs::jpeg::JpegEncoder;
    use image::RgbImage;
    let u8_pixels: Vec<u8> = frame
        .pixels
        .iter()
        .map(|&x| (linear_to_srgb(x.clamp(0.0, 1.0)) * 255.0) as u8)
        .collect();
    let rgb = RgbImage::from_raw(frame.width, frame.height, u8_pixels)
        .ok_or_else(|| anyhow::anyhow!("RgbImage creation failed"))?;
    let mut buf = Vec::new();
    JpegEncoder::new_with_quality(&mut buf, quality).encode_image(&rgb)?;
    Ok(buf)
}

// ── Main pipeline ─────────────────────────────────────────────────────────────

pub fn run_full_pipeline(
    jpeg_frames: Vec<Vec<u8>>,
    config: &PipelineConfig,
    scene: Scene,
    mut progress: impl FnMut(&str, f32),
) -> Result<Vec<u8>> {
    // ── 1. Decode ─────────────────────────────────────────────────────────────
    progress("Decoding...", 0.04);
    let frames: Vec<Frame> = jpeg_frames
        .iter()
        .map(|b| decode_jpeg_to_frame(b))
        .collect::<Result<_>>()?;

    let w = frames[0].width as usize;
    let h = frames[0].height as usize;

    // ── 2. Image statistics (shared by scene detect + AI color) ───────────────
    let stats = compute_image_stats(&frames[0].pixels, w, h);
    log::debug!(
        "[Pipeline] luma={:.2} noise={:.3} contrast={:.1}",
        stats.mean_luma,
        stats.noise_sigma,
        stats.contrast_ratio
    );

    // ── 3. White balance ──────────────────────────────────────────────────────
    progress("White balance...", 0.08);
    let frames: Vec<Frame> = frames
        .into_iter()
        .map(|f| white_balance(f, WhiteBalanceMode::GreyWorld))
        .collect();

    // ── 4. AI-guided color science ────────────────────────────────────────────
    //
    // Compute 224×224 thumbnail ONCE — shared by ColorParamNet.
    // The thumbnail is also available to scene classifier if called here.
    // Full-resolution frame is never passed to ONNX.
    //
    // Performance budget for this block:
    //   thumbnail generation : ~1ms
    //   ColorParamNet (ONNX) : ~6ms   (skipped if model unavailable)
    //   param clamping       : <0.1ms
    //   skin mask (heuristic): ~2ms
    //   MODNet (ONNX)        : ~25ms  (skipped if model unavailable)
    //   apply_color_science  : ~3ms   (SIMD, unchanged)
    //   Total                : ~8ms without MODNet, ~33ms with MODNet
    //
    progress("Color grading...", 0.13);

    let thumb = resize_bilinear(&frames[0].pixels, w, h, 224, 224, 3);
    let params = predict_params_or_fallback(&thumb, &stats, scene);

    log::debug!(
        "[ColorAI] sat={:.2} shadow_lift={:.3} highlight_roll={:.2}",
        params.saturation,
        params.shadow_lift,
        params.highlight_roll
    );

    // Build skin mask (heuristic always, MODNet optional)
    let skin_mask = build_skin_mask(&frames[0]);

    let frames: Vec<Frame> = frames
        .into_iter()
        .map(|f| apply_color_science_with_params(f, &params, Some(&skin_mask)))
        .collect();

    // ── 5. Burst stack or HDR ─────────────────────────────────────────────────
    let (frame, motion_class) = if config.run_hdr && frames.len() > 1 {
        progress("HDR merge...", 0.22);
        (mertens_fusion(&frames), MotionClass::Low)
    } else if frames.len() > 1 {
        progress("Stacking frames...", 0.22);
        let result = stack_burst_detailed(frames)?;
        log::info!(
            "[Pipeline] motion={} frames_used={}",
            result.motion_class.as_str(),
            result.frames_used
        );
        (result.frame, result.motion_class)
    } else {
        (frames.into_iter().next().unwrap(), MotionClass::Low)
    };

    // ── 6. AI Denoiser (skip if noise too low) ────────────────────────────────
    let frame = if config.run_denoiser && stats.noise_sigma >= 0.02 {
        progress("Denoising...", 0.32);
        match run_denoiser(&frame.pixels, h, w) {
            Ok(px) => {
                log::debug!("[Pipeline] Denoiser applied");
                Frame::new(px, frame.width, frame.height)
            }
            Err(e) => {
                log::warn!("[Pipeline] Denoiser skip: {e}");
                frame
            }
        }
    } else {
        log::debug!("[Pipeline] Skip denoiser: noise={:.3}", stats.noise_sigma);
        frame
    };

    // ── 7. AI Enhancer (skip if well-exposed) ─────────────────────────────────
    let frame = if config.run_enhancer && stats.mean_luma <= 0.35 {
        progress("Enhancing...", 0.40);
        match run_enhancer(&frame.pixels, h, w) {
            Ok(px) => Frame::new(px, frame.width, frame.height),
            Err(e) => {
                log::warn!("[Pipeline] Enhancer skip: {e}");
                frame
            }
        }
    } else {
        log::debug!("[Pipeline] Skip enhancer: luma={:.2}", stats.mean_luma);
        frame
    };

    // ── 8. Exposure lift ──────────────────────────────────────────────────────
    let frame = gamma_lift(frame, 0.05);

    // ── 9. Tone mapping ───────────────────────────────────────────────────────
    progress("Tone mapping...", 0.50);
    let mode = match config.tone_mapping.as_str() {
        "reinhard" => ToneMappingMode::Reinhard,
        "none" => ToneMappingMode::None,
        _ => ToneMappingMode::AcesFilmic,
    };
    let frame = tone_map(frame, mode);

    // ── 10. Sharpen ───────────────────────────────────────────────────────────
    progress("Sharpening...", 0.58);
    let frame = unsharp_mask(frame, 0.4);

    // ── 11. AI Super-resolution (skip if already >= 8MP) ─────────────────────
    let current_mp = (w * h) as f32 / 1_000_000.0;
    let frame = if config.run_super_res && current_mp < 8.0 {
        progress("Enhancing detail...", 0.68);
        match run_super_res(&frame.pixels, h, w) {
            Ok(px) => Frame::new(px, frame.width * 2, frame.height * 2),
            Err(e) => {
                log::warn!("[Pipeline] Super-res skip: {e}");
                frame
            }
        }
    } else {
        log::debug!("[Pipeline] Skip super-res: {:.1}MP", current_mp);
        frame
    };

    // ── 12. Face pipeline (Portrait only) ─────────────────────────────────────
    let (frame, face_region) = if scene == Scene::Portrait {
        progress("Face enhancement...", 0.76);
        let (f, found) = apply_face_pipeline(frame);
        let face = if found {
            crate::pipeline::face::detect_face_region(&f)
        } else {
            None
        };
        (f, face)
    } else {
        (frame, None)
    };

    // ── 13. Depth + bokeh (Portrait only, skip if depth unavailable) ──────────
    let frame = if config.run_depth {
        progress("Applying bokeh...", 0.84);
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

        let depth_input = if config.run_super_res {
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

        match run_depth(&depth_input, depth_h, depth_w) {
            Ok(raw_depth) => {
                let refined = refine_depth(
                    &depth_input,
                    &raw_depth,
                    depth_w,
                    depth_h,
                    face_region.as_ref(),
                );
                let disp = if config.run_super_res {
                    resize_bilinear(
                        &refined,
                        depth_w,
                        depth_h,
                        frame.width as usize,
                        frame.height as usize,
                        1,
                    )
                } else {
                    refined
                };
                let focus =
                    detect_focus_threshold(&disp, frame.width as usize, face_region.as_ref());
                apply_bokeh(frame, &disp, focus, 8.0)
            }
            Err(e) => {
                log::warn!("[Pipeline] Depth skip: {e}");
                frame
            }
        }
    } else {
        frame
    };

    // ── 14. Encode ────────────────────────────────────────────────────────────
    progress("Saving...", 0.95);
    let jpeg = encode_frame_to_jpeg(&frame, 95)?;
    progress("Done", 1.0);
    Ok(jpeg)
}

pub fn detect_scene(first_frame_jpeg: &[u8]) -> Scene {
    match decode_jpeg_to_frame(first_frame_jpeg) {
        Ok(frame) => {
            match classify_scene(&frame.pixels, frame.height as usize, frame.width as usize) {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("[Scene] {e}");
                    Scene::Standard
                }
            }
        }
        Err(e) => {
            log::warn!("[Scene] Decode: {e}");
            Scene::Standard
        }
    }
}
