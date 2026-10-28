"""Read-only resources in either a checkout or a frozen engine bundle."""
import sys
from pathlib import Path


def resource_root() -> Path:
    if getattr(sys, "frozen", False):
        return Path(sys._MEIPASS)
    return Path(__file__).resolve().parents[2]


def read_template(path: Path):
    """OpenCV's filename API can fail for Chinese Windows installation paths."""
    import cv2
    import numpy as np

    return cv2.imdecode(np.frombuffer(path.read_bytes(), dtype=np.uint8), cv2.IMREAD_GRAYSCALE)
