//! Hybrid AI-guided DSLR color science engine.
//!
//! Architecture:
//!   1. Fast luma stats (< 1ms, always)
//!   2. ColorParamNet prediction on 224×224 thumbnail (< 10ms, optional)
//!   3. Parameter clamping by scene bounds (< 0.1ms)
//!   4. Existing SIMD per-pixel S-curve + HSL saturation (< 3ms)
//!   5. MODNet skin mask refinement (< 25ms, optional)
//!
//! The classical pipeline is NEVER removed — AI only controls its parameters.

use crate::compute::burst_stack::Frame;
use crate::pipeline::scene::{ImageStats, Scene};

// ── Color profiles (static fallbacks) ────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorProfile {
    Natural,
    Vivid,
    Cinema,
}

impl ColorProfile {
    pub fn as_str(&self) -> &'static str {
        match self {
            ColorProfile::Natural => "natural",
            ColorProfile::Vivid => "vivid",
            ColorProfile::Cinema => "cinema",
        }
    }
}

// ── Profile parameters ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ProfileParams {
    pub shadow_lift: f32,     // [0.0, 0.08]
    pub shadow_thresh: f32,   // [0.0, 0.20]
    pub highlight_roll: f32,  // [0.75, 0.92]
    pub midtone_boost: f32,   // [0.0, 0.12]
    pub saturation: f32,      // [0.70, 1.30]
    pub highlight_desat: f32, // [0.0, 0.40]
}

impl ProfileParams {
    pub fn natural() -> Self {
        Self {
            shadow_lift: 0.03,
            shadow_thresh: 0.10,
            highlight_roll: 0.85,
            midtone_boost: 0.04,
            saturation: 1.00,
            highlight_desat: 0.00,
        }
    }
    pub fn vivid() -> Self {
        Self {
            shadow_lift: 0.02,
            shadow_thresh: 0.08,
            highlight_roll: 0.88,
            midtone_boost: 0.08,
            saturation: 1.15,
            highlight_desat: 0.00,
        }
    }
    pub fn cinema() -> Self {
        Self {
            shadow_lift: 0.05,
            shadow_thresh: 0.15,
            highlight_roll: 0.80,
            midtone_boost: 0.02,
            saturation: 0.90,
            highlight_desat: 0.25,
        }
    }
}

impl ColorProfile {
    pub fn params(&self) -> ProfileParams {
        match self {
            ColorProfile::Natural => ProfileParams::natural(),
            ColorProfile::Vivid => ProfileParams::vivid(),
            ColorProfile::Cinema => ProfileParams::cinema(),
        }
    }
}

// ── Scene-specific parameter bounds ──────────────────────────────────────────

pub struct ParamBounds {
    pub saturation_max: f32,
    pub shadow_lift_max: f32,
    pub midtone_boost_max: f32,
    pub highlight_desat_max: f32,
}

impl Scene {
    pub fn param_bounds(&self) -> ParamBounds {
        match self {
            Scene::Portrait => ParamBounds {
                saturation_max: 0.95,
                shadow_lift_max: 0.06,
                midtone_boost_max: 0.05,
                highlight_desat_max: 0.10,
            },
            Scene::Landscape => ParamBounds {
                saturation_max: 1.30,
                shadow_lift_max: 0.04,
                midtone_boost_max: 0.10,
                highlight_desat_max: 0.15,
            },
            Scene::Night => ParamBounds {
                saturation_max: 0.90,
                shadow_lift_max: 0.08,
                midtone_boost_max: 0.03,
                highlight_desat_max: 0.05,
            },
            Scene::Backlit => ParamBounds {
                saturation_max: 1.10,
                shadow_lift_max: 0.07,
                midtone_boost_max: 0.06,
                highlight_desat_max: 0.30,
            },
            Scene::Document => ParamBounds {
                saturation_max: 0.80,
                shadow_lift_max: 0.02,
                midtone_boost_max: 0.08,
                highlight_desat_max: 0.00,
            },
            _ => ParamBounds {
                saturation_max: 1.15,
                shadow_lift_max: 0.05,
                midtone_boost_max: 0.08,
                highlight_desat_max: 0.20,
            },
        }
    }

    pub fn default_profile(&self) -> ColorProfile {
        match self {
            Scene::Night | Scene::Backlit => ColorProfile::Cinema,
            Scene::Landscape => ColorProfile::Vivid,
            _ => ColorProfile::Natural,
        }
    }
}

// ── AI parameter prediction ───────────────────────────────────────────────────

/// Run ColorParamNet to predict ProfileParams dynamically.
/// Falls back to scene defaults if model unavailable or low confidence.
///
/// ColorParamNet architecture (MobileNetV2-Micro + regression head):
///   Conv 3×3 s2 → 16ch (112×112)
///   DepthwiseSep × 3   → 128ch (14×14)
///   GlobalAvgPool      → 128
///   Concat [128, 6 stats] → FC 134→64→7
///   Outputs: 6 params + 1 confidence, all sigmoid-bounded
///   Size: ~1.8MB INT8, ~6ms XNNPACK on Dimensity 1080
pub fn predict_params_or_fallback(
    img_thumb: &[f32], // 224×224×3, normalized [0,1]
    stats: &ImageStats,
    scene: Scene,
) -> ProfileParams {
    let bounds = scene.param_bounds();
    let default = scene.default_profile().params();

    // Try AI prediction if model is registered
    match try_run_color_param_net(img_thumb, stats) {
        Ok((raw, confidence)) if confidence >= 0.60 => {
            log::debug!("[ColorAI] conf={confidence:.2} → AI params");
            ProfileParams {
                shadow_lift: raw.shadow_lift.clamp(0.0, bounds.shadow_lift_max),
                shadow_thresh: raw.shadow_thresh.clamp(0.0, 0.20),
                highlight_roll: raw.highlight_roll.clamp(0.75, 0.92),
                midtone_boost: raw.midtone_boost.clamp(0.0, bounds.midtone_boost_max),
                saturation: raw.saturation.clamp(0.70, bounds.saturation_max),
                highlight_desat: raw.highlight_desat.clamp(0.0, bounds.highlight_desat_max),
            }
        }
        Ok((_, conf)) => {
            log::debug!("[ColorAI] Low confidence ({conf:.2}) → default");
            default
        }
        Err(e) => {
            log::debug!("[ColorAI] Model unavailable ({e}) → default");
            default
        }
    }
}

fn try_run_color_param_net(
    img_thumb: &[f32],
    stats: &ImageStats,
) -> anyhow::Result<(ProfileParams, f32)> {
    use crate::ai::model_cache::load_model;
    use ndarray::{Array1, Array4};
    use ort::value::Tensor;

    let session = load_model(crate::ai::model_cache::MODEL_KEY_COLOR_PARAMS)?;
    let mut session = session.lock().unwrap();

    // Image tensor: [1, 3, 224, 224]
    let img_array = {
        let mut t = Array4::<f32>::zeros((1, 3, 224, 224));
        for py in 0..224 {
            for px in 0..224 {
                let i = (py * 224 + px) * 3;
                t[[0, 0, py, px]] = img_thumb[i];
                t[[0, 1, py, px]] = img_thumb[i + 1];
                t[[0, 2, py, px]] = img_thumb[i + 2];
            }
        }
        t
    };

    // Scalar stats: [1, 6]
    let stats_array = Array1::<f32>::from(vec![
        stats.mean_luma,
        stats.contrast_ratio.min(10.0) / 10.0,
        stats.noise_sigma.clamp(0.0, 0.2) / 0.2,
        if stats.is_bimodal { 1.0 } else { 0.0 },
        if stats.mostly_white { 1.0 } else { 0.0 },
        0.0, // reserved
    ]);

    let img_input = Tensor::from_array(img_array.into_dyn())?;
    let stats_input = Tensor::from_array(stats_array.into_dyn())?;

    let outputs = session.run(ort::inputs![img_input, stats_input])?;
    let out = outputs[0].try_extract_array::<f32>()?;

    // Decode bounded outputs (sigmoid × range, applied by model)
    let o = |i: usize| out[[0, i]];
    let raw = ProfileParams {
        shadow_lift: o(0),
        shadow_thresh: o(1),
        highlight_roll: o(2),
        midtone_boost: o(3),
        saturation: o(4),
        highlight_desat: o(5),
    };
    let confidence = o(6);

    Ok((raw, confidence))
}

// ── Skin tone detection ───────────────────────────────────────────────────────

/// Returns skin weight [0,1] per pixel.
/// Primary: RGB heuristic (always available, 0ms overhead)
/// Enhancement: MODNet alpha matte (when model loaded, ~25ms)
pub fn build_skin_mask(frame: &Frame) -> Vec<f32> {
    let n = (frame.width * frame.height) as usize;
    let mut mask = vec![0.0f32; n];

    // RGB heuristic (Kovac et al.) — primary path
    for px in 0..n {
        let i = px * 3;
        let r = frame.pixels[i];
        let g = frame.pixels[i + 1];
        let b = frame.pixels[i + 2];
        if is_skin_tone(r, g, b) {
            mask[px] = 1.0;
        }
    }

    // Try MODNet refinement if model available
    if let Ok(modnet_alpha) = try_run_modnet(frame) {
        // Combine: heuristic × MODNet → sharper boundaries
        for px in 0..n {
            mask[px] *= modnet_alpha[px];
        }
        log::debug!("[Skin] MODNet mask applied");
    }

    mask
}

fn try_run_modnet(frame: &Frame) -> anyhow::Result<Vec<f32>> {
    use crate::ai::model_cache::load_model;
    use crate::ai::preprocess::normalize::{hwc_to_nchw, resize_bilinear};
    use ort::value::Tensor;

    let session = load_model(crate::ai::model_cache::MODEL_KEY_MODNET)?;
    let mut session = session.lock().unwrap();

    let w = frame.width as usize;
    let h = frame.height as usize;

    // MODNet input: 512×512
    let resized = resize_bilinear(&frame.pixels, w, h, 512, 512, 3);
    let tensor = hwc_to_nchw(&resized, 512, 512, 3);
    let input = Tensor::from_array(tensor.into_dyn())?;
    let outputs = session.run(ort::inputs![input])?;
    let alpha = outputs[0].try_extract_array::<f32>()?;

    // Upsample alpha matte from 512×512 back to original resolution
    let alpha_vec: Vec<f32> = alpha.iter().copied().collect();
    Ok(resize_bilinear(&alpha_vec, 512, 512, w, h, 1))
}

// ── Skin tone RGB heuristic ───────────────────────────────────────────────────

#[inline(always)]
pub fn is_skin_tone(r: f32, g: f32, b: f32) -> bool {
    let r = r * 255.0;
    let g = g * 255.0;
    let b = b * 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    r > 95.0 && g > 40.0 && b > 20.0 && (max - min) > 15.0 && (r - g).abs() > 15.0 && r > g && r > b
}

// ── Tone curve ────────────────────────────────────────────────────────────────

#[inline(always)]
fn apply_tone_curve(x: f32, p: &ProfileParams) -> f32 {
    // Shadow lift
    let x = if x < p.shadow_thresh {
        let t = x / p.shadow_thresh;
        x + p.shadow_lift * (1.0 - t)
    } else {
        x
    };

    // Midtone contrast (S-curve)
    let x = {
        let dev = x - 0.5;
        (x + p.midtone_boost * dev * (1.0 - dev.abs() * 2.0)).clamp(0.0, 1.0)
    };

    // Highlight rolloff
    if x > p.highlight_roll {
        let t = (x - p.highlight_roll) / (1.0 - p.highlight_roll);
        p.highlight_roll + (1.0 - p.highlight_roll) * (1.0 - (1.0 - t).powi(2))
    } else {
        x
    }
    .clamp(0.0, 1.0)
}

// ── HSL helpers ───────────────────────────────────────────────────────────────

fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) * 0.5;
    if (max - min).abs() < 1e-6 {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if (max - r).abs() < 1e-6 {
        ((g - b) / d + if g < b { 6.0 } else { 0.0 }) / 6.0
    } else if (max - g).abs() < 1e-6 {
        ((b - r) / d + 2.0) / 6.0
    } else {
        ((r - g) / d + 4.0) / 6.0
    };
    (h, s, l)
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s < 1e-6 {
        return (l, l, l);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    (
        hue_to_rgb(p, q, h + 1.0 / 3.0),
        hue_to_rgb(p, q, h),
        hue_to_rgb(p, q, h - 1.0 / 3.0),
    )
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 0.5 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

// ── Main color science function (UNCHANGED interface) ─────────────────────────

/// Apply color science with a static ColorProfile (used as fallback / settings).
pub fn apply_color_science(frame: Frame, profile: ColorProfile) -> Frame {
    apply_color_science_with_params(frame, &profile.params(), None)
}

/// Apply color science with explicit ProfileParams (used by AI-guided path).
/// `skin_mask`: optional per-pixel weight [0,1] — None uses RGB heuristic only.
pub fn apply_color_science_with_params(
    frame: Frame,
    p: &ProfileParams,
    skin_mask: Option<&[f32]>,
) -> Frame {
    let n = frame.pixels.len() / 3;
    let mut output = frame.pixels.clone();

    for px in 0..n {
        let i = px * 3;
        let r = frame.pixels[i];
        let g = frame.pixels[i + 1];
        let b = frame.pixels[i + 2];

        // Tone curve per channel
        let r = apply_tone_curve(r, p);
        let g = apply_tone_curve(g, p);
        let b = apply_tone_curve(b, p);

        // Saturation in HSL space
        let (h, s, l) = rgb_to_hsl(r, g, b);

        // Skin protection: reduce saturation boost by 50% on skin pixels
        let skin_w =
            skin_mask.map(|m| m[px]).unwrap_or_else(
                || {
                    if is_skin_tone(r, g, b) {
                        1.0
                    } else {
                        0.0
                    }
                },
            );
        let sat_mult = 1.0 + (p.saturation - 1.0) * (1.0 - skin_w * 0.5);
        let new_s = (s * sat_mult).clamp(0.0, 1.0);

        // Highlight desaturation (Cinema profile)
        let new_s = if p.highlight_desat > 0.0 && l > 0.75 {
            let t = (l - 0.75) / 0.25;
            new_s * (1.0 - p.highlight_desat * t)
        } else {
            new_s
        };

        let (r, g, b) = hsl_to_rgb(h, new_s.clamp(0.0, 1.0), l);
        output[i] = r.clamp(0.0, 1.0);
        output[i + 1] = g.clamp(0.0, 1.0);
        output[i + 2] = b.clamp(0.0, 1.0);
    }

    Frame::new(output, frame.width, frame.height)
}

// ── NEON fast saturation (ARM64 hot path) ─────────────────────────────────────

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
pub unsafe fn apply_saturation_neon(pixels: &mut [f32], saturation: f32) {
    use std::arch::aarch64::*;
    let sat = vdupq_n_f32(saturation);
    let inv = vdupq_n_f32(1.0 - saturation);
    let chunks = pixels.len() / 12;
    let ptr = pixels.as_mut_ptr();
    for i in 0..chunks {
        let base = ptr.add(i * 12);
        let v0 = vld1q_f32(base);
        let v1 = vld1q_f32(base.add(4));
        let v2 = vld1q_f32(base.add(8));
        let luma = vaddq_f32(
            vaddq_f32(
                vmulq_f32(v0, vdupq_n_f32(0.2126)),
                vmulq_f32(v1, vdupq_n_f32(0.7152)),
            ),
            vmulq_f32(v2, vdupq_n_f32(0.0722)),
        );
        vst1q_f32(base, vaddq_f32(vmulq_f32(v0, sat), vmulq_f32(luma, inv)));
        vst1q_f32(
            base.add(4),
            vaddq_f32(vmulq_f32(v1, sat), vmulq_f32(luma, inv)),
        );
        vst1q_f32(
            base.add(8),
            vaddq_f32(vmulq_f32(v2, sat), vmulq_f32(luma, inv)),
        );
    }
}

#[cfg(not(target_arch = "aarch64"))]
pub fn apply_saturation_neon(pixels: &mut [f32], saturation: f32) {
    for chunk in pixels.chunks_exact_mut(3) {
        let luma = 0.2126 * chunk[0] + 0.7152 * chunk[1] + 0.0722 * chunk[2];
        chunk[0] = (luma + saturation * (chunk[0] - luma)).clamp(0.0, 1.0);
        chunk[1] = (luma + saturation * (chunk[1] - luma)).clamp(0.0, 1.0);
        chunk[2] = (luma + saturation * (chunk[2] - luma)).clamp(0.0, 1.0);
    }
}
