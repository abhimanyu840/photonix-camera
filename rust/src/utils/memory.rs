//! Memory utilities for performance monitoring.
//! Add this file as rust/src/utils/memory.rs
//! Then add `pub mod utils;` and `pub mod memory;` to lib.rs / utils/mod.rs

/// Returns current RSS (Resident Set Size) in bytes.
/// Reads from /proc/self/status — no special permissions needed on Android.
/// Returns 0 on non-Android platforms.
pub fn get_rss_bytes() -> u64 {
    #[cfg(target_os = "android")]
    {
        use std::io::Read;
        let mut content = String::new();
        if let Ok(mut f) = std::fs::File::open("/proc/self/status") {
            let _ = f.read_to_string(&mut content);
            for line in content.lines() {
                if line.starts_with("VmRSS:") {
                    let kb: u64 = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    return kb * 1024;
                }
            }
        }
        0
    }
    #[cfg(not(target_os = "android"))]
    {
        0
    }
}

/// Returns current RSS in megabytes — convenient for logging.
pub fn get_rss_mb() -> f32 {
    get_rss_bytes() as f32 / (1024.0 * 1024.0)
}