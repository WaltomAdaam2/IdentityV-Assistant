from __future__ import annotations

import ctypes
import sys
from ctypes import wintypes
from threading import Event
from typing import Any

from capture.base import CaptureBackend, Frame
from core.models import Rect


class WindowsGraphicsCapture(CaptureBackend):
    """Capture a window surface even when another window covers it."""

    def __init__(self) -> None:
        if sys.platform != "win32":
            raise RuntimeError("Windows Graphics Capture is only available on Windows")
        from windows_capture import WindowsCapture

        self._capture_type = WindowsCapture

    def capture(self, region: Rect | None = None, hwnd: int | None = None) -> Frame:
        if not hwnd:
            raise ValueError("Windows Graphics Capture requires a window handle")

        from windows_capture import Frame as NativeFrame
        from windows_capture import InternalCaptureControl

        finished = Event()
        result: dict[str, object] = {}
        capture = self._capture_type(
            cursor_capture=False,
            draw_border=False,
            secondary_window=False,
            window_hwnd=hwnd,
        )

        @capture.event
        def on_frame_arrived(frame: NativeFrame, control: InternalCaptureControl) -> None:
            try:
                result["image"] = frame.convert_to_bgr().frame_buffer.copy()
            except Exception as error:
                result["error"] = error
            finally:
                control.stop()
                finished.set()

        @capture.event
        def on_closed() -> None:
            finished.set()

        control = capture.start_free_threaded()
        if not finished.wait(3):
            control.stop()
            raise RuntimeError("Windows Graphics Capture timed out")
        if "error" in result:
            raise RuntimeError(f"Windows Graphics Capture failed: {result['error']}")
        image = result.get("image")
        if image is None:
            raise RuntimeError("Windows Graphics Capture closed without a frame")
        if region is not None:
            image = self._crop_client(hwnd, image, region)
        return Frame.from_image(image, "wgc")

    @staticmethod
    def _crop_client(hwnd: int, image: Any, client: Rect) -> Any:
        bounds = wintypes.RECT()
        dwmapi = ctypes.WinDLL("dwmapi", use_last_error=True)
        dwmapi.DwmGetWindowAttribute.argtypes = [
            wintypes.HWND,
            wintypes.DWORD,
            ctypes.c_void_p,
            wintypes.DWORD,
        ]
        dwmapi.DwmGetWindowAttribute.restype = ctypes.c_long
        result = dwmapi.DwmGetWindowAttribute(
            hwnd,
            9,
            ctypes.byref(bounds),
            ctypes.sizeof(bounds),
        )
        if result != 0:
            raise OSError(f"DwmGetWindowAttribute failed with HRESULT 0x{result & 0xFFFFFFFF:08X}")
        left = client.left - bounds.left
        top = client.top - bounds.top
        right = left + client.width
        bottom = top + client.height
        height, width = image.shape[:2]
        if left < 0 or top < 0 or right > width or bottom > height:
            raise RuntimeError(
                f"client crop {left},{top},{right},{bottom} exceeds captured frame {width}x{height}"
            )
        return image[top:bottom, left:right]
