# Proxy stages, upstream updates and quiet journal preferences

Status: complete
Type: task

## Scope

- User confirmed real game login through the proxy works, but reported transient unavailable messages during startup and forced cleanup after six seconds.
- Restore official update/hotfix entries; suppress shortcut creation only. Keep hidden console, owned-process cleanup and managed restart across hotfixes.
- Add a top-right proxy status pill using the existing application status styling. Show startup/checking/restart/ready/stopping/failure stages without stale startup errors.
- Journal map options remain saved without the global saved toast. Floor is temporary and starts at 1F on reentry; retain and verify the existing local-floor behavior.
- Continue approved test seams for proxy lifecycle and launcher UI. Journal verification covers visible preference persistence, no saved toast and temporary floor reset. User already chose recommended defaults for subsequent decisions.

## Evidence and plan

- Existing status reports error after a single health failure. Existing launch retry destroys a still-running process after a single failed health check.
- Existing journal floor already uses local state; only non-floor changes call persistence. The global autosave effect unconditionally shows a toast.
- Official full-update entry opens the release page and exits normally; it does not run an installer. Only explicit managed restart exit code 75 should restart an owned proxy, on the same port; no game relaunch.
- Preserve user package.json and pnpm-lock.yaml edits and keep working on main.

## Verification

- Journal silent-save regression failed before the toast guard, passed afterward; committed as a83b131. Floor behavior was already local and remains unchanged.
- Managed restart fixture failed on the original early-exit behavior and now passes. Coverage includes runtime same-port restart, cancellation before respawn, bounded repeat restarts, normal exits, transient/persistent health failure and update prompts exceeding readiness timeout.
- Rust: 55 passed, 6 intentionally ignored with two test threads. A parallel fixture startup exceeded a three-second test deadline; the isolated case and full two-thread run passed.
- Browser: all 9 launcher/journal tests passed. Existing status classes and top-right position verified; stages replace stale errors. TypeScript/Vite production build passed.
- Actual official embedded Python: all 4 adapter tests passed without starting login, importing certificates or performing updates. User separately confirmed live proxy login works.
- Independent review found update-dialog timeout and cancellation while awaiting background recovery; both fixed with regression coverage. No installer/release published.
- Follow-up: assistant-style component update dialog, selectable source and launch-settings update button; automatic check only at game launch when enabled. Continued in issue 07.
