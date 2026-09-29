from abc import ABC, abstractmethod

from app.context import AutomationContext
from core.enums import TaskFinishReason


class TaskControl:
    def __init__(self) -> None:
        import asyncio
        self.cancelled = asyncio.Event()
        self.resumed = asyncio.Event()
        self.resumed.set()

    async def wait_if_paused(self) -> None:
        await self.resumed.wait()

    def pause(self) -> None:
        self.resumed.clear()

    def resume(self) -> None:
        self.resumed.set()

    def cancel(self) -> None:
        self.cancelled.set()
        self.resumed.set()


class AutomationTask(ABC):
    @abstractmethod
    async def run(self, ctx: AutomationContext, control: TaskControl) -> TaskFinishReason: ...
