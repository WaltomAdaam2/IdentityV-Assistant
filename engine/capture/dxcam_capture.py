from core.models import Rect
from capture.base import CaptureBackend, Frame


class DxcamCapture(CaptureBackend):
    def __init__(self) -> None:
        import dxcam
        self._camera = dxcam.create(output_color="BGR")

    def capture(self, region: Rect | None = None, hwnd: int | None = None) -> Frame:
        area = (region.left, region.top, region.right, region.bottom) if region else None
        image = self._camera.grab(region=area)
        if image is None:
            raise RuntimeError("DXcam returned no frame")
        return Frame.from_image(image, "dxcam")
