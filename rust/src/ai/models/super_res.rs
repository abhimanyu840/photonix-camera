use crate::ai::model_cache::{load_model, MODEL_KEY_SUPER_RES};
use crate::ai::preprocess::normalize::{hwc_to_nchw, nchw_to_hwc};
use crate::ai::preprocess::tile::{split_into_tiles, stitch_tiles};
use anyhow::Result;
use ort::value::Tensor;
use rayon::prelude::*;

const TILE_SIZE: usize = 512;
const TILE_OVERLAP: usize = 64;
const SCALE: usize = 2;

pub fn run_super_res(img_rgb: &[f32], height: usize, width: usize) -> Result<Vec<f32>> {
    let tiles = split_into_tiles(img_rgb, width, height, 3, TILE_SIZE, TILE_OVERLAP);

    // For small images (< 4 tiles): sequential — avoids Mutex contention overhead
    // For large images (>= 4 tiles): parallel — rayon distributes across big cores
    let tile_results: Vec<Vec<f32>> = if tiles.len() < 4 {
        tiles
            .iter()
            .map(|tile| run_tile(tile))
            .collect::<Result<_>>()?
    } else {
        tiles
            .par_iter()
            .map(|tile| run_tile(tile))
            .collect::<Result<_>>()?
    };

    Ok(stitch_tiles(
        &tiles,
        &tile_results,
        width,
        height,
        3,
        TILE_OVERLAP,
        SCALE,
    ))
}

fn run_tile(tile: &crate::ai::preprocess::tile::Tile) -> Result<Vec<f32>> {
    let session = load_model(MODEL_KEY_SUPER_RES)?;
    let mut session = session.lock().unwrap();
    let array = hwc_to_nchw(&tile.pixels, tile.h, tile.w, 3);
    let input = Tensor::from_array(array.into_dyn())?;
    let outputs = session.run(ort::inputs![input])?;
    let out = outputs[0].try_extract_array::<f32>()?;
    Ok(nchw_to_hwc(out.view(), tile.h * SCALE, tile.w * SCALE, 3))
}
