//! White balance and color adjustments.
//! Operates on linear f32 data (before tone mapping).

use crate::compute::burst_stack::Frame;

/// White balance mode.
pub enum WhiteBalanceMode {
    /// Grey world assumption: R/G/B mean should all equal overall mean
    GreyWorld,
    /// Manual colour temperature in Kelvin
    Temperature(f32),
    /// No white balance correction
    None,
}

/// Apply grey world white balance.
/// Scales each channel so its mean equals the overall mean.
fn grey_world_wb(pixels: &mut Vec<f32>) {
    let n = (pixels.len() / 3) as f64;
    let (mut sr, mut sg, mut sb) = (0.0f64, 0.0f64, 0.0f64);

    for chunk in pixels.chunks_exact(3) {
        sr += chunk[0] as f64;
        sg += chunk[1] as f64;
        sb += chunk[2] as f64;
    }

    let mean_r = sr / n;
    let mean_g = sg / n;
    let mean_b = sb / n;
    let mean_grey = (mean_r + mean_g + mean_b) / 3.0;

    if mean_r < 1e-6 || mean_g < 1e-6 || mean_b < 1e-6 {
        return;
    }

    let scale_r = (mean_grey / mean_r) as f32;
    let scale_g = (mean_grey / mean_g) as f32;
    let scale_b = (mean_grey / mean_b) as f32;

    for chunk in pixels.chunks_exact_mut(3) {
        chunk[0] = (chunk[0] * scale_r).clamp(0.0, 1.0);
        chunk[1] = (chunk[1] * scale_g).clamp(0.0, 1.0);
        chunk[2] = (chunk[2] * scale_b).clamp(0.0, 1.0);
    }
}

/// Convert colour temperature (Kelvin) to RGB multipliers.
/// Approximation valid for 1000K–40000K.
fn temperature_to_rgb(kelvin: f32) -> (f32, f32, f32) {
    let t = kelvin / 100.0;
    let r = if t <= 66.0 {
        1.0
    } else {
        (329.698_73 * (t - 60.0).powf(-0.133_204_76) / 255.0).clamp(0.0, 1.0)
    };
    let g = if t <= 66.0 {
        (99.470_8 * t.ln() - 161.119_57) / 255.0
    } else {
        (288.122_169_6 * (t - 60.0).powf(-0.075_514_84) / 255.0)
    }
    .clamp(0.0, 1.0);
    let b = if t >= 66.0 {
        1.0
    } else if t <= 19.0 {
        0.0
    } else {
        (138.517_73 * (t - 10.0).ln() - 305.044_793_2) / 255.0
    }
    .clamp(0.0, 1.0);
    (r, g, b)
}

/// Apply white balance correction.
pub fn white_balance(mut frame: Frame, mode: WhiteBalanceMode) -> Frame {
    match mode {
        WhiteBalanceMode::GreyWorld => {
            grey_world_wb(&mut frame.pixels);
        }
        WhiteBalanceMode::Temperature(k) => {
            let (tr, tg, tb) = temperature_to_rgb(k);
            // Normalise so green channel is always 1.0
            let (sr, sg, sb) = (tr / tg, 1.0, tb / tg);
            for chunk in frame.pixels.chunks_exact_mut(3) {
                chunk[0] = (chunk[0] * sr).clamp(0.0, 1.0);
                chunk[1] = (chunk[1] * sg).clamp(0.0, 1.0);
                chunk[2] = (chunk[2] * sb).clamp(0.0, 1.0);
            }
        }
        WhiteBalanceMode::None => {}
    }
    frame
}

/// Adjust saturation in linear RGB space.
/// `factor`: 0.0 = greyscale, 1.0 = unchanged, 1.5 = more vivid
pub fn adjust_saturation(frame: Frame, factor: f32) -> Frame {
    let pixels: Vec<f32> = frame
        .pixels
        .chunks_exact(3)
        .flat_map(|p| {
            let luma = 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2];
            [
                (luma + factor * (p[0] - luma)).clamp(0.0, 1.0),
                (luma + factor * (p[1] - luma)).clamp(0.0, 1.0),
                (luma + factor * (p[2] - luma)).clamp(0.0, 1.0),
            ]
        })
        .collect();

    Frame::new(pixels, frame.width, frame.height)
}
