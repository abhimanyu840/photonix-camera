mod frb_generated;

pub mod ai;
pub mod api;
pub mod compute;
pub mod pipeline;
/// Configure rayon thread pool for camera workloads.
///
/// ARM big.LITTLE awareness:
///   - Modern phones have 4 "big" + 4 "LITTLE" cores
///   - Rayon defaulting to 8 threads causes LITTLE core spill
///   - LITTLE cores run at ~0.4x speed and cause thermal throttle
///   - 4 threads = stays on big cores = consistent 60fps + no throttle
///
/// Rule of thumb: num_cpus / 2, minimum 2, maximum 4
pub fn configure_rayon() {
    let num_cores = num_cpus::get();
    let thread_count = (num_cores / 2).clamp(2, 4);

    rayon::ThreadPoolBuilder::new()
        .num_threads(thread_count)
        .thread_name(|i| format!("photonix-worker-{i}"))
        .build_global()
        .unwrap_or_else(|e| log::warn!("Rayon pool already initialised: {e}"));

    log::info!("[Rayon] Thread pool: {thread_count} threads (device has {num_cores} cores)");
}