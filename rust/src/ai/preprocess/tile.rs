pub struct Tile {
    pub pixels: Vec<f32>,
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub channels: usize,
}

pub fn split_into_tiles(
    pixels: &[f32],
    img_w: usize,
    img_h: usize,
    channels: usize,
    tile_size: usize,
    overlap: usize,
) -> Vec<Tile> {
    let step = tile_size.saturating_sub(overlap).max(1);
    let mut tiles = Vec::new();
    let mut y = 0usize;
    loop {
        let ty = if img_h >= tile_size {
            y.min(img_h - tile_size)
        } else {
            0
        };
        let th = (img_h - ty).min(tile_size);
        let mut x = 0usize;
        loop {
            let tx = if img_w >= tile_size {
                x.min(img_w - tile_size)
            } else {
                0
            };
            let tw = (img_w - tx).min(tile_size);
            let mut px = vec![0.0f32; tw * th * channels];
            for row in 0..th {
                for col in 0..tw {
                    let si = ((ty + row) * img_w + (tx + col)) * channels;
                    let di = (row * tw + col) * channels;
                    px[di..di + channels].copy_from_slice(&pixels[si..si + channels]);
                }
            }
            tiles.push(Tile {
                pixels: px,
                x: tx,
                y: ty,
                w: tw,
                h: th,
                channels,
            });
            if tx + tw >= img_w {
                break;
            }
            x += step;
        }
        if ty + th >= img_h {
            break;
        }
        y += step;
    }
    tiles
}

pub fn stitch_tiles(
    tiles: &[Tile],
    results: &[Vec<f32>],
    img_w: usize,
    img_h: usize,
    channels: usize,
    overlap: usize,
    scale: usize,
) -> Vec<f32> {
    let ow = img_w * scale;
    let oh = img_h * scale;
    let ov = overlap * scale;
    let mut out = vec![0.0f32; ow * oh * channels];
    let mut wts = vec![0.0f32; ow * oh];

    for (tile, res) in tiles.iter().zip(results.iter()) {
        let tw = tile.w * scale;
        let th = tile.h * scale;
        let tx = tile.x * scale;
        let ty = tile.y * scale;
        for row in 0..th {
            for col in 0..tw {
                let wx = gauss_w(col, tw, ov);
                let wy = gauss_w(row, th, ov);
                let w = wx * wy;
                let op = (ty + row) * ow + (tx + col);
                let sp = (row * tw + col) * channels;
                for c in 0..channels {
                    out[op * channels + c] += res[sp + c] * w;
                }
                wts[op] += w;
            }
        }
    }

    for px in 0..(ow * oh) {
        let w = wts[px];
        if w > 1e-6 {
            for c in 0..channels {
                out[px * channels + c] = (out[px * channels + c] / w).clamp(0.0, 1.0);
            }
        }
    }
    out
}

fn gauss_w(pos: usize, size: usize, overlap: usize) -> f32 {
    if overlap == 0 {
        return 1.0;
    }
    let d = pos.min(size.saturating_sub(1 + pos)) as f32;
    if d >= overlap as f32 {
        1.0
    } else {
        let t = d / overlap as f32;
        0.5 * (1.0 - (std::f32::consts::PI * (1.0 - t)).cos())
    }
}
