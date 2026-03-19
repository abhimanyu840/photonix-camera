//! Tone mapping operators.
//!
//! Inputs and outputs are in linear f32 (0.0–1.0+).
//! Apply BEFORE gamma encoding, AFTER all HDR processing.

use crate::compute::burst_stack::Frame;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToneMappingMode {
    /// ACES filmic approximation — cinematic look, rich shadows, controlled highlights
    AcesFilmic,
    /// Reinhard global — simple, mathematically clean, no burn
    Reinhard,
    /// Passthrough — no tone mapping (for testing)
    None,
}

/// ACES filmic tone map approximation by Krzysztof Narkowicz.
/// Input: linear HDR value. Output: LDR [0,1].
#[inline]
fn aces_filmic(x: f32) -> f32 {
    let a = 2.51f32;
    let b = 0.03f32;
    let c = 2.43f32;
    let d = 0.59f32;
    let e = 0.14f32;
    ((x * (a * x + b)) / (x * (c * x + d) + e)).clamp(0.0, 1.0)
}

/// Reinhard global tone map.
/// Input: linear HDR value. Output: LDR [0,1].
#[inline]
fn reinhard(x: f32) -> f32 {
    x / (1.0 + x)
}

/// Apply sRGB gamma encoding (linear → display).
/// Uses the piecewise sRGB transfer function.
#[inline]
pub fn linear_to_srgb(x: f32) -> f32 {
    if x <= 0.0031308 {
        12.92 * x
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    }
}

/// Apply sRGB gamma decoding (display → linear).
#[inline]
pub fn srgb_to_linear(x: f32) -> f32 {
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}

/// Apply tone mapping to a linear f32 frame.
/// Also applies sRGB gamma encoding — output is display-ready.
pub fn tone_map(frame: Frame, mode: ToneMappingMode) -> Frame {
    let mapped: Vec<f32> = frame
        .pixels
        .iter()
        .map(|&x| {
            let tone_mapped = match mode {
                ToneMappingMode::AcesFilmic => aces_filmic(x),
                ToneMappingMode::Reinhard => reinhard(x),
                ToneMappingMode::None => x.clamp(0.0, 1.0),
            };
            linear_to_srgb(tone_mapped)
        })
        .collect();

    Frame::new(mapped, frame.width, frame.height)
}
