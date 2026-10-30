# Version 0.1.8: update feedback and reviewable releases

Status: resolved

- Work on `SNAPSHOT-v0.1.8`, created from main a9a2900. Preserve the user's pnpm 12 package-manager/lockfile changes outside commits.
- Show current download speed and estimated remaining time to the right of the assistant installer progress bar. Use a short rolling sample, show unknown ETA before bytes arrive or while stalled, and reset on retry. Preserve cancellation and all integrity checks.
- Remove automatic source probes and their UI options, migrate legacy `auto` settings to `direct`, label direct as the default, and preserve explicit manual proxy choices across assistant and login-component flows.
- Render release-note Markdown with safe React elements, no raw HTML or executable URLs. Keep the existing paper palette and bounded scroll area. Use one established Markdown renderer rather than a handwritten parser.
- Bump all application/engine versions to 0.1.8. Record permanent SNAPSHOT branches, post-merge freeze, English-title/bilingual-body PR format, required skills/context and draft-only manual publication in tracked repository docs.
- PR format comes from WaveXinAddon's `.codex/pr-format.md`, matching the referenced shared project convention. Copy it into this repository so future agents do not depend on another checkout.
- Create a reviewable PR and signed Draft Release. Do not merge the PR or publish the release on the user's behalf. The user reviews the PR before merge and personally publishes the release; the branch freezes at merge. Never change the archived legacy repository.
- Continue authorized verified checkpoint commits/pushes. No more questions. Only builds and offline/mock-IPC browser tests; no native application installation/launch or live game/login tests.

## Verification

Use deterministic progress samples (steady, stalled, unknown size, reset), Rust source/config migration tests, real rendered React with mocked IPC for Markdown safety/layout and source choices, typecheck/build, offline suites, CI, clean signed packaging and draft asset/hash/provenance inspection. A draft is intentionally unavailable to public updater clients until the user publishes it.

## Work

- [Implementation and delivery](issues/01-update-feedback.md)

## Delivery

Implementation, signed packaging, independent review, CI and Draft Release preparation are complete; see the issue's final evidence. PR merge and formal publication remain the user's review actions, not automated steps.

## User follow-up (2026-10-31)

The user supplied replacement overview/journal screenshots and requested a centered README header like the reference, using this project's identity, actual stack/license and working navigation. Apply it to both languages without altering the supplied pixels.

The user subsequently explicitly authorized merging the PR after checks pass. This supersedes the earlier leave-PR-open decision. Keep the existing 0.1.8 release as a draft for the user's personal publication, retain its verified binary build commit, and permanently freeze SNAPSHOT-v0.1.8 immediately after merge. Complete all branch edits and verification records before merge.
