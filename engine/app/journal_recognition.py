"""One-shot, read-only journal recognition; never constructs AutomationEngine.

The desktop owns this process. Cancellation terminates only this helper, leaving
the automation process, paused tasks and game window unchanged.
"""
from __future__ import annotations

import base64
import json
import sys
from pathlib import Path
from typing import Callable

import cv2
import numpy as np

from capture.wgc_capture import WindowsGraphicsCapture
from config.schema import EngineConfig
from ipc.messages import event
from vision.journal_matcher import MAX_PIXELS, prepare_template, recognize
from window.windows import WindowsWindowManager


def capture_window(config: EngineConfig, window_handle: int | None = None) -> np.ndarray:
    # No layout, focus, input, capture-manager fallback, or debug-file methods.
    window = WindowsWindowManager(config.window).find_window()
    if window is None:
        raise ValueError("请先打开游戏窗口 / Open the game window first")
    if window_handle is not None and (window.hwnd != window_handle or not window.is_foreground):
        raise ValueError("游戏前台窗口已改变，请重新识别 / The foreground game window changed; try again")
    if window.is_minimized or not window.is_responding:
        raise ValueError("请恢复游戏窗口并等待响应 / Restore the game window and wait for it to respond")
    if window.client_rect.width * window.client_rect.height > MAX_PIXELS:
        raise ValueError("游戏窗口分辨率过大 / Game window exceeds the capture size limit")
    return WindowsGraphicsCapture().capture(window.client_rect, window.hwnd).image


def run_recognition(request: dict, config: EngineConfig,
                    emit: Callable[..., None], capture=capture_window) -> dict:
    items = request.get("templates")
    if not isinstance(items, list) or not items:
        raise ValueError("当前分类没有可识别的一楼地图 / No eligible first-floor maps in this category")
    emit("journal_recognition_progress", stage="capturing", completed=0, total=0)
    frame = capture(config, request.get("window_handle"))
    warnings: list[str] = []

    def templates():
        for item in items:
            try:
                path = Path(item["path"])
                # Rust supplies paths validated against the imported-pack root.
                with path.open("rb") as source:
                    data = source.read(20 * 1024 * 1024 + 1)
                if not data or len(data) > 20 * 1024 * 1024:
                    raise ValueError("image exceeds file-size limit")
                image = cv2.imdecode(np.frombuffer(data, np.uint8), cv2.IMREAD_UNCHANGED)
                if image is None:
                    raise ValueError("image could not be decoded")
                yield prepare_template(item["key"], image)
            except (OSError, ValueError, cv2.error) as error:
                warnings.append(f"{item['key']}: {error}")

    matches = recognize(frame, templates(), template_count=len(items),
                        progress=lambda stage, completed, total: emit(
                            "journal_recognition_progress", stage=stage, completed=completed, total=total))
    preview = None
    if matches:
        crop = matches[0].region.crop(frame)
        ratio = min(1, 600 / max(crop.shape[:2]))
        crop = cv2.resize(crop, (max(1, round(crop.shape[1] * ratio)),
                                 max(1, round(crop.shape[0] * ratio))))
        ok, encoded = cv2.imencode(".jpg", crop, [cv2.IMWRITE_JPEG_QUALITY, 85])
        if ok:
            preview = "data:image/jpeg;base64," + base64.b64encode(encoded).decode("ascii")
    return {"keys": [match.key for match in matches], "preview": preview, "warnings": warnings}


def main(config: EngineConfig) -> int:
    request_id = ""

    def emit(kind: str, **payload: object) -> None:
        print(json.dumps(event(kind, request_id=request_id, **payload), ensure_ascii=False), flush=True)

    try:
        request = json.loads(sys.stdin.readline(4 * 1024 * 1024))
        request_id = request["request_id"]
        result = run_recognition(request, config, emit)
        emit("journal_recognition_result", **result)
        return 0
    except Exception as error:
        emit("error", message=str(error))
        return 1
