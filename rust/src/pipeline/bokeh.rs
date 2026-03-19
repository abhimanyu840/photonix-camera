//! Portrait bokeh: variable Gaussian blur driven by a MiDaS depth map.
//!
//! Algorithm:
//!   1. Determine focus plane: pixels with disparity >= threshold are in-focus
//!   2. For each pixel: blur_sigma = max_blur * (1 - disparity / max_disparity)
//!   3. Apply per-pixel variable Gaussian (approximated by 3 discrete passes)
//!
//! This is a simplified but visually convincing bokeh for mobile.

use crate::compute::burst_stack::Frame;

/// Apply portrait bokeh effect.
///
/// `disparity_map`: single-channel f32, same resolution as `frame`.
///    Values near 1.0 = close to camera (in focus).
///    Values near 0.0 = far from camera (blurred).
/// `focus_threshold`: disparity value above which pixels are sharp (default 0.7)
/// `max_blur_radius`: maximum blur radius in pixels for the most-distant pixels
pub fn apply_bokeh(
    frame: Frame,
    disparity_map: &[f32],
    focus_threshold: f32,
    max_blur_radius: f32,
) -> Frame {
    let w = frame.width as usize;
    let h = frame.height as usize;
    assert_eq!(
        disparity_map.len(),
        w * h,
        "Disparity map must match frame dimensions"
    );

    // Build per-pixel blur radius map
    // In-focus region (disparity >= threshold): radius = 0
    // Background (disparity < threshold): radius proportional to distance from threshold
    let blur_radii: Vec<f32> = disparity_map
        .iter()
        .map(|&d| {
            if d >= focus_threshold {
                0.0 // in focus
            } else {
                // Scale blur from 0 at threshold to max_blur_radius at disparity=0
                let t = 1.0 - (d / focus_threshold);
                max_blur_radius * t * t // quadratic falloff looks more natural
            }
        })
        .collect();

    // Quantize blur radii into discrete levels for efficiency
    // Instead of per-pixel convolution, apply 3 passes with increasing radii
    // and blend based on the pixel's target blur level
    let max_r = max_blur_radius.ceil() as usize;
    if max_r == 0 {
        return frame;
    }

    // Pre-compute blurred versions at 3 radius levels
    let r1 = (max_r / 3).max(1);
    let r2 = (max_r * 2 / 3).max(2);
    let r3 = max_r.max(3);

    let blurred1 = box_blur(&frame.pixels, w, h, r1);
    let blurred2 = box_blur(&frame.pixels, w, h, r2);
    let blurred3 = box_blur(&frame.pixels, w, h, r3);

    // Blend: each pixel picks the blur level matching its disparity
    let output: Vec<f32> = (0..w * h)
        .flat_map(|px| {
            let target_r = blur_radii[px];
            let t = (target_r / max_blur_radius).clamp(0.0, 1.0);

            let result = if t < 0.33 {
                // Blend between sharp and blurred1
                let blend = t / 0.33;
                lerp_pixel(&frame.pixels, &blurred1, px, blend)
            } else if t < 0.66 {
                // Blend between blurred1 and blurred2
                let blend = (t - 0.33) / 0.33;
                lerp_pixel(&blurred1, &blurred2, px, blend)
            } else {
                // Blend between blurred2 and blurred3
                let blend = (t - 0.66) / 0.34;
                lerp_pixel(&blurred2, &blurred3, px, blend)
            };

            result
        })
        .collect();

    Frame::new(output, frame.width, frame.height)
}

/// Linear interpolation between two pixel values at index `px`.
fn lerp_pixel(a: &[f32], b: &[f32], px: usize, t: f32) -> [f32; 3] {
    let i = px * 3;
    [
        (a[i] * (1.0 - t) + b[i] * t).clamp(0.0, 1.0),
        (a[i + 1] * (1.0 - t) + b[i + 1] * t).clamp(0.0, 1.0),
        (a[i + 2] * (1.0 - t) + b[i + 2] * t).clamp(0.0, 1.0),
    ]
}

/// Simple box blur — fast approximation of Gaussian.
/// Applied 3 times horizontally and vertically for a Gaussian-like result.
fn box_blur(pixels: &[f32], w: usize, h: usize, radius: usize) -> Vec<f32> {
    let mut result = pixels.to_vec();
    // Three passes of box blur approximates Gaussian
    for _ in 0..3 {
        result = blur_horizontal(&result, w, h, radius);
        result = blur_vertical(&result, w, h, radius);
    }
    result
}

fn blur_horizontal(pixels: &[f32], w: usize, h: usize, radius: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; w * h * 3];
    let r = radius as isize;

    for y in 0..h {
        for x in 0..w {
            let mut sum = [0.0f32; 3];
            let mut count = 0i32;
            for dx in -r..=r {
                let nx = x as isize + dx;
                if nx >= 0 && nx < w as isize {
                    let i = (y * w + nx as usize) * 3;
                    sum[0] += pixels[i];
                    sum[1] += pixels[i + 1];
                    sum[2] += pixels[i + 2];
                    count += 1;
                }
            }
            let i = (y * w + x) * 3;
            let f = 1.0 / count as f32;
            out[i] = sum[0] * f;
            out[i + 1] = sum[1] * f;
            out[i + 2] = sum[2] * f;
        }
    }
    out
}

fn blur_vertical(pixels: &[f32], w: usize, h: usize, radius: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; w * h * 3];
    let r = radius as isize;

    for y in 0..h {
        for x in 0..w {
            let mut sum = [0.0f32; 3];
            let mut count = 0i32;
            for dy in -r..=r {
                let ny = y as isize + dy;
                if ny >= 0 && ny < h as isize {
                    let i = (ny as usize * w + x) * 3;
                    sum[0] += pixels[i];
                    sum[1] += pixels[i + 1];
                    sum[2] += pixels[i + 2];
                    count += 1;
                }
            }
            let i = (y * w + x) * 3;
            let f = 1.0 / count as f32;
            out[i] = sum[0] * f;
            out[i + 1] = sum[1] * f;
            out[i + 2] = sum[2] * f;
        }
    }
    out
}
