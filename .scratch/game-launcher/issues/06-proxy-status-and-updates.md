# Proxy stages, upstream updates and quiet journal preferences

Status: claimed
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
