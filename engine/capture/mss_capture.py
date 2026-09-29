import numpy as np

from core.models import Rect
from capture.base import CaptureBackend, Frame


class MssCapture(CaptureBackend):
    def __init__(self) -> None:
        import mss
        self._mss = mss.mss()

    def capture(self, region: Rect | None = None, hwnd: int | None = None) -> Frame:
        monitor = {"left": region.left, "top": region.top, "width": region.width, "height": region.height} if region else self._mss.monitors[1]
        return Frame.from_image(np.asarray(self._mss.grab(monitor))[:, :, :3], "mss")
