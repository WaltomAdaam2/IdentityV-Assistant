# Launcher layout and hotkey conflict on unrelated saves

Status: resolved
Type: bug

## Scope and evidence

- Remove the component auto-check option; always check when launching the game, retaining the existing update dialog. Retire the old stored toggle, including false values.
- Two assistant processes were observed. A failed initial F3 registration leaves the registered-key state empty; every unrelated setting save then retries F3 and fails. Keep real conflicts visible when selecting a new hotkey, but do not block unrelated saves on an unchanged unavailable key.
- The launcher background belongs to the inner content while the error banner is outside it. Fixed minimum height and a 180px bottom gap cause overflow when the banner appears.
- Verify settings/hotkey change policy and the already-approved rendered launcher/update seams. Apply existing recommended-default authorization; preserve user package/lock changes and work on main.

## Progress

- Removed the component auto-check preference and migrate away legacy values. Every game launch checks updates; the existing confirmation dialog remains. Assistant startup does not check component updates.
- Unrelated setting saves no longer retry an unchanged hotkey that failed initial registration. Actual hotkey changes retain registration validation and rollback.
- Moved the background to the launcher workspace and removed fixed content height/gap. The settings panel scrolls within the available space below the visible proxy pill.
- Reused the existing status-pill styling, entrance and dot breathing animation; added a short text transition on proxy phase changes. Existing reduced-motion rules disable animations.
- Browser regressions failed before implementation on legacy-false update checking and layout overflow, then passed. Four focused cases passed: updates, long-error layout, proxy stages/visibility/animations, and long announcements/button behavior. Final background assertion passed again. Rendered 1000x680 screenshot inspected.
- TypeScript/Vite production build and 8 Rust config/desktop tests passed. Independent read-only review found no concrete remaining regression. No dependencies added.
- Two running assistant processes were left untouched. Browser mocks and Rust tests do not prove native F3 ownership, native window scaling, or live login; actual shortcut contention still requires releasing the occupied key or selecting another shortcut.
