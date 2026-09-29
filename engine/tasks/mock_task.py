import asyncio

from app.context import AutomationContext
from core.enums import TaskFinishReason
from states.evidence import StateEvidence
from states.enum import GameState
from tasks.base import AutomationTask, TaskControl


class MockTask(AutomationTask):
    """Exercises the production IPC and state plumbing without game interaction."""

    sequence = (GameState.LOBBY, GameState.MODE_SELECT, GameState.MATCHMAKING, GameState.LOADING, GameState.IN_MATCH, GameState.RESULT, GameState.LOBBY)

    async def run(self, ctx: AutomationContext, control: TaskControl) -> TaskFinishReason:
        for state in self.sequence:
            await control.wait_if_paused()
            if control.cancelled.is_set() or ctx.cancellation.cancelled:
                return TaskFinishReason.USER_STOPPED
            evidence = StateEvidence(state=state, confidence=0.99, source="mock")
            # Simulate two stable recognition frames so mock mode exercises state confirmation.
            ctx.state_history.observe(evidence)
            transition = ctx.state_history.observe(evidence)
            await ctx.emit("task_state", state=state.value, previous_state=transition.previous.value if transition else ctx.state_history.current.value, confidence=evidence.confidence)
            if transition:
                ctx.metrics.state_changes += 1
                await ctx.emit("state_transition", **{"from": transition.previous.value, "to": transition.current.value, "confidence": transition.confidence, "duration_in_previous_state": round(transition.duration_in_previous_state, 3)})
            await ctx.emit("log", level="info", source="mock_task", message=f"Mock state: {state.value}", data={})
            await ctx.emit("planned_action", action=f"handle_{state.value}", input="none", target=None, dry_run=True)
            try:
                await asyncio.wait_for(control.cancelled.wait(), timeout=ctx.config.mock_step_delay_seconds)
                return TaskFinishReason.USER_STOPPED
            except TimeoutError:
                pass
        return TaskFinishReason.COMPLETED
