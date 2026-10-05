from __future__ import annotations

import random
from collections.abc import Callable
from time import sleep

from core.models import Point, Rect
from input.base import InputBackend


class InputController:
    def __init__(
        self,
        backend: InputBackend,
        on_action: Callable[[dict], None] | None = None,
        position_jitter_px: int = 0,
        click_duration_jitter_ms: int = 0,
    ) -> None:
        self.backend = backend
        self.on_action = on_action
        self.position_jitter_px = position_jitter_px
        self.click_duration_jitter_ms = click_duration_jitter_ms
        self.pressed_keys: set[str] = set()
        self.mouse_buttons_down: set[str] = set()
        self._key_backends: dict[str, InputBackend] = {}
        self._mouse_backends: dict[str, InputBackend] = {}

    def _record(self, action: str, **details: object) -> None:
        if self.on_action:
            self.on_action({"action": action, **details})

    def click(self, point: Point, button: str = "left", bounds: Rect | None = None) -> None:
        actual = self._jitter_point(point, bounds)
        duration_ms = max(
            80.0,
            100.0 + random.uniform(-self.click_duration_jitter_ms, self.click_duration_jitter_ms),
        )
        self._record(
            "click",
            input="click",
            target={"x": actual.x, "y": actual.y},
            requested_target={"x": point.x, "y": point.y},
            button=button,
            duration_ms=round(duration_ms, 1),
        )
        self.backend.move_mouse(actual)
        try:
            self.mouse_down(button)
            sleep(duration_ms / 1000)
        finally:
            self.mouse_up(button)

    def _jitter_point(self, point: Point, bounds: Rect | None = None) -> Point:
        if self.position_jitter_px <= 0:
            return point
        # Sample inside the allowed rectangle, rather than accumulating points
        # at its edges by clamping a larger random sample. Screen coordinates
        # may be negative on monitors to the left/above the primary display.
        left, right = point.x - self.position_jitter_px, point.x + self.position_jitter_px
        top, bottom = point.y - self.position_jitter_px, point.y + self.position_jitter_px
        if bounds is not None:
            left, right = max(left, bounds.left), min(right, bounds.right - 1)
            top, bottom = max(top, bounds.top), min(bottom, bounds.bottom - 1)
        return Point(
            random.randint(left, right),
            random.randint(top, bottom),
        )

    def press(self, key: str) -> None:
        self._record("press", input="key", key=key)
        if any(held.casefold() == key.casefold() for held in self.pressed_keys):
            raise RuntimeError(f"按键 {key} 正在被其他操作按住，已阻止重复点击")
        try:
            self.key_down(key)
            sleep(0.10)
        finally:
            self.key_up(key)

    def key_down(self, key: str) -> None:
        self._record("key_down", input="key", key=key)
        self.pressed_keys.add(key)
        self._key_backends[key] = self.backend
        self.backend.key_down(key)

    def key_up(self, key: str) -> None:
        self._record("key_up", input="key", key=key)
        self._key_backends.get(key, self.backend).key_up(key)
        self.pressed_keys.discard(key)
        self._key_backends.pop(key, None)

    def mouse_down(self, button: str = "left") -> None:
        self._record("mouse_down", input="mouse", button=button)
        self.mouse_buttons_down.add(button)
        self._mouse_backends[button] = self.backend
        self.backend.mouse_down(button)

    def mouse_up(self, button: str = "left") -> None:
        self._record("mouse_up", input="mouse", button=button)
        self._mouse_backends.get(button, self.backend).mouse_up(button)
        self.mouse_buttons_down.discard(button)
        self._mouse_backends.pop(button, None)

    def release_all(self) -> None:
        first_error = None
        for key in sorted(self.pressed_keys, key=lambda key: key.upper() == "ALT"):
            try:
                self.key_up(key)
            except Exception as error:
                first_error = first_error or error
        for button in tuple(self.mouse_buttons_down):
            try:
                self.mouse_up(button)
            except Exception as error:
                first_error = first_error or error
        self._record("release_all", input="cleanup")
        if first_error:
            raise first_error
