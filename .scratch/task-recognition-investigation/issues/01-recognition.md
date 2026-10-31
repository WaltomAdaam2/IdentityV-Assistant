# Identify non-login task stall

Status: needs-info
Type: bug

## Progress

- Read the app's 2026-10-31 JSONL log. Hide and Seek starts at 08:21:40, 08:24:21 and 08:28:40 UTC after successful preflight; no task_progress event follows those starts. Heartbeats remain running and screenshot metrics increase. This rules out the task button/preflight blocking those recorded starts, but does not prove correct captured pixels.
- A foreground/capture recovery warning occurs initially and reports recovery at 08:21:48; subsequent sessions remain unrecognized until user stop. No failing recognition frame was saved for these sessions.
- Compared v0.1.8 with current source: task routers, detector templates and WGC crop implementation are unchanged; capture manager adds a copied frame for optional failure retention.
- Offline replay of the eight newest already-saved captures: the two Oct 5 lobby frames match all ten home points at native and normalized resolution; two older Hide and Seek frames match ready_start; Oct 30 loading is correctly recognized. Other recent saved images are the patch download screen and dice board, not the failing Oct 31 screen.
- Root cause remains unconfirmed. A saved frame from the stalled session is needed to distinguish an unsupported game screen, changed visual layout, or unexpected capture content; do not claim gameplay fixed from synthetic tests.
- All 267 isolated engine tests passed in 21.02 seconds. No engine/recognition code was changed on this evidence. PR stays draft, with no merge or packaging until the reported failure is resolved.
