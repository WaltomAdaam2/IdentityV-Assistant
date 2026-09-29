from app.context import AutomationContext
from recovery.timeout import StateTimeout


class RecoveryManager:
    """Small bounded recovery hook; future policies can be added without task coupling."""

    async def recover_unknown_state(self, ctx: AutomationContext, timeout: StateTimeout) -> bool:
        if not timeout.expired:
            return False
        ctx.metrics.recovery_attempts += 1
        await ctx.emit("log", level="warning", source="recovery", message="State recognition timeout", data={})
        return True
