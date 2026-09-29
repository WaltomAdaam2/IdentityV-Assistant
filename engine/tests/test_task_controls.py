import asyncio

from tasks.base import TaskControl


def test_task_control_pause_resume_and_cancel() -> None:
    async def exercise() -> None:
        control = TaskControl()
        control.pause()
        blocked = asyncio.create_task(control.wait_if_paused())
        await asyncio.sleep(0)
        assert not blocked.done()
        control.resume()
        await blocked
        control.cancel()
        assert control.cancelled.is_set()
        assert control.resumed.is_set()

    asyncio.run(exercise())
