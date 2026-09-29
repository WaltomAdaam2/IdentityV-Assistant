from config.schema import CaptureConfig
from core.models import Rect
from capture.base import CaptureBackend, Frame


class CaptureManager:
    def __init__(self, config: CaptureConfig) -> None:
        self.config = config
        self.backend: CaptureBackend | None = None
        self.backend_name = "unavailable"

    def _ensure_backend(self) -> CaptureBackend:
        if self.backend:
            return self.backend
        choices = ("dxcam", "mss") if self.config.backend == "auto" else (self.config.backend,)
        errors: list[str] = []
        for name in choices:
            try:
                self.backend = __import__(f"capture.{name}_capture", fromlist=["*"]).__dict__["DxcamCapture" if name == "dxcam" else "MssCapture"]()
                self.backend_name = name
                return self.backend
            except (ImportError, RuntimeError, OSError) as error:
                errors.append(f"{name}: {error}")
        raise RuntimeError("no capture backend available: " + "; ".join(errors))

    def capture(self, region: Rect | None = None) -> Frame:
        try:
            return self._ensure_backend().capture(region)
        except RuntimeError:
            if self.config.backend == "auto" and self.backend_name == "dxcam":
                self.backend, self.backend_name = None, "unavailable"
                return self._ensure_backend().capture(region)
            raise
