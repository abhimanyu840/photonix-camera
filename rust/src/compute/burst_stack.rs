//! Motion-aware burst frame stacking.
//!
//! Pipeline:
//!   1. Compute sharpness score per frame (Laplacian variance)
//!   2. Classify motion magnitude (low / medium / high)
//!   3. Select frames based on motion class
//!   4. Align selected frames with LK optical flow
//!   5. Ghost-reject and weighted-merge

use anyhow::Result;
use rayon::prelude::*;

// ── Frame type ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Frame {
    pub pixels: Vec<f32>,
    pub width: u32,
    pub height: u32,
}

impl Frame {
    pub fn new(pixels: Vec<f32>, width: u32, height: u32) -> Self {
        Self {
            pixels,
            width,
            height,
        }
    }

    /// Convert RGB frame to single-channel luma [0,1].
    pub fn to_luma(&self) -> Vec<f32> {
        self.pixels
            .chunks_exact(3)
            .map(|p| 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2])
            .collect()
    }
}

// ── Motion classification ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MotionClass {
    /// Mean displacement < 2px — use all frames, weight by sharpness
    Low,
    /// Mean displacement 2–8px — use 3 sharpest frames
    Medium,
    /// Mean displacement > 8px — use single sharpest frame
    High,
}

impl MotionClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            MotionClass::Low => "low",
            MotionClass::Medium => "medium",
            MotionClass::High => "high",
        }
    }
}

// ── Sharpness scoring ─────────────────────────────────────────────────────────

/// Laplacian variance sharpness score.
/// Higher = sharper. Uses the LoG approximation kernel [-1,-1,-1,-1,8,-1,-1,-1,-1].
pub fn laplacian_variance(luma: &[f32], w: usize, h: usize) -> f32 {
    if luma.len() < w * h || w < 3 || h < 3 {
        return 0.0;
    }

    let mut sum = 0.0f64;
    let mut sum_sq = 0.0f64;
    let mut count = 0u64;

    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let tl = luma[(y - 1) * w + (x - 1)] as f64;
            let t = luma[(y - 1) * w + x] as f64;
            let tr = luma[(y - 1) * w + (x + 1)] as f64;
            let l = luma[y * w + (x - 1)] as f64;
            let c = luma[y * w + x] as f64;
            let r = luma[y * w + (x + 1)] as f64;
            let bl = luma[(y + 1) * w + (x - 1)] as f64;
            let b = luma[(y + 1) * w + x] as f64;
            let br = luma[(y + 1) * w + (x + 1)] as f64;

            let lap = 8.0 * c - (tl + t + tr + l + r + bl + b + br);
            sum += lap;
            sum_sq += lap * lap;
            count += 1;
        }
    }

    if count == 0 {
        return 0.0;
    }

    let mean = sum / count as f64;
    let variance = sum_sq / count as f64 - mean * mean;
    variance.max(0.0) as f32
}

// ── Optical flow (Lucas-Kanade approximation) ─────────────────────────────────

/// Estimate mean displacement between two luma images using block matching.
/// Returns mean displacement in pixels.
pub fn estimate_motion(prev: &[f32], curr: &[f32], w: usize, h: usize) -> f32 {
    const BLOCK: usize = 16;
    const SEARCH: isize = 8;

    let bw = w / BLOCK;
    let bh = h / BLOCK;

    if bw == 0 || bh == 0 {
        return 0.0;
    }

    let displacements: Vec<f32> = (0..bh)
        .flat_map(|by| {
            (0..bw).map(move |bx| {
                let ox = bx * BLOCK;
                let oy = by * BLOCK;

                let mut best_sad = f32::MAX;
                let mut best_dx = 0isize;
                let mut best_dy = 0isize;

                for dy in -SEARCH..=SEARCH {
                    for dx in -SEARCH..=SEARCH {
                        let mut sad = 0.0f32;
                        let mut valid = true;

                        'sad: for py in 0..BLOCK {
                            for px in 0..BLOCK {
                                let nx = ox as isize + px as isize + dx;
                                let ny = oy as isize + py as isize + dy;
                                if nx < 0 || ny < 0 || nx >= w as isize || ny >= h as isize {
                                    valid = false;
                                    break 'sad;
                                }
                                let prev_v = prev[(oy + py) * w + (ox + px)];
                                let curr_v = curr[ny as usize * w + nx as usize];
                                sad += (prev_v - curr_v).abs();
                            }
                        }

                        if valid && sad < best_sad {
                            best_sad = sad;
                            best_dx = dx;
                            best_dy = dy;
                        }
                    }
                }

                ((best_dx * best_dx + best_dy * best_dy) as f32).sqrt()
            })
        })
        .collect();

    if displacements.is_empty() {
        return 0.0;
    }

    displacements.iter().sum::<f32>() / displacements.len() as f32
}

// ── Frame alignment (translation) ────────────────────────────────────────────

/// Align a frame to the reference using integer-pixel translation.
/// Translation estimated by block matching.
fn align_frame(reference: &[f32], target: &Frame, w: usize, h: usize) -> Frame {
    let ref_luma: Vec<f32> = reference
        .chunks_exact(3)
        .map(|p| 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2])
        .collect();
    let tgt_luma = target.to_luma();

    let (dx, dy) = find_translation(&ref_luma, &tgt_luma, w, h);

    if dx == 0 && dy == 0 {
        return target.clone();
    }

    let mut aligned = vec![0.0f32; w * h * 3];
    for y in 0..h {
        for x in 0..w {
            let sx = x as isize - dx;
            let sy = y as isize - dy;
            if sx >= 0 && sy >= 0 && sx < w as isize && sy < h as isize {
                let si = (sy as usize * w + sx as usize) * 3;
                let di = (y * w + x) * 3;
                aligned[di] = target.pixels[si];
                aligned[di + 1] = target.pixels[si + 1];
                aligned[di + 2] = target.pixels[si + 2];
            }
        }
    }

    Frame::new(aligned, target.width, target.height)
}

/// Find integer translation (dx, dy) via phase correlation on luma.
fn find_translation(prev: &[f32], curr: &[f32], w: usize, h: usize) -> (isize, isize) {
    const SEARCH: isize = 16;
    const BLOCK: usize = 32;

    // Sample from center region for reliability
    let cx = w / 2;
    let cy = h / 2;
    let bx = cx.saturating_sub(BLOCK / 2);
    let by = cy.saturating_sub(BLOCK / 2);

    let mut best_sad = f32::MAX;
    let mut best = (0isize, 0isize);

    for dy in -SEARCH..=SEARCH {
        for dx in -SEARCH..=SEARCH {
            let mut sad = 0.0f32;
            let mut valid = true;

            'outer: for py in 0..BLOCK {
                for px in 0..BLOCK {
                    let nx = bx as isize + px as isize + dx;
                    let ny = by as isize + py as isize + dy;
                    if nx < 0 || ny < 0 || nx >= w as isize || ny >= h as isize {
                        valid = false;
                        break 'outer;
                    }
                    sad += (prev[(by + py) * w + (bx + px)] - curr[ny as usize * w + nx as usize])
                        .abs();
                }
            }

            if valid && sad < best_sad {
                best_sad = sad;
                best = (dx, dy);
            }
        }
    }

    best
}

// ── Ghost rejection ───────────────────────────────────────────────────────────

const GHOST_THRESHOLD: f32 = 0.15;

/// Returns per-pixel weight: 1.0 if close to reference, 0.0 if ghost.
fn ghost_mask(reference: &[f32], frame: &[f32]) -> Vec<f32> {
    reference
        .chunks_exact(3)
        .zip(frame.chunks_exact(3))
        .map(|(r, f)| {
            let diff = (r[0] - f[0]).abs() + (r[1] - f[1]).abs() + (r[2] - f[2]).abs();
            if diff / 3.0 > GHOST_THRESHOLD {
                0.0
            } else {
                1.0
            }
        })
        .collect()
}

// ── Main stack function ───────────────────────────────────────────────────────

pub struct StackResult {
    pub frame: Frame,
    pub motion_class: MotionClass,
    pub sharpness_scores: Vec<f32>,
    pub frames_used: usize,
}

/// Motion-aware burst stack.
///
/// 1. Score each frame by sharpness (Laplacian variance)
/// 2. Estimate motion between first and last frame
/// 3. Select frames and stack strategy based on motion class
pub fn stack_burst(frames: Vec<Frame>) -> Result<Frame> {
    let result = stack_burst_detailed(frames)?;
    log::info!(
        "[BurstStack] motion={} frames_used={}",
        result.motion_class.as_str(),
        result.frames_used
    );
    Ok(result.frame)
}

pub fn stack_burst_detailed(frames: Vec<Frame>) -> Result<StackResult> {
    if frames.is_empty() {
        return Err(anyhow::anyhow!("No frames to stack"));
    }
    if frames.len() == 1 {
        let scores = vec![1.0f32];
        return Ok(StackResult {
            frame: frames.into_iter().next().unwrap(),
            motion_class: MotionClass::Low,
            sharpness_scores: scores,
            frames_used: 1,
        });
    }

    let w = frames[0].width as usize;
    let h = frames[0].height as usize;

    // ── Step 1: Sharpness scores ──────────────────────────────────────────────
    let scores: Vec<f32> = frames
        .par_iter()
        .map(|f| {
            let luma = f.to_luma();
            laplacian_variance(&luma, w, h)
        })
        .collect();

    log::debug!("[BurstStack] sharpness scores: {:?}", scores);

    // ── Step 2: Motion classification ────────────────────────────────────────
    let first_luma = frames[0].to_luma();
    let last_luma = frames[frames.len() - 1].to_luma();
    let mean_disp = estimate_motion(&first_luma, &last_luma, w, h);

    let motion_class = if mean_disp < 2.0 {
        MotionClass::Low
    } else if mean_disp <= 8.0 {
        MotionClass::Medium
    } else {
        MotionClass::High
    };

    log::info!(
        "[BurstStack] mean_displacement={:.1}px → {}",
        mean_disp,
        motion_class.as_str()
    );

    // ── Step 3: Frame selection ───────────────────────────────────────────────
    let selected_frames: Vec<Frame> = match motion_class {
        MotionClass::High => {
            // Single sharpest frame — no stacking
            let best_idx = scores
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .map(|(i, _)| i)
                .unwrap_or(0);
            return Ok(StackResult {
                frame: frames.into_iter().nth(best_idx).unwrap(),
                motion_class,
                sharpness_scores: scores,
                frames_used: 1,
            });
        }
        MotionClass::Medium => {
            // 3 sharpest frames
            let mut indexed: Vec<(usize, f32)> = scores.iter().copied().enumerate().collect();
            indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            let top3: std::collections::HashSet<usize> =
                indexed.iter().take(3).map(|(i, _)| *i).collect();

            frames
                .into_iter()
                .enumerate()
                .filter(|(i, _)| top3.contains(i))
                .map(|(_, f)| f)
                .collect()
        }
        MotionClass::Low => {
            // All frames, but reject below sharpness threshold
            let median = {
                let mut s = scores.clone();
                s.sort_by(|a, b| a.partial_cmp(b).unwrap());
                s[s.len() / 2]
            };
            let threshold = 0.7 * median;

            frames
                .into_iter()
                .enumerate()
                .filter(|(i, _)| scores[*i] >= threshold)
                .map(|(_, f)| f)
                .collect()
        }
    };

    let frames_used = selected_frames.len();

    if selected_frames.is_empty() {
        return Err(anyhow::anyhow!("All frames rejected by sharpness filter"));
    }

    // ── Step 4: Align to reference (first frame) ──────────────────────────────
    let reference = selected_frames[0].clone();
    let aligned: Vec<Frame> = std::iter::once(reference.clone())
        .chain(
            selected_frames
                .into_iter()
                .skip(1)
                .map(|f| align_frame(&reference.pixels, &f, w, h)),
        )
        .collect();

    // ── Step 5: Sharpness-weighted merge with ghost rejection ─────────────────
    let frame_scores: Vec<f32> = {
        let luma0 = aligned[0].to_luma();
        // Recompute sharpness on aligned frames for accurate weighting
        aligned
            .iter()
            .map(|f| {
                let l = f.to_luma();
                laplacian_variance(&l, w, h).max(1e-6)
            })
            .collect()
    };

    let score_sum: f32 = frame_scores.iter().sum();
    let weights: Vec<f32> = frame_scores.iter().map(|s| s / score_sum).collect();

    let ref_pixels = &aligned[0].pixels;
    let ghost_masks: Vec<Vec<f32>> = aligned
        .iter()
        .map(|f| ghost_mask(ref_pixels, &f.pixels))
        .collect();

    let n_pixels = w * h;
    let mut merged = vec![0.0f32; n_pixels * 3];

    for px in 0..n_pixels {
        let mut total_weight = 0.0f32;
        let mut r = 0.0f32;
        let mut g = 0.0f32;
        let mut b = 0.0f32;

        for (fi, frame) in aligned.iter().enumerate() {
            let ghost_w = ghost_masks[fi][px];
            let w_i = weights[fi] * ghost_w;
            let i = px * 3;
            r += frame.pixels[i] * w_i;
            g += frame.pixels[i + 1] * w_i;
            b += frame.pixels[i + 2] * w_i;
            total_weight += w_i;
        }

        if total_weight > 1e-6 {
            let i = px * 3;
            merged[i] = (r / total_weight).clamp(0.0, 1.0);
            merged[i + 1] = (g / total_weight).clamp(0.0, 1.0);
            merged[i + 2] = (b / total_weight).clamp(0.0, 1.0);
        } else {
            // Fallback: use reference pixel
            let i = px * 3;
            merged[i] = ref_pixels[i];
            merged[i + 1] = ref_pixels[i + 1];
            merged[i + 2] = ref_pixels[i + 2];
        }
    }

    Ok(StackResult {
        frame: Frame::new(merged, reference.width, reference.height),
        motion_class,
        sharpness_scores: scores,
        frames_used,
    })
}

// ── Gyroscope integration design ──────────────────────────────────────────────
//
// Design (no implementation — requires Flutter platform channel):
//
// 1. Flutter side: use `sensors_plus` package to read gyroscope events.
//    Pass angular velocity (rad/s) + exposure time (ms) to Rust via bridge.
//
// 2. Rust side: angular_to_pixels(omega_x, omega_y, exposure_ms, focal_px):
//    displacement_x = omega_x * (exposure_ms / 1000.0) * focal_px
//    displacement_y = omega_y * (exposure_ms / 1000.0) * focal_px
//    Where focal_px ≈ image_width / (2 * tan(fov/2)) ≈ 1.1 * image_width
//
// 3. When gyro prediction improves over optical flow:
//    - Scene with large moving subjects (optical flow confused by motion)
//    - Very low light where optical flow has high noise
//    - Burst interval < 50ms (gyro prediction faster than block matching)
//
// 4. Fusion: weight gyro estimate by confidence (low for fast rotation > 2 rad/s)
//    final_disp = (gyro_disp * gyro_conf + of_disp * (1-gyro_conf))
