from time import monotonic


class TaskFeedback:
    """One task's observed stage and active time, shared by events and heartbeats."""
    def __init__(self):
        self.control = None
        self.started = 0.0
        self.stage_started = 0.0
        self.data = {}

    def start(self, task_id, control):
        self.control = control
        self.started = monotonic()
        self.stage_started = 0.0
        self.data = {"task_id": task_id, "stage": "running", "pause_reason": None}

    def elapsed(self):
        return max(0.0, monotonic() - self.started - (self.control.paused_seconds if self.control else 0.0))

    def update(self, payload):
        if payload.get("stage", self.data.get("stage")) != self.data.get("stage"):
            self.stage_started = self.elapsed()
        self.data.update(payload)

    def snapshot(self):
        if not self.data:
            return {}
        elapsed = self.elapsed()
        stage = str(self.data.get("stage", ""))
        next_action = ("review_and_resume" if self.data.get("pause_reason") else
                       "recover_window" if "recover" in stage else
                       "wait_for_screen" if stage.startswith("waiting") or "matching" in stage or stage == "loading" else
                       "observe_match" if stage in {"playing", "fixed", "v_fixed"} else
                       "recognize_and_continue")
        return {**self.data, "task_elapsed_seconds": round(elapsed, 1),
                "stage_elapsed_seconds": round(max(0.0, elapsed - self.stage_started), 1), "next_action": next_action}

    def clear(self):
        self.data = {}
        self.control = None
