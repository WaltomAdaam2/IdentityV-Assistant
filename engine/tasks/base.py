from abc import ABC, abstractmethod
import asyncio
from time import monotonic

from app.context import AutomationContext
from core.enums import TaskFinishReason


async def cancel_tasks(*tasks: asyncio.Task) -> None:
    for task in tasks:
        task.cancel()
    results = await asyncio.gather(*tasks, return_exceptions=True)
    for result in results:
        if isinstance(result, BaseException) and not isinstance(result, asyncio.CancelledError):
            raise result


async def await_with_workers(operation, *workers: asyncio.Task):
    observer = asyncio.create_task(operation)
    pending = set(workers)
    try:
        while True:
            done, _ = await asyncio.wait({observer, *pending}, return_when=asyncio.FIRST_COMPLETED)
            for worker in done & pending:
                pending.remove(worker)
                if not worker.cancelled():
                    worker.result()
            if observer in done:
                return observer.result()
    finally:
        await cancel_tasks(observer)


class TaskControl:
    def __init__(self) -> None:
        import asyncio
        self.cancelled = asyncio.Event()
        self.resumed = asyncio.Event()
        self.resumed.set()
        self._paused_at: float | None = None
        self._paused_seconds = 0.0

    @property
    def paused_seconds(self) -> float:
        return self._paused_seconds + (monotonic() - self._paused_at if self._paused_at is not None else 0.0)

    async def wait_if_paused(self) -> None:
        await self.resumed.wait()

    def pause(self) -> None:
        if self._paused_at is None:
            self._paused_at = monotonic()
        self.resumed.clear()

    def resume(self) -> None:
        if self._paused_at is not None:
            self._paused_seconds += monotonic() - self._paused_at
            self._paused_at = None
        self.resumed.set()

    def cancel(self) -> None:
        self.cancelled.set()
        self.resume()


class AutomationTask(ABC):
    @abstractmethod
    async def run(self, ctx: AutomationContext, control: TaskControl) -> TaskFinishReason: ...
