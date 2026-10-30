# Game launcher and independent login component

Status: accepted
Date: 2026-10-29

## Context

The launch page must locate and start the Identity V client without the Fever platform. The redundant Settings launch/path controls were removed at the user's request. Login traffic requires a maintained protocol implementation. The automation engine has a separate lifecycle and must not own login sessions.

## Decision

Use one Rust game launcher module for executable discovery, validation, direct launch and login component coordination. Reuse the independent official idv-login component instead of copying its protocol implementation into the application. Its account data remains in its own data directory; the assistant does not expose or log credentials.

Only treat a component as owned when its process identity is known. Wait for proxy readiness and verify process mode before launching the game. A listening port or successful URI dispatch alone does not establish ownership or login success.

On assistant exit or update, stop only the login component started by this assistant. Leave the game and pre-existing external components running. The user accepted that a remaining game loses this component's login proxy when the assistant exits. Cleanup must also run on failed startup and must never terminate processes by broad executable-name matching.

## Alternatives

- A new protocol implementation would duplicate a large, evolving login stack and require separate account-authentication maintenance.
- Separate launcher logic in Settings and the new page would allow different validation and cleanup behavior.
- A general application lifecycle framework is unnecessary for this change.

## Consequences

Offline tests can verify discovery, process ownership, readiness, failures and cleanup, but cannot prove successful live account authentication. First login and expired credentials still require the user.

The upstream Windows download is an installer, not the runtime. Discover or select an already installed official runtime and use a dedicated component data directory under the assistant's local data folder. Do not silently execute the installer. Pin the supported runtime version to v6.3.2-stable until a later version's contract has been reviewed. The upstream application also manages certificates and its URI handler; process proxy mode must not be described as having no system-level effects.

## 2026-10-30 refinement

The user requested automatic scan/configuration and in-app official downloads. Reuse the existing download-source selection, verify the fixed release and standalone unpacker by size/SHA-256, extract to staging and atomically publish under the assistant executable directory. Preserve existing installations and do not run installer scripts.

Host the pinned runtime through an independent bootstrap stored in assistant data. Suppress upstream automatic shortcuts and first-use GUI; retain the official main function, account/authentication behavior and cleanup. CREATE_NO_WINDOW plus private pipes replaces console signaling. Important structured logs flow into application logs, and a second click cancels startup/setup. Readiness always includes current health and exact process/port ownership.

The user subsequently requested restoring official update and hotfix entries. Official hotfixes may modify their own runtime files. Explicit managed restart exit code 75 permits at most three consecutive restarts before readiness, preserving the port, data directory and process ownership; normal exits never auto-restart. Background monitoring continues away from the launch page. Transient health failures show checking, three consecutive failures show unavailable, and an exited process invalidates readiness immediately.

Update decisions now use the assistant's existing update-dialog style. A separate preference checks automatically only on game launch; launch settings also provide manual checking. A hidden worker applies only the reviewed hotfix snapshot through upstream code while the owned proxy is paused, honoring the selected source and preserving official validation records. The proxy is restored on its original port even after a failed update, unless the assistant is exiting. Bootstrap retains rollback hooks but delegates new update prompts to the assistant. Full-release downloads use official asset links; future versions are not silently installed across the existing compatibility boundary.

### Subsequent update-check simplification

The user removed the optional component-check setting. This supersedes the separate preference above: always check component updates on game launch and retain the confirmation dialog and manual settings entry. Discard legacy toggle values. Do not check component updates at assistant startup or alter the assistant's own update preference.
