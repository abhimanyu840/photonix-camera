//! Exposure correction: CLAHE and gamma lift.
//! Applied in linear space before tone mapping.

use crate::compute::burst_stack::Frame;

/// Gamma lift for shadow recovery.
/// Applies a soft knee curve: brightens shadows while preserving highlights.
/// `lift`: 0.0 = no effect, 0.2 = mild recovery, 0.4 = strong
pub fn gamma_lift(frame: Frame, lift: f32) -> Frame {
    let pixels: Vec<f32> = frame
        .pixels
        .iter()
        .map(|&x| {
            // Soft shadow lift using a toe curve
            let lifted = x + lift * (1.0 - x) * (1.0 - x) * x.powf(0.5);
            lifted.clamp(0.0, 1.0)
        })
        .collect();

    Frame::new(pixels, frame.width, frame.height)
}

/// Global histogram equalisation on luma channel.
/// Improves local contrast for low-contrast scenes.
/// Applied before tone mapping in linear space.
pub fn histogram_equalize(frame: Frame) -> Frame {
    let w = frame.width as usize;
    let h = frame.height as usize;
    let n_pixels = w * h;

    // Compute luma histogram (256 bins)
    let mut histogram = [0u32; 256];
    let lumas: Vec<f32> = frame
        .pixels
        .chunks_exact(3)
        .map(|p| (0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2]).clamp(0.0, 1.0))
        .collect();

    for &l in &lumas {
        let bin = (l * 255.0) as usize;
        histogram[bin.min(255)] += 1;
    }

    // Compute CDF
    let mut cdf = [0u32; 256];
    cdf[0] = histogram[0];
    for i in 1..256 {
        cdf[i] = cdf[i - 1] + histogram[i];
    }

    let cdf_min = *cdf.iter().find(|&&v| v > 0).unwrap_or(&1);
    let scale = 255.0 / (n_pixels as f32 - cdf_min as f32).max(1.0);

    // Build LUT
    let lut: Vec<f32> = (0..256)
        .map(|i| ((cdf[i].saturating_sub(cdf_min)) as f32 * scale / 255.0).clamp(0.0, 1.0))
        .collect();

    // Apply equalisation: adjust luma, preserve hue/saturation
    let pixels: Vec<f32> = frame
        .pixels
        .chunks_exact(3)
        .zip(lumas.iter())
        .flat_map(|(p, &l)| {
            let l_orig = l.max(1e-6);
            let l_eq = lut[(l * 255.0) as usize];
            let ratio = l_eq / l_orig;
            // Scale RGB channels proportionally
            let strength = 0.5f32; // blend with original to avoid over-eq
            [
                (p[0] * (1.0 + strength * (ratio - 1.0))).clamp(0.0, 1.0),
                (p[1] * (1.0 + strength * (ratio - 1.0))).clamp(0.0, 1.0),
                (p[2] * (1.0 + strength * (ratio - 1.0))).clamp(0.0, 1.0),
            ]
        })
        .collect();

    Frame::new(pixels, frame.width, frame.height)
}
