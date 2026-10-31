from config.schema import CaptureConfig
from core.models import Rect
from capture.base import CaptureBackend, Frame
from dataclasses import replace
from datetime import datetime, timezone


class CaptureManager:
    def __init__(self, config: CaptureConfig) -> None:
        self.config = config
        self.backend: CaptureBackend | None = None
        self.backend_name = "not_initialized"
        self.last_error: str | None = None
        self.retain_last_frame = False
        self.last_frame = None
        self.last_frame_at = None
        self.last_frame_hwnd = None

    def _remember(self, frame, hwnd):
        if self.retain_last_frame and frame.source == "wgc" and hwnd:
            try:
                self.last_frame = replace(frame, image=frame.image.copy())
                self.last_frame_at = datetime.now(timezone.utc)
                self.last_frame_hwnd = hwnd
            except MemoryError:
                self.last_frame = None
        else:
            self.last_frame = None
        return frame

    @staticmethod
    def _create_backend(name: str) -> CaptureBackend:
        module = __import__(f"capture.{name}_capture", fromlist=["*"])
        classes = {
            "wgc": "WindowsGraphicsCapture",
            "dxcam": "DxcamCapture",
            "mss": "MssCapture",
        }
        return module.__dict__[classes[name]]()

    def _choices(self) -> tuple[str, ...]:
        return ("wgc", "dxcam", "mss") if self.config.backend == "auto" else (self.config.backend,)

    def _ensure_backend(self) -> CaptureBackend:
        if self.backend:
            return self.backend
        choices = self._choices()
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

    def capture(self, region: Rect | None = None, hwnd: int | None = None) -> Frame:
        if region is not None and (region.width <= 0 or region.height <= 0):
            raise ValueError("capture region must have positive width and height")
        errors: list[str] = []
        try:
            return self._remember(self._ensure_backend().capture(region, hwnd), hwnd)
        except Exception as primary_error:
            errors.append(f"{self.backend_name}: {primary_error}")
            if self.config.backend == "auto":
                choices = self._choices()
                start = choices.index(self.backend_name) + 1 if self.backend_name in choices else 0
                for name in choices[start:]:
                    try:
                        self.backend = self._create_backend(name)
                        self.backend_name = name
                        frame = self.backend.capture(region, hwnd)
                        self.last_error = "; ".join(errors)
                        return self._remember(frame, hwnd)
                    except Exception as fallback_error:
                        errors.append(f"{name}: {fallback_error}")
                self.backend, self.backend_name = None, "unavailable"
                self.last_error = "; ".join(errors)
                raise RuntimeError("all capture backends failed: " + self.last_error) from primary_error
            self.last_error = errors[0]
            raise
