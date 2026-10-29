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
