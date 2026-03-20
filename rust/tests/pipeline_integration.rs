use photonix_core::compute::burst_stack::{laplacian_variance, stack_burst, Frame};
use photonix_core::compute::color::{white_balance, WhiteBalanceMode};
use photonix_core::compute::hdr_merge::mertens_fusion;
use photonix_core::compute::sharpen::unsharp_mask;
use photonix_core::compute::tone_map::{tone_map, ToneMappingMode};
use photonix_core::pipeline::orchestrator::{
    decode_jpeg_to_frame, encode_frame_to_jpeg, run_full_pipeline,
};
use photonix_core::pipeline::scene::Scene;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn synthetic_frame(w: u32, h: u32, brightness: f32) -> Frame {
    let pixels: Vec<f32> = (0..w * h * 3)
        .map(|i| {
            let px = i / 3;
            let c = i % 3;
            let x = (px % w) as f32 / w as f32;
            let y = (px / w) as f32 / h as f32;
            ((x + y) * 0.5 * brightness + c as f32 * 0.05).clamp(0.0, 1.0)
        })
        .collect();
    Frame::new(pixels, w, h)
}

fn add_noise(frame: &Frame, stddev: f32) -> Frame {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let noisy: Vec<f32> = frame
        .pixels
        .iter()
        .enumerate()
        .map(|(i, &p)| {
            let mut h = DefaultHasher::new();
            i.hash(&mut h);
            let hash = h.finish();
            let noise = ((hash % 1000) as f32 / 500.0 - 1.0) * stddev;
            (p + noise).clamp(0.0, 1.0)
        })
        .collect();
    Frame::new(noisy, frame.width, frame.height)
}

fn psnr(a: &Frame, b: &Frame) -> f64 {
    assert_eq!(a.pixels.len(), b.pixels.len());
    let mse: f64 = a
        .pixels
        .iter()
        .zip(b.pixels.iter())
        .map(|(&x, &y)| (x as f64 - y as f64).powi(2))
        .sum::<f64>()
        / a.pixels.len() as f64;
    if mse < 1e-10 {
        return 100.0;
    }
    10.0 * (1.0f64 / mse).log10()
}

fn make_test_jpeg(w: u32, h: u32, brightness: f32) -> Vec<u8> {
    let frame = synthetic_frame(w, h, brightness);
    let mapped = tone_map(frame, ToneMappingMode::AcesFilmic);
    encode_frame_to_jpeg(&mapped, 90).expect("encode failed")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn test_burst_alignment_no_ghosting() {
    // Reference frames at medium brightness
    let ref_frame = synthetic_frame(256, 256, 0.5);
    let frame2 = synthetic_frame(256, 256, 0.5);
    // Ghost: very different brightness — should be weighted near zero
    let ghost_frame = synthetic_frame(256, 256, 0.9);

    // Stack with ghost rejection
    let frames = vec![ref_frame, frame2, ghost_frame];
    let stacked = stack_burst(frames).expect("stack_burst failed");

    // Output average luma should be close to 0.5 (reference), not 0.9 (ghost)
    let avg: f32 = stacked.pixels.iter().sum::<f32>() / stacked.pixels.len() as f32;

    assert!(
        avg < 0.7,
        "Ghost frame (brightness 0.9) should be down-weighted. \
         Output avg={avg:.3}, expected < 0.7"
    );
    assert!(avg > 0.3, "Output too dark: avg={avg:.3}");
}

#[test]
fn test_hdr_merge_preserves_highlights() {
    let dark = synthetic_frame(128, 128, 0.2);
    let normal = synthetic_frame(128, 128, 0.5);
    let bright = synthetic_frame(128, 128, 0.9);

    let fused = mertens_fusion(&[dark, normal, bright]);

    for &p in &fused.pixels {
        assert!(p >= 0.0 && p <= 1.0, "HDR pixel out of range: {p}");
    }

    let avg_fused: f32 = fused.pixels.iter().sum::<f32>() / fused.pixels.len() as f32;
    assert!(
        avg_fused > 0.25,
        "Fused output too dark: avg={avg_fused:.3}"
    );
}

#[test]
fn test_sharpness_score_increases_after_sharpen() {
    let frame = synthetic_frame(128, 128, 0.5);
    let luma_before = frame.to_luma();
    let score_before = laplacian_variance(&luma_before, 128, 128);

    let sharpened = unsharp_mask(frame, 0.6);
    let luma_after = sharpened.to_luma();
    let score_after = laplacian_variance(&luma_after, 128, 128);

    assert!(
        score_after > score_before,
        "Sharpening should increase Laplacian variance: \
         before={score_before:.1} after={score_after:.1}"
    );
}

#[test]
fn test_white_balance_reduces_cast() {
    let mut pixels = vec![0.0f32; 128 * 128 * 3];
    for i in 0..128 * 128 {
        pixels[i * 3] = 0.8;
        pixels[i * 3 + 1] = 0.4;
        pixels[i * 3 + 2] = 0.4;
    }
    let frame = Frame::new(pixels, 128, 128);
    let balanced = white_balance(frame, WhiteBalanceMode::GreyWorld);

    let n = (128 * 128) as f32;
    let r_mean: f32 = balanced.pixels.iter().step_by(3).sum::<f32>() / n;
    let g_mean: f32 = balanced.pixels.iter().skip(1).step_by(3).sum::<f32>() / n;
    let b_mean: f32 = balanced.pixels.iter().skip(2).step_by(3).sum::<f32>() / n;

    let spread_before = 0.4f32;
    let spread_after = (r_mean - g_mean).abs().max((r_mean - b_mean).abs());

    assert!(
        spread_after < spread_before,
        "WB should reduce channel spread: before={spread_before:.2} after={spread_after:.2}"
    );
}

#[test]
fn test_scene_pipeline_configs_are_valid() {
    let scenes = [
        Scene::Night,
        Scene::Portrait,
        Scene::Landscape,
        Scene::Macro,
        Scene::Standard,
    ];
    for scene in scenes {
        let cfg = scene.pipeline_config();
        assert!(
            cfg.burst_count >= 1 && cfg.burst_count <= 7,
            "burst_count out of range for {:?}: {}",
            scene,
            cfg.burst_count
        );
        assert!(
            !cfg.tone_mapping.is_empty(),
            "tone_mapping empty for {:?}",
            scene
        );
        if scene == Scene::Night {
            assert_eq!(cfg.burst_count, 7);
            assert!(cfg.run_denoiser);
            assert!(cfg.run_enhancer);
        }
        if scene == Scene::Portrait {
            assert!(cfg.run_depth);
        }
        if scene == Scene::Landscape {
            assert!(cfg.run_hdr);
        }
    }
}

#[test]
fn test_encode_decode_roundtrip() {
    let original = synthetic_frame(64, 64, 0.6);
    let mapped = tone_map(original, ToneMappingMode::AcesFilmic);
    let jpeg = encode_frame_to_jpeg(&mapped, 95).expect("encode failed");

    assert!(!jpeg.is_empty());
    assert!(jpeg.len() > 100);

    let decoded = decode_jpeg_to_frame(&jpeg).expect("decode failed");
    assert_eq!(decoded.width, 64);
    assert_eq!(decoded.height, 64);
    assert_eq!(decoded.pixels.len(), 64 * 64 * 3);
}

#[test]
fn test_pipeline_output_not_corrupted() {
    let jpeg = make_test_jpeg(256, 256, 0.5);
    let config = Scene::Standard.pipeline_config();
    let result = run_full_pipeline(vec![jpeg], &config, Scene::Standard, |_, _| {})
        .expect("Pipeline failed");

    assert!(result.len() > 2);
    assert_eq!(result[0], 0xFF, "Not a JPEG: missing FFD8 header byte 0");
    assert_eq!(result[1], 0xD8, "Not a JPEG: missing FFD8 header byte 1");
}

#[test]
fn test_3_frame_burst_pipeline() {
    let frames: Vec<Vec<u8>> = (0..3)
        .map(|i| make_test_jpeg(256, 256, 0.4 + i as f32 * 0.1))
        .collect();
    let config = Scene::Standard.pipeline_config();
    let result = run_full_pipeline(frames, &config, Scene::Standard, |_, _| {});
    assert!(result.is_ok(), "3-frame burst failed: {:?}", result.err());
}

#[test]
fn test_progress_events_fired_in_order() {
    let jpeg = make_test_jpeg(128, 128, 0.5);
    let config = Scene::Standard.pipeline_config();
    let mut stages: Vec<(String, f32)> = Vec::new();

    run_full_pipeline(vec![jpeg], &config, Scene::Standard, |stage, progress| {
        stages.push((stage.to_string(), progress));
    })
    .expect("Pipeline failed");

    assert!(!stages.is_empty(), "No progress events fired");

    let mut last = 0.0f32;
    for (stage, progress) in &stages {
        assert!(
            *progress >= last,
            "Progress went backwards at '{}': {} < {}",
            stage,
            progress,
            last
        );
        last = *progress;
    }

    assert_eq!(
        stages.last().unwrap().1,
        1.0,
        "Final progress should be 1.0"
    );
}

#[test]
fn test_full_standard_pipeline_timing() {
    let jpeg = make_test_jpeg(1920, 1080, 0.5);
    let config = Scene::Standard.pipeline_config();

    let start = std::time::Instant::now();
    let result = run_full_pipeline(vec![jpeg], &config, Scene::Standard, |_, _| {});
    let elapsed = start.elapsed().as_millis();

    assert!(result.is_ok(), "Pipeline failed: {:?}", result.err());
    assert!(!result.unwrap().is_empty());
    println!("Standard pipeline (classical, 1080p): {elapsed}ms");
    assert!(elapsed < 5000, "Too slow: {elapsed}ms");
}

#[test]
fn test_scene_from_hint() {
    assert_eq!(Scene::from_hint("night"), Scene::Night);
    assert_eq!(Scene::from_hint("PORTRAIT"), Scene::Portrait);
    assert_eq!(Scene::from_hint("unknown"), Scene::Standard);
}

#[test]
fn test_scene_pipeline_config() {
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

// ── AI model tests (require model files) ─────────────────────────────────────

#[test]
#[ignore = "requires assets/models/dncnn_int8.onnx"]
fn test_dncnn_reduces_noise() {
    use photonix_core::ai::model_cache::register_models;
    register_models(&[("denoiser", "../../assets/models/dncnn_int8.onnx")]);

    let clean = synthetic_frame(128, 128, 0.5);
    let noisy = add_noise(&clean, 0.1);
    let psnr_b = psnr(&noisy, &clean);

    let pixels = photonix_core::ai::models::denoiser::run_denoiser(&noisy.pixels, 128, 128)
        .expect("Denoiser failed");
    let denoised = Frame::new(pixels, 128, 128);
    let psnr_a = psnr(&denoised, &clean);

    println!("PSNR before: {psnr_b:.2}dB  after: {psnr_a:.2}dB");
    assert!(psnr_a > psnr_b, "Denoiser should improve PSNR");
}

#[test]
#[ignore = "requires assets/models/midas_v21_small.onnx"]
fn test_midas_output_shape() {
    use photonix_core::ai::model_cache::register_models;
    register_models(&[("depth", "../../assets/models/midas_v21_small.onnx")]);

    let frame = synthetic_frame(640, 480, 0.5);
    let disparity =
        photonix_core::ai::models::depth::run_depth(&frame.pixels, 480, 640).expect("MiDaS failed");

    assert_eq!(disparity.len(), 640 * 480);
    for &v in &disparity {
        assert!(v >= 0.0 && v <= 1.0, "Disparity out of [0,1]: {v}");
        assert!(!v.is_nan());
    }
}
