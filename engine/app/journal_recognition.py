"""One-shot, read-only journal recognition; never constructs AutomationEngine.

The desktop owns this process. Cancellation terminates only this helper, leaving
the automation process, paused tasks and game window unchanged.
"""
from __future__ import annotations

import base64
import json
import sys
from pathlib import Path
from time import perf_counter
from typing import Callable

import cv2
import numpy as np

from capture.wgc_capture import WindowsGraphicsCapture
from config.loader import load_config
from config.schema import EngineConfig
from ipc.messages import event
from vision.journal_difficulty import detect_difficulty, detect_s1_floor
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
                    emit: Callable[..., None], capture=capture_window, load_templates=None) -> dict:
    started = perf_counter()
    context = {"season": request.get("season", "s0"),
               "difficulty": request.get("difficulty") if request.get("difficulty") in ("hard", "nightmare") else None,
               "floor": "1" if request.get("season", "s0") == "s0" and request.get("difficulty") != "auto" else None}

    def progress(stage, completed, total):
        emit("journal_recognition_progress", stage=stage, completed=completed, total=total,
             elapsed_ms=round((perf_counter() - started) * 1000), **context)

    def diagnostic_image(kind, image):
        # Dedicated image messages are persisted separately, never as text logs.
        ok, encoded = cv2.imencode(".png", image, [cv2.IMWRITE_PNG_COMPRESSION, 1])
        if ok:
            emit("journal_recognition_image", image_kind=kind,
                 width=image.shape[1], height=image.shape[0],
                 elapsed_ms=round((perf_counter() - started) * 1000),
                 png=base64.b64encode(encoded).decode("ascii"))

    def diagnostic(details):
        emit("journal_recognition_diagnostic", **details)

    def preview_region(region):
        value = make_preview(region.crop(frame))
        if value:
            emit("journal_recognition_preview", preview=value, preview_kind="region", region=vars(region))

    items = request.get("templates")
    deferred = request.get("defer_templates", False)
    if not deferred and "templates_by_difficulty" not in request and (not isinstance(items, list) or not items):
        raise ValueError("当前分类没有可识别的一楼地图 / No eligible first-floor maps in this category")
    progress("capturing", 0, 0)
    capture_started = perf_counter()
    frame = capture(config, request.get("window_handle"))
    diagnostic({"stage": "captured", "elapsed_ms": round((perf_counter() - started) * 1000),
                "capture_ms": round((perf_counter() - capture_started) * 1000)})
    preview = make_preview(frame)
    if preview:
        emit("journal_recognition_preview", preview=preview, preview_kind="full")
    diagnostic_image("full", frame)
    difficulty = None
    floor = "1"
    if deferred or "templates_by_difficulty" in request:
        if request.get("season") == "s1":
            difficulty = request.get("difficulty")
            if difficulty not in ("hard", "nightmare"):
                raise ValueError("请选择 S1 地图难度 / Select the S1 map difficulty")
            diagnostic({"stage": "difficulty_selected", "difficulty": difficulty, "mode": "manual", "season": "s1"})
            progress("floor", 0, 0)
            floor, evidence = detect_s1_floor(frame)
            diagnostic({"stage": "floor_detected", **evidence, "floor": floor})
            context["floor"] = floor
            progress("floor", 0, 0)
        elif request.get("difficulty", "auto") == "auto":
            progress("difficulty", 0, 0)
            difficulty, evidence = detect_difficulty(frame)
            diagnostic({"stage": "difficulty_detected", "difficulty": difficulty, **evidence})
            context.update(difficulty=difficulty, floor=floor)
            progress("difficulty", 0, 0)
        else:
            difficulty = request.get("difficulty")
            if difficulty not in ("hard", "nightmare"):
                raise ValueError("请选择地图难度 / Select a map difficulty")
            diagnostic({"stage": "difficulty_selected", "difficulty": difficulty, "mode": "manual", "season": "s0"})
        pools = load_templates() if deferred else request["templates_by_difficulty"]
        items = pools.get(difficulty, [])
        if request.get("season") == "s1":
            items = [item for item in items if item.get("floor") == floor]
        if not items:
            raise ValueError(f"所选难度缺少地下{floor}层地图，请导入对应地图包 / No B{floor} maps for the selected difficulty" if request.get("season") == "s1" else "检测到的难度缺少一楼地图，请导入对应地图包 / No first-floor maps for the detected difficulty")
    diagnostic({"stage": "indexed", "difficulty": difficulty, "floor":floor, "template_count": len(items),
                "template_keys": [item["key"] for item in items]})
    warnings: list[str] = []

    def templates():
        for index, item in enumerate(items):
            template_started = perf_counter()
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
                template = prepare_template(item["key"], image)
                diagnostic({"stage": "template_prepared", "key": item["key"], "image": f"template_{index}.png",
                            "elapsed_ms": round((perf_counter() - template_started) * 1000)})
                diagnostic_image(f"template_{index}", template.edges)
                yield template
            except (OSError, ValueError, cv2.error) as error:
                warnings.append(f"{item['key']}: {error}")
                diagnostic({"stage": "template_rejected", "key": item["key"], "reason": str(error)})

    matches = recognize(frame, templates(), template_count=len(items), progress=progress,
                        diagnostic=diagnostic, image=diagnostic_image, preview=preview_region)
    preview = None
    if matches:
        crop = matches[0].region.crop(frame)
        diagnostic_image("region", crop)
        preview = make_preview(crop)
    return {"difficulty": difficulty, "floor": floor, "keys": [match.key for match in matches], "preview": preview, "warnings": warnings,
            "elapsed_ms": round((perf_counter() - started) * 1000),
            "matches": [{"key": match.key, "region": vars(match.region), "geometry_cost": match.score,
                         "coverage": match.coverage, "unexplained": match.unexplained}
                        for match in matches]}


def make_preview(crop):
    ratio = min(1, 600 / max(crop.shape[:2]))
    crop = cv2.resize(crop, (max(1, round(crop.shape[1] * ratio)),
                             max(1, round(crop.shape[0] * ratio))))
    ok, encoded = cv2.imencode(".jpg", crop, [cv2.IMWRITE_JPEG_QUALITY, 85])
    return "data:image/jpeg;base64," + base64.b64encode(encoded).decode("ascii") if ok else None


def main(config_path: Path) -> int:
    request_id = ""

    def emit(kind: str, **payload: object) -> None:
        print(json.dumps(event(kind, request_id=request_id, **payload), ensure_ascii=False), flush=True)

    try:
        # Load the native library while idle, without opening a capture session.
        WindowsGraphicsCapture()
        emit("journal_recognition_ready")
        request = json.loads(sys.stdin.readline(4 * 1024 * 1024))
        request_id = request["request_id"]
        # A prepared helper may have waited through settings changes.
        config = load_config(config_path)

        def load_templates():
            message = json.loads(sys.stdin.readline(4 * 1024 * 1024))
            if message.get("request_id") != request_id:
                raise ValueError("Recognition template request ID mismatch")
            return message["templates_by_difficulty"]

        result = run_recognition(request, config, emit, load_templates=load_templates)
        emit("journal_recognition_result", **result)
        return 0
    except Exception as error:
        emit("error", message=str(error))
        return 1
