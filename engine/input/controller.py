from __future__ import annotations

import random
from collections.abc import Callable
from time import sleep

from core.models import Point
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

    def _record(self, action: str, **details: object) -> None:
        if self.on_action:
            self.on_action({"action": action, **details})

    def click(self, point: Point, button: str = "left") -> None:
        actual = self._jitter_point(point)
        duration_ms = max(
            10.0,
            35.0 + random.uniform(-self.click_duration_jitter_ms, self.click_duration_jitter_ms),
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
        self.backend.mouse_down(button)
        try:
            sleep(duration_ms / 1000)
        finally:
            self.backend.mouse_up(button)

    def _jitter_point(self, point: Point) -> Point:
        if self.position_jitter_px <= 0:
            return point
        return Point(
            max(0, point.x + random.randint(-self.position_jitter_px, self.position_jitter_px)),
            max(0, point.y + random.randint(-self.position_jitter_px, self.position_jitter_px)),
        )

    def press(self, key: str) -> None:
        self._record("press", input="key", key=key)
        self.backend.press(key)

    def key_down(self, key: str) -> None:
        self._record("key_down", input="key", key=key)
        self.pressed_keys.add(key)
        self.backend.key_down(key)

    def key_up(self, key: str) -> None:
        self._record("key_up", input="key", key=key)
        self.pressed_keys.discard(key)
        self.backend.key_up(key)

    def mouse_down(self, button: str = "left") -> None:
        self._record("mouse_down", input="mouse", button=button)
        self.mouse_buttons_down.add(button)
        self.backend.mouse_down(button)

    def mouse_up(self, button: str = "left") -> None:
        self._record("mouse_up", input="mouse", button=button)
        self.mouse_buttons_down.discard(button)
        self.backend.mouse_up(button)

    def release_all(self) -> None:
        first_error = None
        for key in tuple(self.pressed_keys):
            try:
                self.backend.key_up(key)
                self.pressed_keys.discard(key)
            except Exception as error:
                first_error = first_error or error
        for button in tuple(self.mouse_buttons_down):
            try:
                self.backend.mouse_up(button)
                self.mouse_buttons_down.discard(button)
            except Exception as error:
                first_error = first_error or error
        self._record("release_all", input="cleanup")
        if first_error:
            raise first_error
