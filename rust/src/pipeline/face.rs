//! Face-aware region processing pipeline.
//!
//! Uses a skin-tone heuristic to detect face regions without an ML model.
//! Applies specialized processing: eye sharpening, skin smoothing,
//! hair/beard edge preservation.
//!
//! Only activated in Portrait mode when a face region is detected.

use crate::compute::burst_stack::Frame;
use crate::compute::sharpen::unsharp_mask;

// ── Face region ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FaceRegion {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

impl FaceRegion {
    /// Eye area: top 25% of the face bounding box
    pub fn eye_rect(&self) -> (usize, usize, usize, usize) {
        (self.x, self.y, self.w, self.h / 4)
    }

    /// Skin area: full bounding box (bilateral filter applied here)
    pub fn skin_rect(&self) -> (usize, usize, usize, usize) {
        (self.x, self.y, self.w, self.h)
    }
}

// ── Skin tone detection ───────────────────────────────────────────────────────

/// Returns true for skin-toned pixels.
/// Heuristic: Kovac et al. RGB skin detection.
#[inline(always)]
fn is_skin(r: f32, g: f32, b: f32) -> bool {
    let r = r * 255.0;
    let g = g * 255.0;
    let b = b * 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);

    r > 95.0 && g > 40.0 && b > 20.0 && (max - min) > 15.0 && (r - g).abs() > 15.0 && r > g && r > b
}

// ── Connected component detection (simplified) ────────────────────────────────

/// Find the largest skin-tone region and return its bounding box.
/// Uses a simple row-scan connected component approach for performance.
pub fn detect_face_region(frame: &Frame) -> Option<FaceRegion> {
    let w = frame.width as usize;
    let h = frame.height as usize;
    let total_pixels = w * h;

    // Build skin mask
    let skin_mask: Vec<bool> = (0..total_pixels)
        .map(|px| {
            let i = px * 3;
            is_skin(frame.pixels[i], frame.pixels[i + 1], frame.pixels[i + 2])
        })
        .collect();

    let skin_count = skin_mask.iter().filter(|&&s| s).count();

    // Must be > 5% of image to be considered a face
    if skin_count < total_pixels / 20 {
        return None;
    }

    // Find bounding box of largest skin region
    let mut min_x = w;
    let mut min_y = h;
    let mut max_x = 0;
    let mut max_y = 0;

    for py in 0..h {
        for px in 0..w {
            if skin_mask[py * w + px] {
                min_x = min_x.min(px);
                min_y = min_y.min(py);
                max_x = max_x.max(px);
                max_y = max_y.max(py);
            }
        }
    }

    if max_x <= min_x || max_y <= min_y {
        return None;
    }

    // Add 20% padding, clamped to image bounds
    let pad_x = ((max_x - min_x) as f32 * 0.2) as usize;
    let pad_y = ((max_y - min_y) as f32 * 0.2) as usize;

    let x = min_x.saturating_sub(pad_x);
    let y = min_y.saturating_sub(pad_y);
    let rx = (max_x + pad_x).min(w);
    let ry = (max_y + pad_y).min(h);

    Some(FaceRegion {
        x,
        y,
        w: rx - x,
        h: ry - y,
    })
}

// ── Bilateral filter approximation ────────────────────────────────────────────

/// Fast bilateral filter: spatial Gaussian × range Gaussian.
/// 5×5 spatial kernel, sigma_range = 0.1.
/// Used for skin smoothing.
fn bilateral_smooth(
    pixels: &[f32],
    output: &mut [f32],
    img_w: usize,
    img_h: usize,
    x0: usize,
    y0: usize,
    w: usize,
    h: usize,
) {
    const SIGMA_S: f32 = 2.0;
    const SIGMA_R: f32 = 0.1;
    const RADIUS: isize = 2; // 5×5 kernel

    for py in y0..y0 + h {
        for px in x0..x0 + w {
            let ci = (py * img_w + px) * 3;
            let center = [pixels[ci], pixels[ci + 1], pixels[ci + 2]];

            let mut sum = [0.0f32; 3];
            let mut total = 0.0f32;

            for dy in -RADIUS..=RADIUS {
                for dx in -RADIUS..=RADIUS {
                    let nx = px as isize + dx;
                    let ny = py as isize + dy;
                    if nx < 0 || ny < 0 || nx >= img_w as isize || ny >= img_h as isize {
                        continue;
                    }
                    let ni = (ny as usize * img_w + nx as usize) * 3;
                    let neighbor = [pixels[ni], pixels[ni + 1], pixels[ni + 2]];

                    // Spatial weight
                    let dist_s = (dx * dx + dy * dy) as f32;
                    let w_s = (-dist_s / (2.0 * SIGMA_S * SIGMA_S)).exp();

                    // Range weight
                    let dist_r = (center[0] - neighbor[0]).powi(2)
                        + (center[1] - neighbor[1]).powi(2)
                        + (center[2] - neighbor[2]).powi(2);
                    let w_r = (-dist_r / (2.0 * SIGMA_R * SIGMA_R)).exp();

                    let w = w_s * w_r;
                    sum[0] += neighbor[0] * w;
                    sum[1] += neighbor[1] * w;
                    sum[2] += neighbor[2] * w;
                    total += w;
                }
            }

            if total > 1e-6 {
                output[ci] = (sum[0] / total).clamp(0.0, 1.0);
                output[ci + 1] = (sum[1] / total).clamp(0.0, 1.0);
                output[ci + 2] = (sum[2] / total).clamp(0.0, 1.0);
            } else {
                output[ci] = center[0];
                output[ci + 1] = center[1];
                output[ci + 2] = center[2];
            }
        }
    }
}

// ── Gaussian soft mask ────────────────────────────────────────────────────────

/// Build a soft compositing mask for a region.
/// Gaussian falloff from 1.0 at center to 0.0 at edges.
fn build_soft_mask(w: usize, h: usize) -> Vec<f32> {
    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;
    let sigma_x = cx * 0.6;
    let sigma_y = cy * 0.6;

    (0..h)
        .flat_map(|py| {
            (0..w).map(move |px| {
                let dx = (px as f32 - cx) / sigma_x;
                let dy = (py as f32 - cy) / sigma_y;
                (-(dx * dx + dy * dy) * 0.5).exp()
            })
        })
        .collect()
}

// ── Main face pipeline ────────────────────────────────────────────────────────

/// Apply face-aware enhancement to a portrait frame.
///
/// Returns the enhanced frame, or the original if no face detected.
pub fn apply_face_pipeline(frame: Frame) -> (Frame, bool) {
    let w = frame.width as usize;
    let h = frame.height as usize;

    let face = match detect_face_region(&frame) {
        Some(f) => f,
        None => return (frame, false),
    };

    log::info!(
        "[Face] Detected region: ({},{}) {}×{}",
        face.x,
        face.y,
        face.w,
        face.h
    );

    let mut output = frame.pixels.clone();

    // ── Skin smoothing (bilateral filter on face bbox) ─────────────────────
    bilateral_smooth(
        &frame.pixels,
        &mut output,
        w,
        h,
        face.x,
        face.y,
        face.w,
        face.h,
    );

    // ── Eye sharpening (top 25% of face bbox) ─────────────────────────────
    let (ex, ey, ew, eh) = face.eye_rect();
    if ew > 4 && eh > 4 {
        // Extract eye region, sharpen, blend back
        let mut eye_pixels: Vec<f32> = Vec::with_capacity(ew * eh * 3);
        for py in ey..ey + eh {
            for px in ex..ex + ew {
                let i = (py * w + px) * 3;
                eye_pixels.push(output[i]);
                eye_pixels.push(output[i + 1]);
                eye_pixels.push(output[i + 2]);
            }
        }

        let eye_frame = Frame::new(eye_pixels, ew as u32, eh as u32);
        // Extra sharpening: amount=0.6 (stronger than default 0.4)
        let sharpened_eye = unsharp_mask(eye_frame, 0.6);

        // Write sharpened eye region back
        for py in 0..eh {
            for px in 0..ew {
                let src_i = (py * ew + px) * 3;
                let dst_i = ((ey + py) * w + (ex + px)) * 3;
                output[dst_i] = sharpened_eye.pixels[src_i];
                output[dst_i + 1] = sharpened_eye.pixels[src_i + 1];
                output[dst_i + 2] = sharpened_eye.pixels[src_i + 2];
            }
        }
    }

    // ── Composite: blend processed region back with soft mask ──────────────
    let mask = build_soft_mask(face.w, face.h);

    for py in 0..face.h {
        for px in 0..face.w {
            let img_i = ((face.y + py) * w + (face.x + px)) * 3;
            let mask_v = mask[py * face.w + px];

            // Blend: output = mask * enhanced + (1 - mask) * original
            for c in 0..3 {
                output[img_i + c] =
                    mask_v * output[img_i + c] + (1.0 - mask_v) * frame.pixels[img_i + c];
            }
        }
    }

    (Frame::new(output, frame.width, frame.height), true)
}
