//! Edge-aware depth map refinement using a guided filter.
//!
//! Problem: MiDaS outputs blurry depth edges → bokeh bleeds across
//! subject boundaries (hair, shoulders).
//!
//! Solution: guided filter uses the RGB image as a guide to sharpen
//! depth edges — depth boundaries align with RGB color edges.

use crate::pipeline::face::FaceRegion;

// ── Guided filter ─────────────────────────────────────────────────────────────

/// Apply guided filter to refine a depth map.
///
/// `guide`: RGB luma image [0,1], same resolution as depth
/// `depth`: raw MiDaS disparity map [0,1]
/// `w`, `h`: image dimensions
/// `radius`: filter window half-size (default: 8 → 17×17 window)
/// `epsilon`: regularization (default: 0.01)
///
/// Returns: refined depth map with edges aligned to RGB boundaries
pub fn guided_filter_depth(
    guide: &[f32],
    depth: &[f32],
    w: usize,
    h: usize,
    radius: usize,
    epsilon: f32,
) -> Vec<f32> {
    assert_eq!(guide.len(), w * h);
    assert_eq!(depth.len(), w * h);

    // Step 1: Compute local means using box filter
    let mean_i = box_filter(guide, w, h, radius);
    let mean_p = box_filter(depth, w, h, radius);

    // Step 2: Compute local covariance and variance
    // cov_ip = mean(I * p) - mean_I * mean_p
    // var_i  = mean(I * I) - mean_I * mean_I

    let ip_product: Vec<f32> = guide.iter().zip(depth.iter()).map(|(i, p)| i * p).collect();
    let ii_product: Vec<f32> = guide.iter().map(|i| i * i).collect();

    let mean_ip = box_filter(&ip_product, w, h, radius);
    let mean_ii = box_filter(&ii_product, w, h, radius);

    // Step 3: Compute linear coefficients a, b
    // a = cov_ip / (var_i + epsilon)
    // b = mean_p - a * mean_i

    let mut a = vec![0.0f32; w * h];
    let mut b = vec![0.0f32; w * h];

    for px in 0..w * h {
        let cov_ip = mean_ip[px] - mean_i[px] * mean_p[px];
        let var_i = mean_ii[px] - mean_i[px] * mean_i[px];
        a[px] = cov_ip / (var_i + epsilon);
        b[px] = mean_p[px] - a[px] * mean_i[px];
    }

    // Step 4: Average a, b over local window
    let mean_a = box_filter(&a, w, h, radius);
    let mean_b = box_filter(&b, w, h, radius);

    // Step 5: Compute output: q = mean_a * I + mean_b
    (0..w * h)
        .map(|px| (mean_a[px] * guide[px] + mean_b[px]).clamp(0.0, 1.0))
        .collect()
}

/// Integral image-based box filter for fast local mean computation.
fn box_filter(src: &[f32], w: usize, h: usize, radius: usize) -> Vec<f32> {
    // Build integral image
    let mut integral = vec![0.0f64; (w + 1) * (h + 1)];

    for y in 1..=h {
        for x in 1..=w {
            integral[y * (w + 1) + x] = src[(y - 1) * w + (x - 1)] as f64
                + integral[(y - 1) * (w + 1) + x]
                + integral[y * (w + 1) + (x - 1)]
                - integral[(y - 1) * (w + 1) + (x - 1)];
        }
    }

    // Query box sums
    let mut output = vec![0.0f32; w * h];

    for y in 0..h {
        for x in 0..w {
            let x1 = x.saturating_sub(radius);
            let y1 = y.saturating_sub(radius);
            let x2 = (x + radius + 1).min(w);
            let y2 = (y + radius + 1).min(h);

            let count = ((x2 - x1) * (y2 - y1)) as f64;
            if count < 1.0 {
                output[y * w + x] = src[y * w + x];
                continue;
            }

            let sum = integral[y2 * (w + 1) + x2]
                - integral[y1 * (w + 1) + x2]
                - integral[y2 * (w + 1) + x1]
                + integral[y1 * (w + 1) + x1];

            output[y * w + x] = (sum / count) as f32;
        }
    }

    output
}

// ── Focus plane detection ─────────────────────────────────────────────────────

/// Detect the in-focus distance threshold from the depth map.
///
/// If a face region is provided, use the median disparity within the face
/// bounding box as the focus distance. Otherwise, use the top 30% threshold.
pub fn detect_focus_threshold(depth: &[f32], w: usize, face: Option<&FaceRegion>) -> f32 {
    match face {
        Some(f) => {
            // Collect disparity values within face bbox
            let mut face_depths: Vec<f32> = (f.y..f.y + f.h)
                .flat_map(|py| (f.x..f.x + f.w).map(move |px| depth[py * w + px]))
                .collect();

            if face_depths.is_empty() {
                return 0.7;
            }

            face_depths.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let median = face_depths[face_depths.len() / 2];

            // In-focus: median ± 15%
            (median - 0.15).clamp(0.0, 1.0)
        }
        None => {
            // Default: top 30% of disparity = in-focus
            let mut sorted = depth.to_vec();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let threshold_idx = (sorted.len() as f32 * 0.70) as usize;
            sorted.get(threshold_idx).copied().unwrap_or(0.7)
        }
    }
}

// ── Main refinement function ──────────────────────────────────────────────────

/// Refine a raw MiDaS depth map using the RGB image as a guide.
///
/// Returns the refined depth map with sharper edges.
pub fn refine_depth(
    rgb_frame: &[f32],
    raw_depth: &[f32],
    w: usize,
    h: usize,
    face: Option<&FaceRegion>,
) -> Vec<f32> {
    // Convert RGB to luma for guide
    let luma: Vec<f32> = rgb_frame
        .chunks_exact(3)
        .map(|p| 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2])
        .collect();

    // Apply guided filter: radius=8, epsilon=0.01
    guided_filter_depth(&luma, raw_depth, w, h, 8, 0.01)
}
