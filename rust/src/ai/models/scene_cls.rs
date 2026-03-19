use crate::ai::model_cache::{load_model, MODEL_KEY_SCENE};
use crate::ai::preprocess::normalize::{normalize_imagenet, resize_bilinear};
use anyhow::Result;
use ort::value::Tensor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scene {
    Night,
    Portrait,
    Landscape,
    Macro,
    Standard,
}

impl Scene {
    pub fn as_str(&self) -> &'static str {
        match self {
            Scene::Night => "night",
            Scene::Portrait => "portrait",
            Scene::Landscape => "landscape",
            Scene::Macro => "macro",
            Scene::Standard => "standard",
        }
    }
}

pub fn classify_scene(img_rgb: &[f32], height: usize, width: usize) -> Result<Scene> {
    let avg_luma: f32 = img_rgb
        .chunks_exact(3)
        .map(|p| 0.2126 * p[0] + 0.7152 * p[1] + 0.0722 * p[2])
        .sum::<f32>()
        / (height * width) as f32;
    if avg_luma < 0.15 {
        return Ok(Scene::Night);
    }

    let session = load_model(MODEL_KEY_SCENE)?;
    let mut session = session.lock().unwrap();
    let resized = resize_bilinear(img_rgb, width, height, 224, 224, 3);
    let array = normalize_imagenet(&resized, 224, 224);
    let input = Tensor::from_array(array.into_dyn())?;
    let outputs = session.run(ort::inputs![input])?;
    let logits = outputs[0].try_extract_array::<f32>()?;

    let n = logits.shape()[1];
    let top = (0..n)
        .max_by(|&a, &b| logits[[0, a]].partial_cmp(&logits[[0, b]]).unwrap())
        .unwrap_or(0);
    Ok(match top {
        0..=9 | 840 | 878 | 895 => Scene::Portrait,
        970..=980 => Scene::Landscape,
        300..=400 | 984..=987 => Scene::Macro,
        _ => Scene::Standard,
    })
}
