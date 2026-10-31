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
Q2 diagnostic preview modal vs alternatives;
Q3 top feedback; Q4 import settings modal/apply terminology; Q5 update suppression
scope; Q6 launcher settings surface and motion remain pending. Recommendations are
not approvals.
