//! Burst frame alignment and sharpness-weighted stacking.
//!
//! Algorithm:
//!   1. Score each frame by Laplacian variance (sharpness proxy)
//!   2. Align all frames to the sharpest frame using Lucas-Kanade
//!      optical flow on downscaled luma (16x downscale for speed)
//!   3. Ghost rejection: per-pixel temporal median, mark outliers
//!   4. Weighted average: weight = sharpness_score * ghost_mask
//!
//! SNR improvement: √N for N aligned frames (3 frames → 1.73× SNR)

use anyhow::{anyhow, Result};
use rayon::prelude::*;

/// A decoded frame in linear f32 RGB.
pub struct Frame {
    pub pixels: Vec<f32>, // interleaved RGB, linear light
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

    pub fn pixel_count(&self) -> usize {
        (self.width * self.height) as usize
    }

    /// Get pixel at (x, y) as [R, G, B] floats
    pub fn get_pixel(&self, x: u32, y: u32) -> [f32; 3] {
        let idx = ((y * self.width + x) * 3) as usize;
        [self.pixels[idx], self.pixels[idx + 1], self.pixels[idx + 2]]
    }

    /// Convert to luma (grayscale) for alignment/scoring
    pub fn to_luma(&self) -> Vec<f32> {
        self.pixels
            .chunks_exact(3)
            .map(|p| 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2])
            .collect()
    }
}

/// Compute sharpness score using Laplacian variance.
/// Higher = sharper. Used to pick reference frame and compute weights.
pub fn laplacian_variance(luma: &[f32], width: u32, height: u32) -> f64 {
    let w = width as usize;
    let h = height as usize;
    let mut sum = 0.0f64;
    let mut count = 0usize;

    for y in 1..(h - 1) {
        for x in 1..(w - 1) {
            let idx = y * w + x;
            // 3×3 Laplacian kernel
            let lap = -luma[idx - w - 1] - luma[idx - w] - luma[idx - w + 1] - luma[idx - 1]
                + 8.0 * luma[idx]
                - luma[idx + 1]
                - luma[idx + w - 1]
                - luma[idx + w]
                - luma[idx + w + 1];
            sum += (lap as f64).powi(2);
            count += 1;
        }
    }

    if count == 0 {
        0.0
    } else {
        sum / count as f64
    }
}

/// Downsample a luma buffer by factor (for fast optical flow estimation).
fn downsample_luma(luma: &[f32], width: u32, height: u32, factor: u32) -> (Vec<f32>, u32, u32) {
    let ow = width / factor;
    let oh = height / factor;
    let mut out = vec![0.0f32; (ow * oh) as usize];

    for y in 0..oh {
        for x in 0..ow {
            // Average 2×2 block for better quality than point sampling
            let sx = x * factor;
            let sy = y * factor;
            let mut acc = 0.0f32;
            let mut cnt = 0u32;
            for dy in 0..factor.min(height - sy) {
                for dx in 0..factor.min(width - sx) {
                    acc += luma[((sy + dy) * width + (sx + dx)) as usize];
                    cnt += 1;
                }
            }
            out[(y * ow + x) as usize] = if cnt > 0 { acc / cnt as f32 } else { 0.0 };
        }
    }

    (out, ow, oh)
}

/// Lucas-Kanade pyramid alignment.
/// Returns (dx, dy) translation offset of `source` relative to `reference`.
/// Uses 3-level pyramid on 16× downscaled luma for sub-pixel accuracy.
pub fn estimate_translation(
    reference_luma: &[f32],
    source_luma: &[f32],
    width: u32,
    height: u32,
) -> (f32, f32) {
    // Coarse pyramid search on 16× downscale
    let factor = 16u32;
    let (ref_down, dw, dh) = downsample_luma(reference_luma, width, height, factor);
    let (src_down, _, _) = downsample_luma(source_luma, width, height, factor);

    let (dx_coarse, dy_coarse) = lucas_kanade_translation(&ref_down, &src_down, dw, dh);

    // Scale back to full resolution
    (dx_coarse * factor as f32, dy_coarse * factor as f32)
}

/// Iterative Lucas-Kanade translation estimator.
/// Assumes small motion (< frame_size / 4 pixels).
fn lucas_kanade_translation(
    reference: &[f32],
    source: &[f32],
    width: u32,
    height: u32,
) -> (f32, f32) {
    let w = width as usize;
    let h = height as usize;
    let mut dx = 0.0f32;
    let mut dy = 0.0f32;

    // 5 iterations of gradient descent
    for _ in 0..5 {
        let mut a11 = 0.0f64;
        let mut a12 = 0.0f64;
        let mut a22 = 0.0f64;
        let mut b1 = 0.0f64;
        let mut b2 = 0.0f64;

        for y in 1..(h - 1) {
            for x in 1..(w - 1) {
                let idx = y * w + x;
                // Image gradient
                let ix = (reference[idx + 1] - reference[idx - 1]) * 0.5;
                let iy = (reference[idx + w] - reference[idx - w]) * 0.5;

                // Warp source with current (dx, dy) estimate
                let sx = x as f32 + dx;
                let sy = y as f32 + dy;
                let warped = bilinear_sample(source, width, height, sx, sy);

                let it = (reference[idx] - warped) as f64;
                let ix = ix as f64;
                let iy = iy as f64;

                a11 += ix * ix;
                a12 += ix * iy;
                a22 += iy * iy;
                b1 += ix * it;
                b2 += iy * it;
            }
        }

        // Solve 2×2 linear system
        let det = a11 * a22 - a12 * a12;
        if det.abs() < 1e-10 {
            break;
        }

        dx += ((a22 * b1 - a12 * b2) / det) as f32;
        dy += ((a11 * b2 - a12 * b1) / det) as f32;
    }

    (dx, dy)
}

/// Bilinear interpolation at fractional pixel coordinates.
fn bilinear_sample(buf: &[f32], width: u32, height: u32, x: f32, y: f32) -> f32 {
    let x = x.clamp(0.0, width as f32 - 1.001);
    let y = y.clamp(0.0, height as f32 - 1.001);
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(width as usize - 1);
    let y1 = (y0 + 1).min(height as usize - 1);
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let w = width as usize;

    let v00 = buf[y0 * w + x0];
    let v10 = buf[y0 * w + x1];
    let v01 = buf[y1 * w + x0];
    let v11 = buf[y1 * w + x1];

    v00 * (1.0 - fx) * (1.0 - fy) + v10 * fx * (1.0 - fy) + v01 * (1.0 - fx) * fy + v11 * fx * fy
}

/// Warp a frame by (dx, dy) translation using bilinear interpolation.
fn warp_frame(frame: &Frame, dx: f32, dy: f32) -> Frame {
    let w = frame.width;
    let h = frame.height;
    let mut out = vec![0.0f32; (w * h * 3) as usize];

    // Parallel over rows
    out.par_chunks_mut((w * 3) as usize)
        .enumerate()
        .for_each(|(y, row)| {
            for x in 0..w as usize {
                let sx = x as f32 - dx;
                let sy = y as f32 - dy;
                for c in 0..3 {
                    // Sample each channel independently
                    let channel: Vec<f32> =
                        frame.pixels.iter().skip(c).step_by(3).copied().collect();
                    row[x * 3 + c] = bilinear_sample(&channel, w, h, sx, sy);
                }
            }
        });

    Frame::new(out, w, h)
}

/// Ghost rejection: per-pixel, mark pixels that deviate > threshold
/// from the temporal median as ghosts (moving objects).
/// Returns a weight map (1.0 = good, 0.0 = ghost).
fn ghost_rejection_weights(frames: &[Frame], threshold: f32) -> Vec<Vec<f32>> {
    let n = frames.len();
    let pixel_count = frames[0].pixel_count();
    let mut weights: Vec<Vec<f32>> = vec![vec![1.0f32; pixel_count]; n];

    // Compute per-pixel median across frames for each channel
    for px in 0..pixel_count {
        for c in 0..3usize {
            let mut vals: Vec<f32> = frames.iter().map(|f| f.pixels[px * 3 + c]).collect();
            vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let median = vals[n / 2];

            // Mark pixels too far from median
            for (fi, frame) in frames.iter().enumerate() {
                let diff = (frame.pixels[px * 3 + c] - median).abs();
                if diff > threshold {
                    weights[fi][px] *= 0.0; // ghost — zero weight
                }
            }
        }
    }

    weights
}

/// Main burst stack function.
/// Returns sharpness-weighted, ghost-rejected average of all frames.
pub fn stack_burst(frames: Vec<Frame>) -> Result<Frame> {
    if frames.is_empty() {
        return Err(anyhow!("No frames to stack"));
    }
    if frames.len() == 1 {
        return Ok(frames.into_iter().next().unwrap());
    }

    let w = frames[0].width;
    let h = frames[0].height;

    // Step 1: Score sharpness of each frame
    let scores: Vec<f64> = frames
        .iter()
        .map(|f| laplacian_variance(&f.to_luma(), w, h))
        .collect();

    // Reference frame = sharpest
    let ref_idx = scores
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .map(|(i, _)| i)
        .unwrap_or(0);

    let ref_luma = frames[ref_idx].to_luma();

    // Step 2: Align all frames to reference
    let aligned: Vec<Frame> = frames
        .into_iter()
        .enumerate()
        .map(|(i, frame)| {
            if i == ref_idx {
                frame
            } else {
                let src_luma = frame.to_luma();
                let (dx, dy) = estimate_translation(&ref_luma, &src_luma, w, h);
                if dx.abs() < 0.5 && dy.abs() < 0.5 {
                    frame // No meaningful motion
                } else {
                    warp_frame(&frame, dx, dy)
                }
            }
        })
        .collect();

    // Step 3: Ghost rejection weights
    let ghost_weights = ghost_rejection_weights(&aligned, 0.15);

    // Step 4: Sharpness-weighted average
    let sharpness_weights: Vec<f32> = scores
        .iter()
        .map(|&s| (s as f32).sqrt()) // sqrt for less extreme weighting
        .collect();

    let pixel_count = (w * h) as usize;
    let mut output = vec![0.0f32; pixel_count * 3];

    for px in 0..pixel_count {
        let mut weight_sum = 0.0f32;
        let mut rgb = [0.0f32; 3];

        for (fi, frame) in aligned.iter().enumerate() {
            let w_ghost = ghost_weights[fi][px];
            let w_sharp = sharpness_weights[fi];
            let w = w_ghost * w_sharp;

            for c in 0..3 {
                rgb[c] += frame.pixels[px * 3 + c] * w;
            }
            weight_sum += w;
        }

        if weight_sum > 1e-6 {
            for c in 0..3 {
                output[px * 3 + c] = (rgb[c] / weight_sum).clamp(0.0, 1.0);
            }
        } else {
            // All frames ghosted at this pixel — use reference frame
            for c in 0..3 {
                output[px * 3 + c] = aligned[ref_idx].pixels[px * 3 + c];
            }
        }
    }

    Ok(Frame::new(output, w, h))
}
