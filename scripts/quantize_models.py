#!/usr/bin/env python3
"""
Photonix Camera — Model Quantization Script
Converts float32 ONNX models to INT8 for faster ARM64 inference.

Usage:
    pip install onnxruntime onnx
    python scripts/quantize_models.py

Expected speedup on ARM64 (INT8 vs float32):
  DnCNN:        2.0-2.5x  (conv-heavy, INT8 very effective)
  Zero-DCE:     1.5-2.0x  (lightweight, less headroom)
  Real-ESRGAN:  2.0-3.0x  (largest model, biggest gain)
  MiDaS:        1.5-2.0x  (mixed ops)
  MobileNetV3:  1.8-2.2x  (standard CNN, good INT8 support)
"""

from pathlib import Path
from onnxruntime.quantization import quantize_dynamic, QuantType
import onnx
import sys

MODELS_DIR = Path(__file__).parent.parent / "assets" / "models"

# Models to quantize: (input_name, output_name, per_channel)
MODELS = {
    "dncnn_fp32.onnx": ("dncnn_int8.onnx", True),
    "zerodce_fp32.onnx": ("zerodce_int8.onnx", False),
    "realesrgan_x2_fp32.onnx": ("realesrgan_x2_int8.onnx", True),
    "midas_v21_small.onnx": ("midas_v21_small_int8.onnx", False),
    "mobilenet_scene.onnx": ("mobilenet_scene_int8.onnx", True),
}


def quantize_model(src: Path, dst: Path, per_channel: bool = True):
    if dst.exists():
        print(f"  Already exists: {dst.name} ({dst.stat().st_size // 1024}KB)")
        return

    if not src.exists():
        print(f"  SKIP: {src.name} not found — download first")
        return

    print(f"  Quantizing {src.name} ({src.stat().st_size // 1024}KB) ...")
    quantize_dynamic(
        model_input=str(src),
        model_output=str(dst),
        weight_type=QuantType.QUInt8,
        per_channel=per_channel,
        # Reduce model ops to ONNX-compatible subset for NNAPI compatibility
        optimize_model=True,
    )
    ratio = src.stat().st_size / dst.stat().st_size
    print(f"  Done: {dst.name} ({dst.stat().st_size // 1024}KB) — {ratio:.1f}x smaller")


def print_speedup_table():
    print("\n" + "=" * 60)
    print("  Expected speedup: float32 → INT8 (ARM64 Cortex-A)")
    print("=" * 60)
    print(f"  {'Model':<25} {'fp32 size':>10} {'INT8 size':>10} {'Speedup':>8}")
    print(f"  {'-'*55}")
    models = [
        ("DnCNN", "~4MB", "~1MB", "2.0-2.5x"),
        ("Zero-DCE", "~0.4MB", "~0.1MB", "1.5-2.0x"),
        ("Real-ESRGAN", "~17MB", "~5MB", "2.0-3.0x"),
        ("MiDaS v2.1", "~50MB", "~14MB", "1.5-2.0x"),
        ("MobileNetV3", "~6MB", "~1.5MB", "1.8-2.2x"),
    ]
    for name, fp32, int8, speedup in models:
        print(f"  {name:<25} {fp32:>10} {int8:>10} {speedup:>8}")
    print("=" * 60 + "\n")


def main():
    print_speedup_table()
    print(f"Quantizing models in: {MODELS_DIR}\n")

    for src_name, (dst_name, per_channel) in MODELS.items():
        src = MODELS_DIR / src_name
        dst = MODELS_DIR / dst_name
        quantize_model(src, dst, per_channel)

    print("\nDone. Update assets/models/ references in Cargo.toml if needed.")


if __name__ == "__main__":
    main()
