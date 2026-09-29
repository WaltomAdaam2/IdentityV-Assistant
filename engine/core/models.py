from dataclasses import dataclass, field
from typing import Any


@dataclass(frozen=True)
class Rect:
    left: int
    top: int
    width: int
    height: int

    @property
    def right(self) -> int:
        return self.left + self.width

    @property
    def bottom(self) -> int:
        return self.top + self.height


@dataclass(frozen=True)
class Point:
    x: int
    y: int


@dataclass(frozen=True)
class CoordinateSpace:
    base_width: int = 1920
    base_height: int = 1080

    def to_client(self, point: Point, client_rect: Rect) -> Point:
        return Point(round(point.x * client_rect.width / self.base_width), round(point.y * client_rect.height / self.base_height))

    def to_screen(self, point: Point, client_rect: Rect) -> Point:
        client = self.to_client(point, client_rect)
        return Point(client_rect.left + client.x, client_rect.top + client.y)


@dataclass
class EngineError:
    code: str
    message: str
    recoverable: bool
    details: dict[str, Any] = field(default_factory=dict)
