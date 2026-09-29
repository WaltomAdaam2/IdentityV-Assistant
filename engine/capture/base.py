from abc import ABC, abstractmethod
from dataclasses import dataclass
from time import monotonic
from typing import Any

from core.models import Rect


@dataclass(frozen=True)
class Frame:
    image: Any
    timestamp: float
    width: int
    height: int
    source: str

    @classmethod
    def from_image(cls, image: Any, source: str) -> "Frame":
        height, width = image.shape[:2]
        return cls(image=image, timestamp=monotonic(), width=width, height=height, source=source)


class CaptureBackend(ABC):
    @abstractmethod
    def capture(self, region: Rect | None = None) -> Frame: ...
