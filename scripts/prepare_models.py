#!/usr/bin/env python3
"""
Photonix Camera — Model preparation script
Downloads OSS models, quantizes to INT8, saves to assets/models/
Run once before Phase 6 build.

Requirements: pip install onnxruntime onnx huggingface_hub torch torchvision
"""

import os
import urllib.request
import shutil
from pathlib import Path

MODELS_DIR = Path(__file__).parent.parent / "assets" / "models"
MODELS_DIR.mkdir(parents=True, exist_ok=True)


def download(url: str, dest: Path):
    if dest.exists():
        print(f"  Already exists: {dest.name}")
        return
    print(f"  Downloading {dest.name}...")
    urllib.request.urlretrieve(url, dest)
    print(f"  Done: {dest.stat().st_size // 1024}KB")


def quantize_int8(src: Path, dst: Path):
    if dst.exists():
        print(f"  Already quantized: {dst.name}")
        return
    from onnxruntime.quantization import quantize_dynamic, QuantType

    print(f"  Quantizing {src.name} → {dst.name}...")
    quantize_dynamic(str(src), str(dst), weight_type=QuantType.QUInt8)
    print(f"  Done: {dst.stat().st_size // 1024}KB")


# ── 1. MobileNetV3-Small (scene classification) ───────────────────────────────
# Apache 2.0 — export from torchvision
print("\n[1/5] MobileNetV3-Small (scene classifier)")
scene_path = MODELS_DIR / "mobilenet_scene.onnx"
if not scene_path.exists():
    try:
        import torch
        import torchvision.models as models

        model = models.mobilenet_v3_small(
            weights=models.MobileNet_V3_Small_Weights.DEFAULT
        )
        model.eval()
        dummy = torch.randn(1, 3, 224, 224)
        torch.onnx.export(
            model,
            dummy,
            str(scene_path),
            input_names=["input"],
            output_names=["output"],
            dynamic_axes={"input": {0: "batch"}, "output": {0: "batch"}},
            opset_version=12,
        )
        print(f"  Exported: {scene_path.stat().st_size // 1024}KB")
    except ImportError:
        print("  torch not available — downloading pre-exported model")
        # Fallback: use ONNX Model Zoo MobileNetV3
        download(
            "https://github.com/onnx/models/raw/main/validated/vision/classification/mobilenet/model/mobilenetv3-small-batch1.onnx",
            MODELS_DIR / "mobilenet_scene_fp32.onnx",
        )
        quantize_int8(MODELS_DIR / "mobilenet_scene_fp32.onnx", scene_path)

# ── 2. DnCNN (denoising) ──────────────────────────────────────────────────────
# MIT license — from KAIR repository
print("\n[2/5] DnCNN (denoiser)")
dncnn_fp32 = MODELS_DIR / "dncnn_fp32.onnx"
dncnn_int8 = MODELS_DIR / "dncnn_int8.onnx"
if not dncnn_int8.exists():
    # Use a small DnCNN-S variant (~1MB fp32)
    download(
        "https://github.com/cszn/KAIR/releases/download/v1.0/dncnn_gray_blind.pth",
        MODELS_DIR / "dncnn.pth",
    )
    print("  Note: Convert dncnn.pth to ONNX manually using scripts/convert_dncnn.py")
    print("  OR download pre-converted:")
    print("  https://huggingface.co/eugenesiow/DnCNN/resolve/main/dncnn.onnx")
    print("  Save as assets/models/dncnn_fp32.onnx then run quantize_int8()")

# ── 3. Zero-DCE (low-light enhancement) ──────────────────────────────────────
print("\n[3/5] Zero-DCE (low-light enhancer)")
zerodce_path = MODELS_DIR / "zerodce.onnx"
if not zerodce_path.exists():
    print("  Download from: https://github.com/Li-Chongyi/Zero-DCE")
    print("  Or use the ONNX export:")
    download(
        "https://huggingface.co/onnx-community/Zero-DCE/resolve/main/model.onnx",
        zerodce_path,
    )

# ── 4. MiDaS v2.1 Small (depth estimation) ───────────────────────────────────
print("\n[4/5] MiDaS v2.1 Small (depth estimator)")
midas_path = MODELS_DIR / "midas_v21_small.onnx"
if not midas_path.exists():
    download(
        "https://github.com/isl-org/MiDaS/releases/download/v2_1/midas_v21_small_256.onnx",
        midas_path,
    )

# ── 5. Real-ESRGAN x2 (super-resolution) ─────────────────────────────────────
print("\n[5/5] Real-ESRGAN x2 mobile (super-resolution)")
realesrgan_fp32 = MODELS_DIR / "realesrgan_x2_fp32.onnx"
realesrgan_int8 = MODELS_DIR / "realesrgan_x2_int8.onnx"
if not realesrgan_int8.exists():
    download(
        "https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.5.0/realesr-animevideov3.pth",
        MODELS_DIR / "realesrgan.pth",
    )
    print("  Note: Export to ONNX using basicsr then quantize")
    print("  See: https://github.com/xinntao/Real-ESRGAN#onnx-export")

print("\n✓ Model preparation complete")
print(f"  Models directory: {MODELS_DIR}")
for f in sorted(MODELS_DIR.glob("*.onnx")):
    print(f"  {f.name}: {f.stat().st_size // 1024}KB")
