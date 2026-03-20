#!/usr/bin/env python3
"""
Photonix Camera — Image Quality Metrics
Usage: python scripts/quality_check.py <original.jpg> <processed.jpg>

Outputs: PSNR, SSIM, and a pass/fail table per pipeline mode.
Requires: pip install scikit-image pillow numpy
"""

import sys
import json
import numpy as np
from pathlib import Path


def load_image(path: str) -> np.ndarray:
    from PIL import Image

    img = Image.open(path).convert("RGB")
    return np.array(img, dtype=np.float32) / 255.0


def psnr(original: np.ndarray, processed: np.ndarray) -> float:
    """Peak Signal-to-Noise Ratio. Higher = better. >30dB is good."""
    if original.shape != processed.shape:
        from PIL import Image
        import io

        # Resize processed to match original
        h, w = original.shape[:2]
        from skimage.transform import resize

        processed = resize(processed, (h, w), anti_aliasing=True)

    mse = np.mean((original - processed) ** 2)
    if mse < 1e-10:
        return 100.0
    return float(10 * np.log10(1.0 / mse))


def ssim(original: np.ndarray, processed: np.ndarray) -> float:
    """Structural Similarity Index. Range [0,1]. >0.9 is excellent."""
    from skimage.metrics import structural_similarity

    if original.shape != processed.shape:
        from skimage.transform import resize

        h, w = original.shape[:2]
        processed = resize(processed, (h, w), anti_aliasing=True)
    return float(
        structural_similarity(
            original, processed, multichannel=True, data_range=1.0, channel_axis=2
        )
    )


def brisque_score(img: np.ndarray) -> float:
    """
    Blind/Referenceless Image Spatial Quality Evaluator.
    Lower = better quality. <30 is excellent, <50 is good.
    Approximation using local variance (full BRISQUE needs libsvm).
    """
    gray = 0.2126 * img[:, :, 0] + 0.7152 * img[:, :, 1] + 0.0722 * img[:, :, 2]
    # Local variance as a proxy for noise/artifact level
    from scipy.ndimage import uniform_filter

    mean = uniform_filter(gray, size=7)
    mean_sq = uniform_filter(gray**2, size=7)
    variance = np.clip(mean_sq - mean**2, 0, None)
    # Normalise to approximate BRISQUE scale
    return float(np.mean(np.sqrt(variance)) * 1000)


# Pass/fail thresholds per pipeline mode
THRESHOLDS = {
    "standard": {
        "psnr_min": 25.0,
        "ssim_min": 0.85,
        "brisque_max": 45.0,
    },
    "night": {
        "psnr_min": 22.0,  # Lower — more aggressive processing
        "ssim_min": 0.80,
        "brisque_max": 50.0,
    },
    "portrait": {
        "psnr_min": 20.0,  # Super-res changes resolution
        "ssim_min": 0.75,
        "brisque_max": 40.0,
    },
    "landscape": {
        "psnr_min": 23.0,
        "ssim_min": 0.82,
        "brisque_max": 42.0,
    },
}


def main():
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <original.jpg> <processed.jpg> [mode]")
        print("  mode: standard | night | portrait | landscape (default: standard)")
        sys.exit(1)

    original_path = sys.argv[1]
    processed_path = sys.argv[2]
    mode = sys.argv[3] if len(sys.argv) > 3 else "standard"

    original = load_image(original_path)
    processed = load_image(processed_path)

    psnr_val = psnr(original, processed)
    ssim_val = ssim(original, processed)

    try:
        from scipy.ndimage import uniform_filter

        brisque_val = brisque_score(processed)
    except ImportError:
        brisque_val = None
        print("Note: scipy not installed — skipping BRISQUE")

    thresholds = THRESHOLDS.get(mode, THRESHOLDS["standard"])

    print(f"\n{'='*55}")
    print(f"  Photonix Quality Report — {mode.upper()} mode")
    print(f"  Original:  {original_path}")
    print(f"  Processed: {processed_path}")
    print(f"{'='*55}")
    print(f"  {'Metric':<20} {'Value':>10}  {'Threshold':>12}  {'Pass?':>6}")
    print(f"  {'-'*50}")

    psnr_pass = psnr_val >= thresholds["psnr_min"]
    ssim_pass = ssim_val >= thresholds["ssim_min"]

    print(
        f"  {'PSNR (dB)':<20} {psnr_val:>10.2f}  {thresholds['psnr_min']:>12.1f}  {'✓' if psnr_pass else '✗':>6}"
    )
    print(
        f"  {'SSIM':<20} {ssim_val:>10.4f}  {thresholds['ssim_min']:>12.2f}  {'✓' if ssim_pass else '✗':>6}"
    )

    if brisque_val is not None:
        brisque_pass = brisque_val <= thresholds["brisque_max"]
        print(
            f"  {'BRISQUE (lower=better)':<20} {brisque_val:>10.1f}  {thresholds['brisque_max']:>12.1f}  {'✓' if brisque_pass else '✗':>6}"
        )
    else:
        brisque_pass = True

    print(f"{'='*55}")
    all_pass = psnr_pass and ssim_pass and brisque_pass
    print(f"  Overall: {'PASS ✓' if all_pass else 'FAIL ✗'}")
    print(f"{'='*55}\n")

    sys.exit(0 if all_pass else 1)


if __name__ == "__main__":
    main()
