//! Image sharpening via unsharp mask.
//!
//! Unsharp mask: output = input + amount * (input - gaussian_blur(input))
//! Works on display-encoded (gamma) data — applied AFTER tone mapping.

use crate::compute::burst_stack::Frame;

/// Gaussian blur kernel weights for radius 1 (3×3)
const KERNEL_3X3: [f32; 9] = [
    1.0 / 16.0,
    2.0 / 16.0,
    1.0 / 16.0,
    2.0 / 16.0,
    4.0 / 16.0,
    2.0 / 16.0,
    1.0 / 16.0,
    2.0 / 16.0,
    1.0 / 16.0,
];

/// Apply 3×3 Gaussian blur to a single channel buffer.
fn gaussian_blur_3x3(channel: &[f32], width: u32, height: u32) -> Vec<f32> {
    let w = width as usize;
    let h = height as usize;
    let mut out = channel.to_vec();

    for y in 1..(h - 1) {
        for x in 1..(w - 1) {
            let mut acc = 0.0f32;
            for ky in 0..3usize {
                for kx in 0..3usize {
                    let px = x + kx - 1;
                    let py = y + ky - 1;
                    acc += channel[py * w + px] * KERNEL_3X3[ky * 3 + kx];
                }
            }
            out[y * w + x] = acc;
        }
    }
    out
}

/// Unsharp mask sharpening.
///
/// `amount`: sharpening strength (0.0 = none, 1.0 = strong, typical 0.4–0.8)
/// `radius`: blur radius (1 = 3×3 kernel)
pub fn unsharp_mask(frame: Frame, amount: f32) -> Frame {
    let w = frame.width;
    let h = frame.height;

    let mut output = frame.pixels.clone();

    // Process each channel independently
    for c in 0..3usize {
        let channel: Vec<f32> = frame.pixels.iter().skip(c).step_by(3).copied().collect();
        let blurred = gaussian_blur_3x3(&channel, w, h);

        for px in 0..(w * h) as usize {
            let detail = channel[px] - blurred[px];
            output[px * 3 + c] = (channel[px] + amount * detail).clamp(0.0, 1.0);
        }
    }

    Frame::new(output, w, h)
}
