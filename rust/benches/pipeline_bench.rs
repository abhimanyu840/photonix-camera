use criterion::{black_box, criterion_group, criterion_main, Criterion};
use photonix_core::compute::burst_stack::{laplacian_variance, stack_burst, Frame};
use photonix_core::compute::color::{white_balance, WhiteBalanceMode};
use photonix_core::compute::sharpen::unsharp_mask;
use photonix_core::compute::tone_map::{tone_map, ToneMappingMode};

fn make_test_frame(w: u32, h: u32) -> Frame {
    let pixels = vec![0.5f32; (w * h * 3) as usize];
    Frame::new(pixels, w, h)
}

fn bench_laplacian(c: &mut Criterion) {
    let frame = make_test_frame(1920, 1080);
    let luma = frame.to_luma();
    c.bench_function("laplacian_variance_1080p", |b| {
        b.iter(|| laplacian_variance(black_box(&luma), 1920, 1080))
    });
}

fn bench_tone_map(c: &mut Criterion) {
    c.bench_function("aces_tone_map_1080p", |b| {
        b.iter(|| {
            let frame = make_test_frame(1920, 1080);
            tone_map(black_box(frame), ToneMappingMode::AcesFilmic)
        })
    });
}

fn bench_sharpen(c: &mut Criterion) {
    c.bench_function("unsharp_mask_1080p", |b| {
        b.iter(|| {
            let frame = make_test_frame(1920, 1080);
            unsharp_mask(black_box(frame), 0.4)
        })
    });
}

fn bench_white_balance(c: &mut Criterion) {
    c.bench_function("grey_world_wb_1080p", |b| {
        b.iter(|| {
            let frame = make_test_frame(1920, 1080);
            white_balance(black_box(frame), WhiteBalanceMode::GreyWorld)
        })
    });
}

criterion_group!(
    benches,
    bench_laplacian,
    bench_tone_map,
    bench_sharpen,
    bench_white_balance
);
criterion_main!(benches);
            mertens_fusion(black_box(&frames))
        })
    });
}

// ── Tone mapping ──────────────────────────────────────────────────────────────

fn bench_tone_map(c: &mut Criterion) {
    let mut group = c.benchmark_group("tone_map");
    for mode in [ToneMappingMode::AcesFilmic, ToneMappingMode::Reinhard] {
        let name = format!("{:?}", mode);
        group.bench_function(&name, |b| {
            b.iter(|| {
                let frame = make_frame(4032, 3024);
                tone_map(black_box(frame), mode)
            })
        });
    }
    group.finish();
}

// ── Sharpen ───────────────────────────────────────────────────────────────────

fn bench_sharpen(c: &mut Criterion) {
    c.bench_function("unsharp_mask_12mp", |b| {
        b.iter(|| {
            let frame = make_frame(4032, 3024);
            unsharp_mask(black_box(frame), 0.4)
        })
    });
}

// ── White balance ─────────────────────────────────────────────────────────────

fn bench_white_balance(c: &mut Criterion) {
    c.bench_function("grey_world_wb_12mp", |b| {
        b.iter(|| {
            let frame = make_frame(4032, 3024);
            white_balance(black_box(frame), WhiteBalanceMode::GreyWorld)
        })
    });
}

// ── Full classical pipeline ───────────────────────────────────────────────────

fn bench_full_pipeline_classical(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline_classical");
    group.measurement_time(std::time::Duration::from_secs(30));
    group.sample_size(10);

    for scene in [Scene::Night, Scene::Standard, Scene::Landscape] {
        let name = scene.as_str().to_string();
        group.bench_function(&name, |b| {
            let config = scene.pipeline_config();
            let frame_count = config.burst_count as usize;
            b.iter(|| {
                let frames: Vec<Vec<u8>> = (0..frame_count)
                    .map(|_| make_jpeg(1920, 1080))
                    .collect();
                run_full_pipeline(
                    black_box(frames),
                    &config,
                    scene,
                    |_, _| {},
                ).unwrap()
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_burst_align,
    bench_hdr_merge,
    bench_tone_map,
    bench_sharpen,
    bench_white_balance,
    bench_full_pipeline_classical,
);
criterion_main!(benches);