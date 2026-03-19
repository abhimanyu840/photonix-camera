use crate::ai::model_cache::{load_model, MODEL_KEY_ENHANCER};
use crate::ai::preprocess::normalize::{hwc_to_nchw, nchw_to_hwc};
use anyhow::Result;
use ort::value::Tensor;

pub fn run_enhancer(img_rgb: &[f32], height: usize, width: usize) -> Result<Vec<f32>> {
    let session = load_model(MODEL_KEY_ENHANCER)?;
    let mut session = session.lock().unwrap();
    let array = hwc_to_nchw(img_rgb, height, width, 3);
    let input = Tensor::from_array(array.into_dyn())?;
    let outputs = session.run(ort::inputs![input])?;
    let out = outputs[0].try_extract_array::<f32>()?;
    Ok(nchw_to_hwc(out.view(), height, width, 3))
}
