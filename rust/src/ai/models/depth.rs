use crate::ai::model_cache::{load_model, MODEL_KEY_DEPTH};
use crate::ai::preprocess::normalize::{normalize_imagenet, resize_bilinear};
use anyhow::Result;
use ort::value::Tensor;

const MIDAS_SIZE: usize = 256;

pub fn run_depth(img_rgb: &[f32], orig_h: usize, orig_w: usize) -> Result<Vec<f32>> {
    let session = load_model(MODEL_KEY_DEPTH)?;
    let mut session = session.lock().unwrap();
    let resized = resize_bilinear(img_rgb, orig_w, orig_h, MIDAS_SIZE, MIDAS_SIZE, 3);
    let array = normalize_imagenet(&resized, MIDAS_SIZE, MIDAS_SIZE);
    let input = Tensor::from_array(array.into_dyn())?;
    let outputs = session.run(ort::inputs![input])?;
    let out = outputs[0].try_extract_array::<f32>()?;

    let raw: Vec<f32> = out.iter().copied().collect();
    let (mn, mx) = raw
        .iter()
        .fold((f32::MAX, f32::MIN), |(a, b), &v| (a.min(v), b.max(v)));
    let range = (mx - mn).max(1e-6);
    let norm: Vec<f32> = raw.iter().map(|&v| (v - mn) / range).collect();
    Ok(resize_bilinear(
        &norm, MIDAS_SIZE, MIDAS_SIZE, orig_w, orig_h, 1,
    ))
}
