//! Flutter ↔ Rust bridge API surface.
//! All public functions here are auto-exposed to Dart by frb codegen.

use crate::frb_generated::StreamSink;
use crate::pipeline::orchestrator::{detect_scene, run_full_pipeline};
use crate::pipeline::scene::Scene;

// ── Types exposed to Dart ─────────────────────────────────────────────────────

/// Progress update streamed to Dart during pipeline execution.
pub struct ProcessingUpdate {
    pub stage: String,
    pub progress: f32,
    pub is_complete: bool,
    pub result_bytes: Vec<u8>, // non-empty only when is_complete = true
    pub error: String,         // non-empty only on error
}

/// Returned by benchmark_roundtrip.
pub struct RoundtripResult {
    pub buffer_size_bytes: u64,
    pub rust_processing_us: u64,
    pub passed: bool,
    pub message: String,
}

// ── Core pipeline API ─────────────────────────────────────────────────────────

/// Process burst frames with live progress updates streamed to Dart.
///
/// Dart usage:
/// ```dart
/// final stream = captureAndProcess(frames: frames, sceneHint: "portrait");
/// await for (final update in stream) {
///   if (update.isComplete) {
///     // update.resultBytes contains the processed JPEG
///   } else {
///     // update.stage + update.progress for overlay
///   }
/// }
/// ```
pub fn capture_and_process(
    frames: Vec<Vec<u8>>,
    scene_hint: Option<String>,
    sink: StreamSink<ProcessingUpdate>,
) {
    // Run on background thread — never block the Dart UI thread
    std::thread::spawn(move || {
        let progress = {
            let sink_ref = &sink;
            move |stage: &str, progress: f32| {
                let _ = sink_ref.add(ProcessingUpdate {
                    stage: stage.to_string(),
                    progress,
                    is_complete: false,
                    result_bytes: vec![],
                    error: String::new(),
                });
            }
        };

        // Determine scene
        let scene = if let Some(hint) = &scene_hint {
            Scene::from_hint(hint)
        } else if !frames.is_empty() {
            detect_scene(&frames[0])
        } else {
            Scene::Standard
        };

        log::info!("[Pipeline] Scene: {:?}", scene);
        let config = scene.pipeline_config();

        // Run pipeline
        match run_full_pipeline(frames, &config, scene, progress) {
            Ok(jpeg_bytes) => {
                let _ = sink.add(ProcessingUpdate {
                    stage: "Done".to_string(),
                    progress: 1.0,
                    is_complete: true,
                    result_bytes: jpeg_bytes,
                    error: String::new(),
                });
            }
            Err(e) => {
                log::error!("[Pipeline] Failed: {e}");
                let _ = sink.add(ProcessingUpdate {
                    stage: String::new(),
                    progress: 0.0,
                    is_complete: false,
                    result_bytes: vec![],
                    error: e.to_string(),
                });
            }
        }
    });
}

/// Process a single JPEG frame synchronously (no progress stream).
/// Used by capture_coordinator for fast mode.
pub fn process_single(frame: Vec<u8>, scene_hint: Option<String>) -> Vec<u8> {
    let scene = scene_hint
        .as_deref()
        .map(Scene::from_hint)
        .unwrap_or_else(|| detect_scene(&frame));
    let config = scene.pipeline_config();

    run_full_pipeline(vec![frame], &config, scene, |_, _| {}).unwrap_or_else(|e| {
        log::error!("process_single failed: {e}");
        vec![]
    })
}

// ── P2 validation functions (kept for bridge test screen) ─────────────────────

pub fn get_engine_version() -> String {
    format!(
        "Photonix Engine v{} (Rust 1.82+, frb 2.11)",
        env!("CARGO_PKG_VERSION")
    )
}

pub fn process_image_bytes(bytes: Vec<u8>) -> Vec<u8> {
    bytes
}

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
/// Called once when RustLib.init() runs on app startup.
pub fn init_photonix_engine() {
    crate::configure_rayon();
    if let Err(e) = crate::ai::session_pool::init_ort() {
        log::error!("ORT init failed: {e}");
    }
    log::info!(
        "[Engine] Photonix Engine v{} initialised",
        env!("CARGO_PKG_VERSION")
    );
}

/// Pre-warm the scene classifier at app launch.
/// Loads the model into the LRU cache so the first capture doesn't pay
/// the model load cost (~50-200ms depending on storage speed).
///
/// Called in background after UI is visible — does NOT block startup.
pub fn prewarm_scene_classifier(model_path: String) {
    std::thread::spawn(move || {
        use crate::ai::model_cache::register_models;
        register_models(&[("scene_cls", &model_path)]);

        match crate::ai::model_cache::load_model("scene_cls") {
            Ok(_) => log::info!("[Prewarm] Scene classifier ready"),
            Err(e) => log::warn!("[Prewarm] Scene classifier failed: {e}"),
        }
    });
}
