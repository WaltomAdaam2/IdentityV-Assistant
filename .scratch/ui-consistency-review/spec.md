# UI consistency, feedback and component update review

## User objective

Review all six supplied screenshots: diagnostic preview, backup success message,
empty/populated settings import preview, repeated component update prompt and
launcher settings. Ask the user about each proposed UI change, including other
unsuitable inline feedback and missing/inappropriate motion across the frontend.
Use grill-me/grilling for decision rounds; preserve all explicit requested outcomes.

- Document mandatory staged use of tauri-v2, frontend-design, ui-ux-pro-max,
  animate, impeccable and web-design-guidelines in AGENTS.md; ponytail for every
  code change.
- Establish unified previews and feedback; use Confirm apply terminology for
  imported settings, with readable labels rather than raw internal field paths.
- Add an option to suppress component update prompts, with scope decided by user.
  Diagnose repeated prompts from actual records; do not assume download failure.
- Animate launcher settings open/close with keyboard/focus/reduced-motion support.
- Inventory other UI and animation issues and confirm each proposed change.

## Current constraints and evidence

- SNAPSHOT-v0.1.10 at e7713e1, PR #3 open/draft; original package.json and lockfile
  edits remain excluded. No merge or release packaging during unresolved task
  recognition investigation; the existing signed v0.1.9 draft remains unchanged.
- React 18 / TypeScript / Vite frontend, Tauri v2 single custom-titlebar window,
  minimum/default 1000x680. Existing custom IPC handles files and updates; no new
  window/plugin permission is inherently needed for frontend dialogs.
- Six screenshots inspected. Latest requested changes supersede the prior decision
  to keep support previews inline; no product-wide redesign chosen yet.
- Native assistant, login component and gameplay must not be run; use fixtures,
  read-only update metadata, mock IPC, browser rendering and builds.

## Decision round 1

Q1 confirmed: preserve incumbent cream/olive visual direction, existing font and
settings rows; unify overlays/feedback/button hierarchy without a full redesign.
Q2 confirmed: centered diagnostic preview with file list, total size, screenshot
inclusion and normal-size Save ZIP/Cancel footer actions.
Q3 confirmed: nonblocking top notices for export/apply/save, automatic dismissal,
manual close, hover/focus retention, longer errors with expandable detail.
Q4 confirmed: centered Apply imported settings dialog with readable field names,
Current/After apply columns and Confirm apply; no-difference preview has Close only.
Q5 confirmed: ignore only this exact batch at game launch; a new version/commit
reminds again. Manual checking remains available.
Q6 confirmed: preserve the dark launcher settings popover, group paths/actions,
animate opening/closing and support Escape, return focus and reduced motion.

The remaining audit proposals in audit.md are not yet approved. Present them as
selectable questions, as requested again by the user after the first popup round.

## Subsequent timing instruction

All automatically dismissed notifications, including settings-saved feedback,
must remain for 2 seconds and fade over 0.5 seconds. This supersedes earlier
different success/error durations. Collapse the occupied layout space during the
fade so surrounding content moves smoothly; preserve hover/focus retention.
Interactive confirmation dialogs are manually closed, not timed out.

## Decision round 2

Q7-Q30 present audit items A01-A24 individually. Q31 asks to repair the reproduced
narrow log toolbar (clear button becomes vertical); Q32 asks to widen the narrow
launcher news column and prevent English header overlap.

All recommendations were approved except these explicit corrections:
- Q16: after clarification, confirmed to show a button that starts the originally
  requested task/queue after a passed retry; execution requires the user's click.
- Q28: retain decorative card/bubble effects and inspect comparable surfaces for
  missing effects to complete. Do not remove hover, sweep or decorative movement
  merely because it is optional; keep the approved faster main-page transition.
- Q30: lock closing during an active update download; users cancel the download
  before closing. Do not substitute a background-download discovery feature.
- Q31/Q32: fix the layouts. Respect the native minimum window size and verify the
  smaller CSS viewport caused by the existing in-app zoom rather than claiming
  an unsupported native window size was tested.

## Latest delivery decision

User explicitly set version 0.1.10. The latest instruction authorizes restoring sidebar press behavior, a focused final review, merging PR #3, then creating the v0.1.10 Draft Release. Build the clean pushed commit before merge and freeze the SNAPSHOT branch after merge; the draft targets that original build. Do not publish. Preserve the previously approved UI changes and the offline-only validation scope.
