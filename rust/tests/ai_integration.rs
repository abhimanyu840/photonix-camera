#[cfg(test)]
mod tests {
    use photonix_core::ai::preprocess::normalize::{
        hwc_to_nchw, nchw_to_hwc, normalize_imagenet, resize_bilinear, rgb_to_luma_nchw,
    };
    use photonix_core::ai::preprocess::tile::{split_into_tiles, stitch_tiles};
    use photonix_core::ai::session_pool::init_environment;

    fn px(w: usize, h: usize, ch: usize) -> Vec<f32> {
        (0..w * h * ch).map(|i| (i % 256) as f32 / 255.0).collect()
    }

    #[test]
    fn test_init() {
        init_environment().unwrap();
    }

    #[test]
    fn test_normalize_shape() {
        assert_eq!(
            normalize_imagenet(&px(224, 224, 3), 224, 224).shape(),
            &[1, 3, 224, 224]
        );
    }

    #[test]
    fn test_normalize_no_nan() {
        for &v in normalize_imagenet(&px(224, 224, 3), 224, 224).iter() {
            assert!(!v.is_nan() && !v.is_infinite());
        }
    }

    #[test]
    fn test_hwc_nchw_roundtrip() {
        let orig = px(64, 64, 3);
        let tensor = hwc_to_nchw(&orig, 64, 64, 3);
        let back = nchw_to_hwc(tensor.view().into_dyn(), 64, 64, 3);
        for (a, b) in orig.iter().zip(back.iter()) {
            assert!((a - b).abs() < 1e-5);
        }
    }

    #[test]
    fn test_luma_shape() {
        assert_eq!(
            rgb_to_luma_nchw(&px(64, 64, 3), 64, 64).shape(),
            &[1, 1, 64, 64]
        );
    }

    #[test]
    fn test_luma_range() {
        for &v in rgb_to_luma_nchw(&px(64, 64, 3), 64, 64).iter() {
            assert!(v >= 0.0 && v <= 1.0);
        }
    }

    #[test]
    fn test_resize_shape() {
        assert_eq!(
            resize_bilinear(&px(640, 480, 3), 640, 480, 256, 256, 3).len(),
            256 * 256 * 3
        );
    }

    #[test]
    fn test_resize_no_nan() {
        for &v in &resize_bilinear(&px(640, 480, 3), 640, 480, 256, 256, 3) {
            assert!(!v.is_nan());
        }
    }

    #[test]
    fn test_tile_split() {
        assert!(!split_into_tiles(&px(512, 512, 3), 512, 512, 3, 256, 32).is_empty());
    }

    #[test]
    fn test_tile_stitch() {
        let tiles = split_into_tiles(&px(512, 512, 3), 512, 512, 3, 256, 32);
        let res: Vec<Vec<f32>> = tiles.iter().map(|t| t.pixels.clone()).collect();
        let out = stitch_tiles(&tiles, &res, 512, 512, 3, 32, 1);
        assert_eq!(out.len(), 512 * 512 * 3);
        for &v in &out {
            assert!(!v.is_nan() && v >= 0.0 && v <= 1.0);
        }
    }
}

#[test]
fn test_scene_pipeline_config() {
    use photonix_core::pipeline::scene::Scene;

    let night = Scene::Night.pipeline_config();
    assert!(night.run_denoiser);
    assert!(night.run_enhancer);
    assert!(!night.run_super_res);
    assert_eq!(night.burst_count, 7);

    let portrait = Scene::Portrait.pipeline_config();
    assert!(portrait.run_denoiser);
    assert!(portrait.run_super_res);
    assert!(portrait.run_depth);
    assert_eq!(portrait.burst_count, 3);

    let landscape = Scene::Landscape.pipeline_config();
    assert!(!landscape.run_denoiser);
    assert!(landscape.run_super_res);
    assert!(landscape.run_hdr);

    let standard = Scene::Standard.pipeline_config();
    assert!(standard.run_denoiser);
    assert!(!standard.run_super_res);
    assert!(!standard.run_depth);
}

#[test]
fn test_scene_from_hint() {
    use photonix_core::pipeline::scene::Scene;
    assert_eq!(Scene::from_hint("night"), Scene::Night);
    assert_eq!(Scene::from_hint("PORTRAIT"), Scene::Portrait);
    assert_eq!(Scene::from_hint("unknown"), Scene::Standard);
}
