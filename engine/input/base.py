from abc import ABC, abstractmethod

from core.models import Point


class InputBackend(ABC):
    @abstractmethod
    def move_mouse(self, point: Point) -> None: ...

    @abstractmethod
    def mouse_down(self, button: str = "left") -> None: ...

    @abstractmethod
    def mouse_up(self, button: str = "left") -> None: ...

    @abstractmethod
    def key_down(self, key: str) -> None: ...

    @abstractmethod
    def key_up(self, key: str) -> None: ...

    def click(self, point: Point, button: str = "left") -> None:
        self.move_mouse(point)
        self.mouse_down(button)
        self.mouse_up(button)

    def press(self, key: str) -> None:
        self.key_down(key)
        self.key_up(key)
