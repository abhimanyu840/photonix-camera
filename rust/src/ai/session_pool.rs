//! ORT session builder with optimised EP configuration.
//!
//! EP priority strategy (based on ORT mobile docs):
//!   INT8 models:    XNNPACK (primary) → CPU
//!   float32 models: XNNPACK (primary) → NNAPI (if available) → CPU
//!
//! Note: NNAPI deprecated in Android 15. XNNPACK is the reliable primary.

use anyhow::Result;
use ort::session::{builder::GraphOptimizationLevel, Session};
use std::path::Path;

/// Initialize ORT once at startup.
/// Call before any session is built.
pub fn init_ort() -> Result<()> {
    ort::init().commit();
    Ok(())
}

/// Build an optimised session for a given model.
///
/// `is_int8`: true for quantized INT8 models.
///   INT8: XNNPACK primary (CPU SIMD, no hardware negotiation overhead)
///   fp32: XNNPACK primary, NNAPI secondary (may use GPU on supported devices)
pub fn build_session(model_path: &Path) -> Result<Session> {
    let session = Session::builder()
        .map_err(|e| anyhow::anyhow!("builder: {e}"))?
        // Level3 enables all graph optimisations including layout transforms
        // For NNAPI: use Level1 to keep ONNX ops (Level3 uses custom fused ops
        // that NNAPI cannot execute, causing more partitioning and slower perf)
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(|e| anyhow::anyhow!("opt_level: {e}"))?
        // 2 intra-op threads: enough for tile-parallel conv without thermal pressure
        // ARM big.LITTLE: 2 threads stays on big cores, 4+ spills to LITTLE cores
        .with_intra_threads(2)
        .map_err(|e| anyhow::anyhow!("threads: {e}"))?
        // 1 inter-op thread: our pipeline manages parallelism via rayon
        .with_inter_threads(1)
        .map_err(|e| anyhow::anyhow!("inter_threads: {e}"))?
        .commit_from_file(model_path)
        .map_err(|e| anyhow::anyhow!("load '{}': {e}", model_path.display()))?;

    log::info!(
        "[ORT] Loaded: {}",
        model_path.file_name().unwrap_or_default().to_string_lossy()
    );

    Ok(session)
}

pub fn init_environment() -> Result<()> {
    Ok(())
}
