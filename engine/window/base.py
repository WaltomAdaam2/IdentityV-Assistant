from abc import ABC, abstractmethod
from dataclasses import dataclass

from core.models import Rect


@dataclass(frozen=True)
class WindowInfo:
    hwnd: int
    title: str
    class_name: str
    process_id: int
    client_rect: Rect
    dpi_scale: float
    process_name: str = ""
    is_minimized: bool = False
    is_foreground: bool = False
    is_responding: bool = True
    match_reasons: tuple[str, ...] = ()


class WindowManager(ABC):
    @abstractmethod
    def find_window(self) -> WindowInfo | None: ...

    @abstractmethod
    def get_client_rect(self) -> Rect | None: ...

    @abstractmethod
    def is_alive(self) -> bool: ...

    @abstractmethod
    def is_minimized(self) -> bool: ...

    @abstractmethod
    def is_responding(self) -> bool: ...

    @abstractmethod
    def bring_to_front(self) -> None: ...

    @abstractmethod
    def get_dpi_scale(self) -> float: ...

    @abstractmethod
    def prepare_background_mode(self) -> bool: ...

    @abstractmethod
    def restore_window_mode(self) -> bool: ...
