use ndarray::{Array4, ArrayViewD};

pub fn normalize_imagenet(pixels: &[f32], height: usize, width: usize) -> Array4<f32> {
    const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
    const STD: [f32; 3] = [0.229, 0.224, 0.225];
    let mut t = Array4::<f32>::zeros((1, 3, height, width));
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) * 3;
            for c in 0..3 {
                t[[0, c, y, x]] = (pixels[i + c] - MEAN[c]) / STD[c];
            }
        }
    }
    t
}

pub fn hwc_to_nchw(pixels: &[f32], height: usize, width: usize, ch: usize) -> Array4<f32> {
    let mut t = Array4::<f32>::zeros((1, ch, height, width));
    for y in 0..height {
        for x in 0..width {
            for c in 0..ch {
                t[[0, c, y, x]] = pixels[(y * width + x) * ch + c];
            }
        }
    }
    t
}

/// Takes ArrayViewD<f32> — what try_extract_array() returns
pub fn nchw_to_hwc(view: ArrayViewD<f32>, height: usize, width: usize, ch: usize) -> Vec<f32> {
    let mut out = vec![0.0f32; height * width * ch];
    for y in 0..height {
        for x in 0..width {
            for c in 0..ch {
                out[(y * width + x) * ch + c] = view[[0, c, y, x]];
            }
        }
    }
    out
}

pub fn rgb_to_luma_nchw(pixels: &[f32], height: usize, width: usize) -> Array4<f32> {
    let mut t = Array4::<f32>::zeros((1, 1, height, width));
    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) * 3;
            t[[0, 0, y, x]] = 0.2126 * pixels[i] + 0.7152 * pixels[i + 1] + 0.0722 * pixels[i + 2];
        }
    }
    t
}

pub fn resize_bilinear(
    src: &[f32],
    src_w: usize,
    src_h: usize,
    dst_w: usize,
    dst_h: usize,
    ch: usize,
) -> Vec<f32> {
    let mut out = vec![0.0f32; dst_h * dst_w * ch];
    let sx = src_w as f32 / dst_w as f32;
    let sy = src_h as f32 / dst_h as f32;
    for y in 0..dst_h {
        for x in 0..dst_w {
            let fx = ((x as f32 + 0.5) * sx - 0.5).max(0.0);
            let fy = ((y as f32 + 0.5) * sy - 0.5).max(0.0);
            let x0 = (fx as usize).min(src_w - 1);
            let y0 = (fy as usize).min(src_h - 1);
            let x1 = (x0 + 1).min(src_w - 1);
            let y1 = (y0 + 1).min(src_h - 1);
            let ax = fx - x0 as f32;
            let ay = fy - y0 as f32;
            for c in 0..ch {
                out[(y * dst_w + x) * ch + c] =
                    src[(y0 * src_w + x0) * ch + c] * (1.0 - ax) * (1.0 - ay)
                        + src[(y0 * src_w + x1) * ch + c] * ax * (1.0 - ay)
                        + src[(y1 * src_w + x0) * ch + c] * (1.0 - ax) * ay
                        + src[(y1 * src_w + x1) * ch + c] * ax * ay;
            }
        }
    }
    out
}
