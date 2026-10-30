# Restore the main window from the tray icon

Status: resolved
Type: bug

## Expected behavior

After minimizing to the tray, left single-click and double-click both show, restore, and focus the main window. Preserve the current tab. Right-click keeps the tray menu.

## Progress

- Existing tray builder only handles the Show menu command; icon click events are not registered.
- User approved the event boundary for TDD.
- The regression test first failed because left click did not request restoration, then passed with the event handler. Left release and left double-click invoke the shared show/unminimize/focus operation; other buttons do not.
- Left click no longer opens the tray menu; right click retains the existing menu. The menu Show command uses the same restoration function.
- Both desktop tests passed. Independent read-only review reported no findings.
- Native Windows mouse delivery and actual foreground focus have not been exercised; the unit test proves event routing, not OS behavior.

## 2026-10-30 title-bar minimize

- User requested the top-right minimize button always minimize to the taskbar. It now invokes a dedicated native command that bypasses tray hiding until the window is restored and focused. Other system minimize actions still respect the existing tray preference; the setting description states this exception.
- Browser interaction test confirms the title-bar button invokes the taskbar command instead of the generic window minimize command. Rust compilation and existing tray-event tests passed. Independent static review found no new defect; actual taskbar restore in the elevated native window remains untested.
