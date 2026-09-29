from app.context import AutomationContext
from core.enums import TaskFinishReason
from tasks.base import AutomationTask, TaskControl


class WeeklyTask(AutomationTask):
    """Future extension point. No game coordinates or gameplay logic exist in this phase."""

    async def run(self, ctx: AutomationContext, control: TaskControl) -> TaskFinishReason:
        await ctx.emit("log", level="info", source="weekly_task", message="Weekly task is a framework placeholder", data={})
        return TaskFinishReason.COMPLETED
