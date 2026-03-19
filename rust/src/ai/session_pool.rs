use anyhow::Result;
use ort::session::{builder::GraphOptimizationLevel, Session};
use std::path::Path;
use std::sync::Arc;

pub fn init_ort() -> Result<()> {
    ort::init().commit(); // returns bool, not Result — no ? needed
    Ok(())
}

pub fn build_session(model_path: &Path) -> Result<Session> {
    let session = Session::builder()
        .map_err(|e| anyhow::anyhow!("builder: {e}"))?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(|e| anyhow::anyhow!("opt_level: {e}"))?
        .with_intra_threads(2)
        .map_err(|e| anyhow::anyhow!("threads: {e}"))?
        .commit_from_file(model_path)
        .map_err(|e| anyhow::anyhow!("load: {e}"))?;
    Ok(session)
}

pub fn init_environment() -> Result<()> {
    Ok(())
}
