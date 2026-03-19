use crate::ai::model_cache::{load_model, MODEL_KEY_DENOISER};
use crate::ai::preprocess::normalize::rgb_to_luma_nchw;
use crate::ai::preprocess::tile::{split_into_tiles, stitch_tiles};
use anyhow::Result;
use ort::value::Tensor;

const TILE_SIZE: usize = 256;
const TILE_OVERLAP: usize = 32;

pub fn run_denoiser(img_rgb: &[f32], height: usize, width: usize) -> Result<Vec<f32>> {
    let session = load_model(MODEL_KEY_DENOISER)?;
    let mut session = session.lock().unwrap();
    let tiles = split_into_tiles(img_rgb, width, height, 3, TILE_SIZE, TILE_OVERLAP);
    let mut results = Vec::with_capacity(tiles.len());

    for tile in &tiles {
        let array = rgb_to_luma_nchw(&tile.pixels, tile.h, tile.w);
        let input = Tensor::from_array(array.into_dyn())?;
        let outputs = session.run(ort::inputs![input])?;
        let out = outputs[0].try_extract_array::<f32>()?;

        let mut rgb_out = tile.pixels.clone();
        for py in 0..tile.h {
            for px in 0..tile.w {
                let i = (py * tile.w + px) * 3;
                let orig_l = 0.2126 * tile.pixels[i]
                    + 0.7152 * tile.pixels[i + 1]
                    + 0.0722 * tile.pixels[i + 2];
                let new_l = out[[0, 0, py, px]];
                if orig_l > 1e-6 {
                    let r = new_l / orig_l;
                    for c in 0..3 {
                        rgb_out[i + c] = (tile.pixels[i + c] * r).clamp(0.0, 1.0);
                    }
                }
            }
        }
        results.push(rgb_out);
    }
    Ok(stitch_tiles(
        &tiles,
        &results,
        width,
        height,
        3,
        TILE_OVERLAP,
        1,
    ))
}
