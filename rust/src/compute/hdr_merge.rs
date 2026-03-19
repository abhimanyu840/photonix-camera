//! Mertens Exposure Fusion for HDR-like output from bracketed exposures.
//!
//! No tone mapping needed — Laplacian pyramid blending produces
//! a well-exposed LDR image directly from multiple exposures.
//!
//! Three weight maps per frame:
//!   W_contrast  = Laplacian magnitude (local detail)
//!   W_saturation = per-pixel RGB std dev (colour richness)
//!   W_exposure  = Gaussian centred at 0.5 (well-exposedness)

// use rayon::prelude::*;

use crate::compute::burst_stack::Frame;

/// Compute contrast weight map using Laplacian of luma.
fn contrast_weights(luma: &[f32], width: u32, height: u32) -> Vec<f32> {
    let w = width as usize;
    let h = height as usize;
    let mut weights = vec![0.0f32; w * h];

    for y in 1..(h - 1) {
        for x in 1..(w - 1) {
            let idx = y * w + x;
            let lap = (-luma[idx - w - 1] - luma[idx - w] - luma[idx - w + 1] - luma[idx - 1]
                + 8.0 * luma[idx]
                - luma[idx + 1]
                - luma[idx + w - 1]
                - luma[idx + w]
                - luma[idx + w + 1])
                .abs();
            weights[idx] = lap;
        }
    }
    weights
}

/// Compute saturation weight map: std dev of RGB channels per pixel.
fn saturation_weights(pixels: &[f32]) -> Vec<f32> {
    pixels
        .chunks_exact(3)
        .map(|p| {
            let mean = (p[0] + p[1] + p[2]) / 3.0;
            let var = ((p[0] - mean).powi(2) + (p[1] - mean).powi(2) + (p[2] - mean).powi(2)) / 3.0;
            var.sqrt()
        })
        .collect()
}

/// Compute exposure weight map: Gaussian centred at 0.5.
/// Well-exposed pixels (near middle grey) get highest weight.
fn exposure_weights(luma: &[f32]) -> Vec<f32> {
    let sigma = 0.2f32;
    luma.iter()
        .map(|&l| (-(l - 0.5).powi(2) / (2.0 * sigma.powi(2))).exp())
        .collect()
}

/// Normalise per-pixel weights across all frames so they sum to 1.
fn normalise_weights(weight_maps: Vec<Vec<f32>>) -> Vec<Vec<f32>> {
    let n_pixels = weight_maps[0].len();
    let mut normalised: Vec<Vec<f32>> = vec![vec![0.0; n_pixels]; weight_maps.len()];

    for px in 0..n_pixels {
        let sum: f32 = weight_maps.iter().map(|w| w[px]).sum();
        let sum = if sum < 1e-6 { 1.0 } else { sum };
        for (fi, w) in weight_maps.iter().enumerate() {
            normalised[fi][px] = w[px] / sum;
        }
    }

    normalised
}

/// Build a Gaussian pyramid of `levels` levels.
fn gaussian_pyramid(
    data: &[f32],
    width: u32,
    height: u32,
    levels: usize,
) -> Vec<(Vec<f32>, u32, u32)> {
    let mut pyramid = vec![(data.to_vec(), width, height)];

    for _ in 1..levels {
        let (prev, pw, ph) = pyramid.last().unwrap();
        if *pw <= 4 || *ph <= 4 {
            break;
        }
        let (nw, nh) = (pw / 2, ph / 2);
        let mut level = vec![0.0f32; (nw * nh) as usize];

        for y in 0..nh as usize {
            for x in 0..nw as usize {
                // 2×2 box average for Gaussian approximation
                let idx00 = (y * 2) * *pw as usize + x * 2;
                let idx10 = idx00 + 1;
                let idx01 = idx00 + *pw as usize;
                let idx11 = idx01 + 1;
                level[y * nw as usize + x] =
                    (prev[idx00] + prev[idx10] + prev[idx01] + prev[idx11]) * 0.25;
            }
        }
        pyramid.push((level, nw, nh));
    }

    pyramid
}

/// Upsample a level to (target_w × target_h) using bilinear interpolation.
fn upsample(data: &[f32], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Vec<f32> {
    let mut out = vec![0.0f32; (dst_w * dst_h) as usize];
    let sx = src_w as f32 / dst_w as f32;
    let sy = src_h as f32 / dst_h as f32;

    for y in 0..dst_h as usize {
        for x in 0..dst_w as usize {
            let fx = (x as f32 + 0.5) * sx - 0.5;
            let fy = (y as f32 + 0.5) * sy - 0.5;
            let x0 = (fx.floor() as usize).min(src_w as usize - 1);
            let y0 = (fy.floor() as usize).min(src_h as usize - 1);
            let x1 = (x0 + 1).min(src_w as usize - 1);
            let y1 = (y0 + 1).min(src_h as usize - 1);
            let ax = fx - fx.floor();
            let ay = fy - fy.floor();
            let sw = src_w as usize;

            out[y * dst_w as usize + x] = data[y0 * sw + x0] * (1.0 - ax) * (1.0 - ay)
                + data[y0 * sw + x1] * ax * (1.0 - ay)
                + data[y1 * sw + x0] * (1.0 - ax) * ay
                + data[y1 * sw + x1] * ax * ay;
        }
    }
    out
}

/// Mertens exposure fusion of bracketed frames.
/// Returns a single HDR-like f32 frame.
pub fn mertens_fusion(frames: &[Frame]) -> Frame {
    assert!(!frames.is_empty(), "Need at least 1 frame for fusion");

    let w = frames[0].width;
    let h = frames[0].height;
    let levels = 5usize;

    // Compute and normalise weight maps
    let raw_weights: Vec<Vec<f32>> = frames
        .iter()
        .map(|f| {
            let luma: Vec<f32> = f
                .pixels
                .chunks_exact(3)
                .map(|p| 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2])
                .collect();
            let wc = contrast_weights(&luma, w, h);
            let ws = saturation_weights(&f.pixels);
            let we = exposure_weights(&luma);

            // Combine: product of all three weight maps
            wc.iter()
                .zip(ws.iter())
                .zip(we.iter())
                .map(|((c, s), e)| c * s * e + 1e-6)
                .collect()
        })
        .collect();

    let norm_weights = normalise_weights(raw_weights);

    // Build pyramids for each channel and weight map, blend, reconstruct
    let mut output = vec![0.0f32; (w * h * 3) as usize];

    for c in 0..3usize {
        // Extract single channel from each frame
        let channels: Vec<Vec<f32>> = frames
            .iter()
            .map(|f| f.pixels.iter().skip(c).step_by(3).copied().collect())
            .collect();

        // Build Laplacian pyramids for each frame's channel
        let mut result_pyramid: Vec<(Vec<f32>, u32, u32)> =
            gaussian_pyramid(&vec![0.0f32; (w * h) as usize], w, h, levels);

        for (fi, channel) in channels.iter().enumerate() {
            let gauss_pyr = gaussian_pyramid(channel, w, h, levels);
            let weight_pyr = gaussian_pyramid(&norm_weights[fi], w, h, levels);

            // Laplacian pyramid of the channel
            for lev in 0..result_pyramid.len() {
                let (ref mut res, rw, rh) = result_pyramid[lev];
                let (ref gdata, gw, gh) = gauss_pyr[lev];
                let (ref wdata, _, _) = weight_pyr[lev];

                // Laplacian = current - upsampled(next)
                let laplacian = if lev + 1 < gauss_pyr.len() {
                    let (ref gnext, gnw, gnh) = gauss_pyr[lev + 1];
                    let up = upsample(gnext, gnw, gnh, gw, gh);
                    gdata.iter().zip(up.iter()).map(|(g, u)| g - u).collect()
                } else {
                    gdata.clone() // coarsest level
                };

                for px in 0..(rw * rh) as usize {
                    res[px] += wdata[px] * laplacian[px];
                }
            }
        }

        // Reconstruct from pyramid (collapse)
        let mut reconstructed = result_pyramid.last().unwrap().0.clone();
        for lev in (0..result_pyramid.len() - 1).rev() {
            let (_, lw, lh) = result_pyramid[lev];
            let up = upsample(
                &reconstructed,
                result_pyramid[lev + 1].1,
                result_pyramid[lev + 1].2,
                lw,
                lh,
            );
            reconstructed = result_pyramid[lev]
                .0
                .iter()
                .zip(up.iter())
                .map(|(l, u)| (l + u).clamp(0.0, 1.0))
                .collect();
        }

        // Write back to output
        for px in 0..(w * h) as usize {
            output[px * 3 + c] = reconstructed[px];
        }
    }

    Frame::new(output, w, h)
}
