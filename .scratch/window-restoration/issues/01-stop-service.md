# Restore the game window when stopping automation

Status: resolved
Type: bug

## Scope

- Interpret the requested window bar as the game title bar/border removed by automation. Restore the saved original layout when stopping the service.
- Existing Win32 restoration returns false on failure. The engine ignores that return value, skipping its existing retry and application error log.
- Use the existing engine run/shutdown seam with a fake window adapter, under the user's recommended-default authorization. Verify transient restoration failure is retried; offline checks do not prove live game restoration.
- Preserve unrelated package and lockfile edits; commit and push the verified fix on main as previously authorized.

## Progress

- Treat an explicit false restoration result as failure, reusing the existing bounded retry and application error log. Successful restoration and original style/rectangle/topmost snapshot behavior remain unchanged.
- Added the false-return case to the existing shutdown regression: failed before the fix (one attempt instead of two), then passed.
- Shutdown, window manager delegation and Win32 controller suites: 32 passed. Reviewed the stop command through Rust graceful shutdown to Python cleanup; no additional lifecycle machinery needed.
- Live game restoration and forced termination remain unverified. A failed Win32 call can still prevent restoration after both attempts; it is now reported instead of silently treated as success.
