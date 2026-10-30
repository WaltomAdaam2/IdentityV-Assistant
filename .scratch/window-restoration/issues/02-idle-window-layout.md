# Keep normal game launch independent of automation layout

Status: resolved
Type: bug

## Scope

- The user reports a borderless game positioned in a corner on ordinary game launch. Launcher code does not set game window styles; the automation engine applies its configured layout during startup and every idle heartbeat.
- Only an active automation task may apply the automation layout. Idle service startup and diagnostics must leave newly launched game windows alone. Preserve restoration on service shutdown.
- Extend the existing engine lifecycle test seam: idle heartbeat leaves style untouched, starting a task applies layout before task work, shutdown restores it. Follow the user's recommended-default and verified checkpoint authorization.

## Progress

- Removed service-start layout application and gated heartbeat layout handling on running/paused automation. Apply layout before task work, including queued tasks and resumed task execution.
- Lifecycle regression failed on the old idle startup/heartbeat behavior, then passed. Window/shutdown suites: 33 passed; task queue, supervision and input suites: 31 passed.
- Reviewed direct launcher code: no game window geometry/style mutation. Real game launch was not exercised. Previously altered windows still need their owning old service stopped to restore its snapshot; restart the service to load this fix.
