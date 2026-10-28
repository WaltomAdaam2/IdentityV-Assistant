"""Offline smoke check used on the built engine, without input or capture."""
import importlib
import json

from config.loader import load_config
from core.resources import read_template, resource_root


def check_package() -> None:
    for name in (
        "cv2", "numpy", "psutil", "pydantic", "mss", "dxcam",
        "win32api", "win32gui", "win32con", "win32process",
        "windows_capture", "onnxruntime", "rapidocr_onnxruntime",
    ):
        importlib.import_module(name)
    from rapidocr_onnxruntime import RapidOCR

    root = resource_root()
    load_config(root / "config" / "default.json")
    templates = list((root / "assets" / "templates").rglob("*.png"))
    if not templates:
        raise RuntimeError("No recognition templates bundled")
    for template in templates:
        if read_template(template) is None:
            raise RuntimeError(f"Invalid template: {template}")
    # Constructing OCR loads the bundled ONNX models, with no network download.
    RapidOCR()
    print(json.dumps({"self_test": "passed", "templates": len(templates)}))
