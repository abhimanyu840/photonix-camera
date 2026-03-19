//! Scene detection and per-scene pipeline configuration.
//!
//! The scene classifier (MobileNetV3) runs first on every capture.
//! Its output selects a PipelineConfig that determines which stages run
//! and how many burst frames to capture.

/// Scene type detected from the image.
/// Re-exported from ai::models::scene_cls — kept here as the canonical type
/// used throughout the pipeline layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scene {
    Night,
    Portrait,
    Landscape,
    Macro,
    Standard,
}

impl Scene {
    pub fn as_str(&self) -> &'static str {
        match self {
            Scene::Night => "night",
            Scene::Portrait => "portrait",
            Scene::Landscape => "landscape",
            Scene::Macro => "macro",
            Scene::Standard => "standard",
        }
    }

    /// Parse from a hint string passed from Dart.
    pub fn from_hint(hint: &str) -> Self {
        match hint.to_lowercase().as_str() {
            "night" => Scene::Night,
            "portrait" => Scene::Portrait,
            "landscape" => Scene::Landscape,
            "macro" => Scene::Macro,
            _ => Scene::Standard,
        }
    }

    /// Returns the pipeline configuration for this scene.
    ///
    /// Timing targets (ARM64, NNAPI):
    ///   Night:     DnCNN(40ms) + Zero-DCE(15ms) + burst7(120ms) = ~260ms
    ///   Portrait:  DnCNN(40ms) + Real-ESRGAN(60ms) + MiDaS(80ms) = ~328ms
    ///   Landscape: Real-ESRGAN(60ms) + HDR(80ms) = ~300ms
    ///   Standard:  DnCNN(40ms) only = ~160ms
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
            },
            Scene::Portrait => PipelineConfig {
                run_denoiser: true,
                run_enhancer: false,
                run_super_res: true,
                run_depth: true,
                run_hdr: false,
                burst_count: 3,
                tone_mapping: "aces".to_string(),
            },
            Scene::Landscape => PipelineConfig {
                run_denoiser: false,
                run_enhancer: false,
                run_super_res: true,
                run_depth: false,
                run_hdr: true,
                burst_count: 3,
                tone_mapping: "aces".to_string(),
            },
            Scene::Macro => PipelineConfig {
                run_denoiser: true,
                run_enhancer: false,
                run_super_res: true,
                run_depth: false,
                run_hdr: false,
                burst_count: 3,
                tone_mapping: "aces".to_string(),
            },
            Scene::Standard => PipelineConfig {
                run_denoiser: true,
                run_enhancer: false,
                run_super_res: false,
                run_depth: false,
                run_hdr: false,
                burst_count: 3,
                tone_mapping: "aces".to_string(),
            },
        }
    }
}

/// Complete configuration for one capture-process cycle.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub run_denoiser: bool,
    pub run_enhancer: bool,
    pub run_super_res: bool,
    pub run_depth: bool,
    pub run_hdr: bool,
    pub burst_count: u8,
    pub tone_mapping: String,
}
