from __future__ import annotations

import asyncio
from contextlib import suppress
from datetime import datetime, timezone
from pathlib import Path
from time import perf_counter
from typing import Any
from uuid import uuid4

from app.context import AutomationContext
from capture.manager import CaptureManager
from config.loader import parse_config
from config.schema import EngineConfig
from core.enums import EngineStatus, ErrorCode, TaskFinishReason
from core.models import CoordinateSpace, Point, Rect
from input.controller import InputController
from input.windows_message import WindowsMessageInput
from ipc.messages import ENGINE_VERSION, event
from ipc.reader import NdjsonReader
from ipc.writer import NdjsonWriter
from runtime.cancellation import CancellationToken
from runtime.metrics import RuntimeMetrics
from runtime.memory_guard import MemoryGuard
from states.history import StateHistory
from tasks.base import AutomationTask, TaskControl
from tasks.login_lobby_task import LoginLobbyTask
from window.base import WindowInfo
from window.windows import WindowsWindowManager


class AutomationEngine:
    def __init__(self, config: EngineConfig) -> None:
        self.config = config
        self.writer = NdjsonWriter()
        self.reader = NdjsonReader()
        self.status = EngineStatus.STARTING
        self.task_status = "idle"
        self._shutdown = asyncio.Event()
        self._task_control: TaskControl | None = None
        self._task_runner: asyncio.Task[None] | None = None
        self._task_queue: list[str] = []
        self._developer_input_enabled = False
        self._window_health: str | None = None
        self._memory_pressure = False
        window = WindowsWindowManager(config.window)
        self.context = AutomationContext(
            config,
            window,
            CaptureManager(config.capture),
            InputController(
                WindowsMessageInput(window.current_hwnd),
                position_jitter_px=config.automation.click_position_jitter_px,
                click_duration_jitter_ms=config.automation.click_duration_jitter_ms,
            ),
            RuntimeMetrics(),
            CancellationToken(),
            StateHistory(config.vision.state_confirmation_count),
            self.emit,
            MemoryGuard(config.memory_protection),
        )

    async def emit(self, event_type: str, **payload: object) -> None:
        await self.writer.send(event(event_type, **payload))

    async def run(self) -> None:
        self.status = EngineStatus.IDLE
        await self.emit(
            "ready",
            engine_version=ENGINE_VERSION,
            heartbeat_interval_seconds=self.config.heartbeat_interval_seconds,
        )
        heartbeat = asyncio.create_task(self._heartbeat_loop())
        try:
            async for command in self.reader.messages():
                await self._handle_command(command)
                if self._shutdown.is_set():
                    break
        finally:
            self.status = EngineStatus.STOPPING
            await self._stop_task(TaskFinishReason.USER_STOPPED)
            self.context.input.release_all()
            heartbeat.cancel()
            with suppress(asyncio.CancelledError):
                await heartbeat
            self.status = EngineStatus.STOPPED

    async def _heartbeat_loop(self) -> None:
        while not self._shutdown.is_set():
            window = self.context.window.find_window()
            health = self._window_health_payload(window)
            await self.emit(
                "heartbeat",
                engine_status=self.status.value,
                task_status=self.task_status,
                **health,
            )
            if health["window_status"] != self._window_health:
                self._window_health = str(health["window_status"])
                messages = {
                    "ready": "Game window is ready",
                    "minimized": "Game window is minimized",
                    "unresponsive": "Game window is not responding",
                    "closed": "Game window is not open",
                }
                await self.emit(
                    "window_health",
                    level="info" if self._window_health == "ready" else "warning",
                    source="window",
                    message=messages[self._window_health],
                    data={"important": True},
                    **health,
                )
            await self.emit("metrics", **self.context.metrics.snapshot())
            try:
                await asyncio.wait_for(self._shutdown.wait(), timeout=self.config.heartbeat_interval_seconds)
            except TimeoutError:
                continue

    async def _handle_command(self, command: dict) -> None:
        command_type = command["type"]
        request_id = command.get("request_id") if isinstance(command.get("request_id"), str) else None
        if command_type == "ping":
            window = self.context.window.find_window()
            await self.emit(
                "heartbeat",
                engine_status=self.status.value,
                task_status=self.task_status,
                **self._window_health_payload(window),
            )
        elif command_type == "start_task":
            await self._start_task(str(command.get("task", "login_lobby")))
        elif command_type == "start_tasks":
            names = command.get("tasks")
            if not isinstance(names, list) or not names or any(not isinstance(name, str) for name in names):
                await self.emit("error", code="TASK_UNKNOWN", message="Expected a nonempty task list", recoverable=True, details={})
            elif any(name not in self._task_factories() for name in names):
                await self.emit("error", code="TASK_UNKNOWN", message="Unknown task in queue", recoverable=True, details={})
            elif self._task_runner and not self._task_runner.done():
                await self.emit("error", code="TASK_ALREADY_RUNNING", message="A task is already running", recoverable=True, details={})
            else:
                self._task_queue = names[1:]
                await self._start_task(names[0], preserve_queue=True)
        elif command_type == "pause_task" and self._task_control:
            self._task_control.pause()
            self.status, self.task_status = EngineStatus.PAUSED, "paused"
            self.context.input.release_all()
        elif command_type == "resume_task" and self._task_control:
            self._task_control.resume()
            self.status, self.task_status = EngineStatus.RUNNING, "running"
        elif command_type == "stop_task":
            await self._stop_task(TaskFinishReason.USER_STOPPED)
        elif command_type == "detect_window":
            await self._detect_window(request_id)
        elif command_type == "capture_frame":
            await self._capture_frame(request_id)
        elif command_type == "test_template":
            await self._test_template(command, request_id)
        elif command_type == "input_test":
            await self._input_test(command, request_id)
        elif command_type == "reload_settings":
            await self._reload_settings(command)
        elif command_type == "shutdown":
            self._shutdown.set()

    async def _reload_settings(self, command: dict) -> None:
        settings = command.get("settings")
        if not isinstance(settings, dict):
            await self.emit(
                "error",
                code="CONFIG_INVALID",
                message="Settings must be an object",
                recoverable=True,
                details={},
            )
            return
        try:
            config = parse_config(settings)
        except ValueError as error:
            await self.emit(
                "error",
                code="CONFIG_INVALID",
                message=str(error),
                recoverable=True,
                details={},
            )
            return

        previous = self.config
        self.config = config
        self.context.config = config
        self.context.window.config = config.window
        self.context.input.position_jitter_px = config.automation.click_position_jitter_px
        self.context.input.click_duration_jitter_ms = config.automation.click_duration_jitter_ms
        self.context.memory_guard.enabled = config.memory_protection
        self.context.state_history = StateHistory(config.vision.state_confirmation_count)

        if previous.capture != config.capture:
            self.context.capture.config = config.capture
            self.context.capture.backend = None
            self.context.capture.backend_name = "not_initialized"
            self.context.capture.last_error = None

        await self.emit("settings_reloaded", level="info", source="settings")

    async def _detect_window(self, request_id: str | None = None) -> None:
        window = self.context.window.find_window()
        if window is None:
            await self.emit(
                "window_diagnostics",
                found=False,
                hwnd=None,
                title=None,
                process_id=None,
                process_name=None,
                class_name=None,
                client_rect=None,
                dpi_scale=None,
                is_minimized=None,
                is_foreground=None,
                match_reasons=[],
                capture_backend=self.context.capture.backend_name,
                capture_backend_configured=self.config.capture.backend,
                capture_last_error=self.context.capture.last_error,
                request_id=request_id,
            )
            await self.emit("error", code=ErrorCode.WINDOW_NOT_FOUND.value, message="Configured game window was not found", recoverable=True, details={"process_names": self.config.window.process_names, "title_contains": self.config.window.title_contains, "class_names": self.config.window.class_names}, request_id=request_id)
            return
        await self.emit("window_diagnostics", **self._window_payload(window, request_id))

    async def _capture_frame(self, request_id: str | None = None) -> None:
        window = self.context.window.find_window()
        if window is None:
            await self.emit("error", code=ErrorCode.WINDOW_NOT_FOUND.value, message="Cannot capture before the configured game window is found", recoverable=True, details={"operation": "capture_frame"}, request_id=request_id)
            return
        if window.is_minimized:
            await self.emit("window_diagnostics", **self._window_payload(window, request_id))
            await self.emit("error", code=ErrorCode.WINDOW_INVALID.value, message="Cannot capture a minimized game window", recoverable=True, details={"hwnd": window.hwnd, "operation": "capture_frame"}, request_id=request_id)
            return
        started = perf_counter()
        try:
            await self._protect_memory()
            frame = self.context.capture.capture(window.client_rect, window.hwnd)
            self.context.metrics.screenshots += 1
            captured_at = datetime.now(timezone.utc)
            path = self._save_debug_frame(frame.image, captured_at)
        except Exception as error:
            await self.emit("window_diagnostics", **self._window_payload(window, request_id))
            await self.emit("error", code=ErrorCode.CAPTURE_FAILED.value, message=str(error), recoverable=True, details={"hwnd": window.hwnd, "region": self._rect_payload(window.client_rect), "capture_backend_configured": self.config.capture.backend, "capture_backend_active": self.context.capture.backend_name, "capture_last_error": self.context.capture.last_error}, request_id=request_id)
            return
        await self.emit("window_diagnostics", **self._window_payload(window, request_id))
        await self.emit(
            "debug_frame",
            path=str(path),
            width=frame.width,
            height=frame.height,
            state=self.context.state_history.current.value,
            source=frame.source,
            captured_at=captured_at.isoformat(),
            capture_ms=round((perf_counter() - started) * 1000, 2),
            file_size=path.stat().st_size,
            region=self._rect_payload(window.client_rect),
            hwnd=window.hwnd,
            request_id=request_id,
        )

    @staticmethod
    def _rect_payload(rect: Rect) -> dict[str, int]:
        return {"left": rect.left, "top": rect.top, "width": rect.width, "height": rect.height}

    def _window_payload(self, window: WindowInfo, request_id: str | None = None) -> dict[str, object]:
        return {
            "found": True,
            "hwnd": window.hwnd,
            "title": window.title,
            "process_id": window.process_id,
            "process_name": window.process_name,
            "class_name": window.class_name,
            "client_rect": self._rect_payload(window.client_rect),
            "dpi_scale": window.dpi_scale,
            "is_minimized": window.is_minimized,
            "is_foreground": window.is_foreground,
            "is_responding": window.is_responding,
            "match_reasons": window.match_reasons,
            "capture_backend": self.context.capture.backend_name,
            "capture_backend_configured": self.config.capture.backend,
            "capture_last_error": self.context.capture.last_error,
            "request_id": request_id,
        }

    def _window_health_payload(self, window: WindowInfo | None) -> dict[str, object]:
        if window is None:
            return {
                "window_status": "closed",
                "found": False,
                "hwnd": None,
                "process_id": None,
                "is_minimized": None,
                "is_responding": None,
            }
        status = (
            "unresponsive"
            if not window.is_responding
            else "minimized"
            if window.is_minimized
            else "ready"
        )
        return {
            "window_status": status,
            "found": True,
            "hwnd": window.hwnd,
            "title": window.title,
            "process_id": window.process_id,
            "process_name": window.process_name,
            "class_name": window.class_name,
            "client_rect": self._rect_payload(window.client_rect),
            "dpi_scale": window.dpi_scale,
            "is_minimized": window.is_minimized,
            "is_foreground": window.is_foreground,
            "is_responding": window.is_responding,
            "match_reasons": window.match_reasons,
            "capture_backend": self.context.capture.backend_name,
            "capture_backend_configured": self.config.capture.backend,
            "capture_last_error": self.context.capture.last_error,
        }

    async def _test_template(self, command: dict[str, Any], request_id: str | None = None) -> None:
        template_value = command.get("template")
        if not isinstance(template_value, str) or not template_value:
            await self.emit("error", code=ErrorCode.TEMPLATE_NOT_FOUND.value, message="A template path is required", recoverable=True, details={}, request_id=request_id)
            return
        template_root = Path(__file__).resolve().parents[2] / "assets" / "templates"
        template_path = (template_root / template_value).resolve()
        if not template_path.is_relative_to(template_root.resolve()) or not template_path.is_file():
            await self.emit("error", code=ErrorCode.TEMPLATE_NOT_FOUND.value, message="Template must be an existing file under assets/templates", recoverable=True, details={"template": template_value}, request_id=request_id)
            return
        window = self.context.window.find_window()
        if window is None:
            await self.emit("error", code=ErrorCode.WINDOW_NOT_FOUND.value, message="Cannot test a template before the configured game window is found", recoverable=True, details={"operation": "test_template"}, request_id=request_id)
            return
        if window.is_minimized:
            await self.emit("window_diagnostics", **self._window_payload(window, request_id))
            await self.emit("error", code=ErrorCode.WINDOW_INVALID.value, message="Cannot test a template against a minimized game window", recoverable=True, details={"hwnd": window.hwnd, "operation": "test_template"}, request_id=request_id)
            return
        try:
            import cv2
            from vision.template_matcher import TemplateMatcher
            await self._protect_memory()
            frame = self.context.capture.capture(window.client_rect, window.hwnd)
            self.context.metrics.screenshots += 1
            template = cv2.imread(str(template_path), cv2.IMREAD_UNCHANGED)
            match = TemplateMatcher().match(frame.image, template, self.config.vision.template_threshold)
        except Exception as error:
            await self.emit("error", code=ErrorCode.CAPTURE_FAILED.value, message=str(error), recoverable=True, details={"template": template_value, "hwnd": window.hwnd, "region": self._rect_payload(window.client_rect), "capture_backend_active": self.context.capture.backend_name}, request_id=request_id)
            return
        await self.emit("window_diagnostics", **self._window_payload(window, request_id))
        await self.emit("template_match", template=template_value, found=match.found, confidence=match.confidence, bbox=match.bbox, threshold=self.config.vision.template_threshold, request_id=request_id)

    async def _protect_memory(self) -> None:
        percent = self.context.memory_guard.protect()
        if percent is None:
            return
        under_pressure = percent > self.context.memory_guard.threshold_percent
        if under_pressure and not self._memory_pressure:
            await self.emit(
                "log",
                level="warning",
                source="memory",
                message=f"System memory usage reached {percent:.1f}%; unused caches were released",
                data={"important": True, "memory_percent": percent},
            )
        self._memory_pressure = under_pressure

    async def _input_test(self, command: dict[str, Any], request_id: str | None = None) -> None:
        action = command.get("action")
        if not isinstance(action, dict):
            await self.emit("error", code=ErrorCode.INPUT_FAILED.value, message="input_test action must be an object", recoverable=True, details={}, request_id=request_id)
            return
        if action.get("type") == "set_enabled":
            self._developer_input_enabled = bool(action.get("enabled"))
            await self.emit("planned_action", action="input_test_enabled", input="none", target=None, enabled=self._developer_input_enabled, request_id=request_id)
            return
        if not self._developer_input_enabled:
            await self.emit("error", code=ErrorCode.INPUT_FAILED.value, message="Developer input test mode is disabled", recoverable=True, details={}, request_id=request_id)
            return
        try:
            if action.get("type") == "click":
                window = self.context.window.find_window()
                if window is None:
                    raise RuntimeError("Configured game window was not found")
                reference = Point(int(action["x"]), int(action["y"]))
                point = CoordinateSpace().to_client(reference, window.client_rect)
                screen = Point(window.client_rect.left + point.x, window.client_rect.top + point.y)
                self.context.input.click(point)
                await self.emit("planned_action", action="input_test_click", input="click", target={"x": point.x, "y": point.y, "screen_x": screen.x, "screen_y": screen.y, "reference_x": reference.x, "reference_y": reference.y, "hwnd": window.hwnd}, request_id=request_id)
            elif action.get("type") == "press":
                key = str(action["key"])
                self.context.input.press(key)
                await self.emit("planned_action", action="input_test_press", input="key", target={"key": key}, request_id=request_id)
            else:
                raise ValueError("input_test supports click and press actions")
        except (KeyError, TypeError, ValueError, RuntimeError) as error:
            await self.emit("error", code=ErrorCode.INPUT_FAILED.value, message=str(error), recoverable=True, details={}, request_id=request_id)

    def _save_debug_frame(self, image: Any, captured_at: datetime) -> Path:
        import cv2

        capture_id = f"{captured_at:%Y%m%dT%H%M%S%fZ}-{uuid4().hex[:8]}"
        path = Path(__file__).resolve().parents[2] / "debug" / "captures" / f"{capture_id}.png"
        path.parent.mkdir(parents=True, exist_ok=True)
        if not cv2.imwrite(str(path), image):
            raise RuntimeError("could not save debug frame")
        return path

    @staticmethod
    def _task_factories() -> dict[str, type[AutomationTask]]:
        return {"login_lobby": LoginLobbyTask}

    async def _start_task(self, task_name: str, preserve_queue: bool = False) -> None:
        if self._task_runner and not self._task_runner.done():
            await self.emit("error", code="TASK_ALREADY_RUNNING", message="A task is already running", recoverable=True, details={})
            return
        factories = self._task_factories()
        if task_name not in factories:
            await self.emit("error", code="TASK_UNKNOWN", message=f"Unknown task: {task_name}", recoverable=True, details={})
            return
        if not preserve_queue:
            self._task_queue.clear()
        self._task_control = TaskControl()
        self.status, self.task_status = EngineStatus.RUNNING, "running"
        await self.emit("task_started", task_id=task_name)
        self._task_runner = asyncio.create_task(self._run_task(factories[task_name]()))

    async def _run_task(self, task: AutomationTask) -> None:
        try:
            reason = await task.run(self.context, self._task_control or TaskControl())
        except Exception as error:
            self.status, self.task_status = EngineStatus.ERROR, "error"
            await self.emit("error", code=ErrorCode.ENGINE_CRASHED.value, message=str(error), recoverable=False, details={})
            reason = TaskFinishReason.ENGINE_ERROR
        finally:
            self.context.input.release_all()
        if self.status != EngineStatus.ERROR:
            self.status, self.task_status = EngineStatus.IDLE, "idle"
        await self.emit("task_finished", reason=reason.value, stats=self.context.metrics.snapshot())
        if reason == TaskFinishReason.COMPLETED and self._task_queue:
            next_name = self._task_queue.pop(0)
            self._task_control = TaskControl()
            self.status, self.task_status = EngineStatus.RUNNING, "running"
            await self.emit("task_started", task_id=next_name)
            self._task_runner = asyncio.create_task(self._run_task(self._task_factories()[next_name]()))
        else:
            self._task_queue.clear()

    async def _stop_task(self, reason: TaskFinishReason) -> None:
        self._task_queue.clear()
        if not self._task_runner or self._task_runner.done():
            return
        if self._task_control:
            self._task_control.cancel()
        with suppress(asyncio.CancelledError):
            await self._task_runner
        self.context.input.release_all()
        self.status, self.task_status = EngineStatus.IDLE, "idle"
