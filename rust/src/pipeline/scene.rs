//! Multi-signal scene detection engine.
//!
//! Combines:
//!   1. Fast luma heuristics (< 1ms, always runs first)
//!   2. MobileNetV3 top-3 probabilities (runs if luma is ambiguous)
//!   3. Contrast ratio + noise level for fine-grained routing

// ── Scene enum ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scene {
    Night,
    Portrait,
    Landscape,
    Macro,
    Document,
    Backlit,
    Standard,
}

use crate::compute::color_science::ColorProfile;

impl Scene {
    pub fn as_str(&self) -> &'static str {
        match self {
            Scene::Night => "night",
            Scene::Portrait => "portrait",
            Scene::Landscape => "landscape",
            Scene::Macro => "macro",
            Scene::Document => "document",
            Scene::Backlit => "backlit",
            Scene::Standard => "standard",
        }
    }

    pub fn from_hint(hint: &str) -> Self {
        match hint.to_lowercase().as_str() {
            "night" => Scene::Night,
            "portrait" => Scene::Portrait,
            "landscape" => Scene::Landscape,
            "macro" => Scene::Macro,
            "document" => Scene::Document,
            "backlit" => Scene::Backlit,
            _ => Scene::Standard,
        }
    }

    pub fn pipeline_config(&self) -> PipelineConfig {
        match self {
            Scene::Night => PipelineConfig {
                run_denoiser: true,
                run_enhancer: true,
                run_super_res: false,
                run_depth: false,
                run_hdr: false,
                burst_count: 7,
                tone_mapping: "aces".to_string(),
                color_profile: crate::compute::color_science::ColorProfile::Natural,
            },
            Scene::Portrait => PipelineConfig {
                run_denoiser: true,
                run_enhancer: false,
                run_super_res: true,
                run_depth: true,
                run_hdr: false,
                burst_count: 3,
                tone_mapping: "aces".to_string(),
                color_profile: crate::compute::color_science::ColorProfile::Natural,
            },
            Scene::Landscape => PipelineConfig {
                run_denoiser: false,
                run_enhancer: false,
                run_super_res: true,
                run_depth: false,
                run_hdr: true,
                burst_count: 3,
                tone_mapping: "aces".to_string(),
                color_profile: crate::compute::color_science::ColorProfile::Vivid,
            },
            Scene::Macro => PipelineConfig {
                run_denoiser: true,
                run_enhancer: false,
                run_super_res: true,
                run_depth: false,
                run_hdr: false,
                burst_count: 3,
                tone_mapping: "aces".to_string(),
                color_profile: crate::compute::color_science::ColorProfile::Natural,
            },
            Scene::Document => PipelineConfig {
                run_denoiser: false,
                run_enhancer: false,
                run_super_res: true,
                run_depth: false,
                run_hdr: false,
                burst_count: 1,
                tone_mapping: "none".to_string(),
                color_profile: crate::compute::color_science::ColorProfile::Natural,
            },
            Scene::Backlit => PipelineConfig {
                run_denoiser: false,
                run_enhancer: true,
                run_super_res: false,
                run_depth: false,
                run_hdr: true,
                burst_count: 3,
                tone_mapping: "aces".to_string(),
                color_profile: crate::compute::color_science::ColorProfile::Cinema,
            },
            Scene::Standard => PipelineConfig {
                run_denoiser: true,
                run_enhancer: false,
                run_super_res: false,
                run_depth: false,
                run_hdr: false,
                burst_count: 3,
                tone_mapping: "aces".to_string(),
                color_profile: crate::compute::color_science::ColorProfile::Natural,
            },
        }
    }
}

// ── Pipeline config ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub run_denoiser: bool,
    pub run_enhancer: bool,
    pub run_super_res: bool,
    pub run_depth: bool,
    pub run_hdr: bool,
    pub burst_count: u8,
    pub tone_mapping: String,
    pub color_profile: crate::compute::color_science::ColorProfile,
}

// ── Image statistics ──────────────────────────────────────────────────────────

pub struct ImageStats {
    pub mean_luma: f32,
    pub contrast_ratio: f32, // p95 / (p5 + 1e-6)
    pub noise_sigma: f32,    // MAD estimator
    pub is_bimodal: bool,    // true if backlit histogram
    pub mostly_white: bool,  // true if document
}

pub fn compute_image_stats(pixels: &[f32], _w: usize, _h: usize) -> ImageStats {
    let lumas: Vec<f32> = pixels
        .chunks_exact(3)
        .map(|p| 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2])
        .collect();

    if lumas.is_empty() {
        return ImageStats {
            mean_luma: 0.5,
            contrast_ratio: 1.0,
            noise_sigma: 0.0,
            is_bimodal: false,
            mostly_white: false,
        };
    }

    let mean_luma = lumas.iter().sum::<f32>() / lumas.len() as f32;

    // Contrast ratio: p95 / p5
    let mut sorted = lumas.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p5 = sorted[(sorted.len() as f32 * 0.05) as usize];
    let p95 = sorted[(sorted.len() as f32 * 0.95) as usize];
    let contrast_ratio = p95 / (p5 + 1e-6);

    // Noise sigma via MAD (Median Absolute Deviation)
    let median = sorted[sorted.len() / 2];
    let mut abs_devs: Vec<f32> = lumas.iter().map(|&x| (x - median).abs()).collect();
    abs_devs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mad = abs_devs[abs_devs.len() / 2];
    let noise_sigma = 1.4826 * mad; // consistent estimator for Gaussian noise

    // Bimodal check (backlit): count dark (<0.2) and bright (>0.8) pixels
    let dark_frac = lumas.iter().filter(|&&l| l < 0.2).count() as f32 / lumas.len() as f32;
    let bright_frac = lumas.iter().filter(|&&l| l > 0.8).count() as f32 / lumas.len() as f32;
    let is_bimodal = dark_frac > 0.25 && bright_frac > 0.25;

    // Mostly white check (document)
    let white_frac = lumas.iter().filter(|&&l| l > 0.75).count() as f32 / lumas.len() as f32;
    let mostly_white = white_frac > 0.55 && contrast_ratio > 3.0;

    ImageStats {
        mean_luma,
        contrast_ratio,
        noise_sigma,
        is_bimodal,
        mostly_white,
    }
}

// ── Luma-only fast classification ─────────────────────────────────────────────

pub fn classify_by_luma(stats: &ImageStats) -> Option<Scene> {
    if stats.mean_luma < 0.12 {
        return Some(Scene::Night);
    }
    if stats.is_bimodal && stats.mean_luma > 0.4 {
        return Some(Scene::Backlit);
    }
    if stats.mostly_white {
        return Some(Scene::Document);
    }
    None // ambiguous — run model
}

// ── AI model classification ───────────────────────────────────────────────────

pub fn classify_scene(img_rgb: &[f32], height: usize, width: usize) -> anyhow::Result<Scene> {
    use crate::ai::model_cache::load_model;
    use crate::ai::preprocess::normalize::{normalize_imagenet, resize_bilinear};
    use ort::value::Tensor;

    let stats = compute_image_stats(img_rgb, width, height);

    // Fast luma path — skip model if obvious
    if let Some(scene) = classify_by_luma(&stats) {
        log::debug!("[Scene] Fast-path: {}", scene.as_str());
        return Ok(scene);
    }

    // Run MobileNetV3 for ambiguous scenes
    let session = load_model(crate::ai::model_cache::MODEL_KEY_SCENE)?;
    let mut session = session.lock().unwrap();

    let resized = resize_bilinear(img_rgb, width, height, 224, 224, 3);
    let tensor = normalize_imagenet(&resized, 224, 224);
    let input = Tensor::from_array(tensor.into_dyn())?;
    let outputs = session.run(ort::inputs![input])?;
    let logits = outputs[0].try_extract_array::<f32>()?;

    let n = logits.shape()[1];

    // Get top-3 indices + confidences
    let mut indexed: Vec<(usize, f32)> = (0..n).map(|i| (i, logits[[0, i]])).collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let top3 = &indexed[..3.min(indexed.len())];

    let top1_conf = top3[0].1;
    let top1_idx = top3[0].0;

    log::debug!("[Scene] top1={top1_idx} conf={top1_conf:.2}");

    // Confidence gating: if uncertain, fall back to luma heuristics
    if top1_conf < 0.45 {
        log::debug!("[Scene] Low confidence ({top1_conf:.2}) — using luma heuristics");
        return Ok(scene_from_stats(&stats));
    }

    Ok(scene_from_imagenet_class(top1_idx, &stats))
}

fn scene_from_imagenet_class(class: usize, stats: &ImageStats) -> Scene {
    match class {
        // Person / face classes
        0..=9 | 840 | 878 | 895 => {
            if stats.noise_sigma > 0.08 {
                Scene::Night
            } else {
                Scene::Portrait
            }
        }
        // Outdoor / natural scenes
        970..=980 => Scene::Landscape,
        // Close-up / detailed
        300..=400 | 984..=987 => Scene::Macro,
        _ => scene_from_stats(stats),
    }
}

fn scene_from_stats(stats: &ImageStats) -> Scene {
    if stats.mean_luma < 0.15 {
        return Scene::Night;
    }
    if stats.is_bimodal {
        return Scene::Backlit;
    }
    if stats.mostly_white {
        return Scene::Document;
    }
    if stats.contrast_ratio > 5.0 {
        return Scene::Landscape;
    }
    if stats.noise_sigma > 0.05 {
        return Scene::Night;
    }
    Scene::Standard
}
