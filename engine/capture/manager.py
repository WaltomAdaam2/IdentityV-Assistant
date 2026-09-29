from config.schema import CaptureConfig
from core.models import Rect
from capture.base import CaptureBackend, Frame


class CaptureManager:
    def __init__(self, config: CaptureConfig) -> None:
        self.config = config
        self.backend: CaptureBackend | None = None
        self.backend_name = "not_initialized"
        self.last_error: str | None = None

    @staticmethod
    def _create_backend(name: str) -> CaptureBackend:
        module = __import__(f"capture.{name}_capture", fromlist=["*"])
        return module.__dict__["DxcamCapture" if name == "dxcam" else "MssCapture"]()

    def _ensure_backend(self) -> CaptureBackend:
        if self.backend:
            return self.backend
        choices = ("dxcam", "mss") if self.config.backend == "auto" else (self.config.backend,)
        errors: list[str] = []
        for name in choices:
            try:
                self.backend = self._create_backend(name)
                self.backend_name = name
                self.last_error = None
                return self.backend
            except Exception as error:
                errors.append(f"{name}: {error}")
        self.last_error = "; ".join(errors)
        raise RuntimeError("no capture backend available: " + self.last_error)

    def capture(self, region: Rect | None = None) -> Frame:
        if region is not None and (region.width <= 0 or region.height <= 0):
            raise ValueError("capture region must have positive width and height")
        try:
            return self._ensure_backend().capture(region)
        except Exception as primary_error:
            if self.config.backend == "auto" and self.backend_name == "dxcam":
                try:
                    self.backend = self._create_backend("mss")
                    self.backend_name = "mss"
                    frame = self.backend.capture(region)
                    self.last_error = f"dxcam: {primary_error}"
                    return frame
                except Exception as fallback_error:
                    self.backend, self.backend_name = None, "unavailable"
                    self.last_error = f"dxcam: {primary_error}; mss: {fallback_error}"
                    raise RuntimeError("all capture backends failed: " + self.last_error) from fallback_error
            self.last_error = f"{self.backend_name}: {primary_error}"
            raise
