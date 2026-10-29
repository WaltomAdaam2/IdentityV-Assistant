# Game launcher and independent login component

Status: accepted
Date: 2026-10-29

## Context

The launch page and Settings must locate and start the same Identity V client without the Fever platform. Login traffic requires a maintained protocol implementation. The automation engine has a separate lifecycle and must not own login sessions.

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
