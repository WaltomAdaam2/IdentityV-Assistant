# Version 0.1.7: repository consolidation and reliable feedback

Status: resolved

## User decisions (2026-10-30)

- Work on current main; commit and push narrowly verified milestones. Preserve existing package.json / pnpm-lock.yaml edits unless directly required by this task.
- Consolidate all desktop-releases releases into the public IdentityV-Assistant source repository. Preserve historical installer/signature bytes. Reformat release notes from evidence, append asset SHA-256 after Full Changelog, and do not invent entries to meet a minimum count.
- Publish the same signed 0.1.7 installer in both repositories once. The old repository needs its own old-URL manifest for existing clients. The user will delete that repository later; users who have not upgraded then need manual installation.
- Current first-party source and 0.1.7 use GPL-3.0-only. Preserve historical binary notices and third-party licenses, provenance and credits.
- Route map import outcomes to the existing top notice style, with fade, hover/focus retention, manual dismissal and longer error reading time. Investigate missing notices rather than assuming an earlier change removed them.
- Focus architecture changes on the actual release/update/import problems. Follow WaveXinAddon's concise Chinese/English README and separate detailed documentation organization. Independently review changes.
- Latest Q7 answer overrides the earlier blanket acceptance: build and offline tests only. Do not install or run the new native application, sign in, run live matches, or operate the user's game. Frontend interaction checks use the existing mocked native bridge as offline tests.
- User subsequently requested no more questions while away. Resolve routine choices using these decisions and the most appropriate minimal solution. Do not delete repositories/branches, rewrite history, change remotes, or perform repository administration.

## Deliverables and completion evidence

1. Map the source, domain docs, active specifications, lifecycle, updater, map import, build and release flows; record evidence and limits, not a claim of literal infallibility.
2. Repair the failed GitHub workflow; inspect the failing step and verify a successful run for the delivered commit.
3. Inventory all source/destination releases and tags, archive original assets, verify hashes, migrate all historical release assets and write true changelogs. Retain original metadata where changed URLs require a separately documented migration.
4. Replace runtime, about, packaging, release and documentation links; preserve version-bound signatures, hash/size checks and strict project URL validation. Verify new-source and legacy-bridge metadata/downloads without installation.
5. Exercise success, partial/all failure, cancellation, repeated identical notices, hover/focus, dismissal and transient lifetime through real rendered frontend with mocked native operations; retain persistent map-library errors.
6. Concise bilingual READMEs, separate Chinese/English detail docs, source licensing, third-party credits and release documentation agree with implementation.
7. Independent defect review; focused regressions plus final Rust/Python/frontend/build checks. Publish signed Windows x64 0.1.7 installer, signature, latest.json, checksum list, source tag and release notes; verify remote bytes and stable latest.
8. Report installation, live authentication and gameplay as untested per user instruction.

## Baseline

- Local main / origin main: 91e1dd261db543d90deebdc813bbcda944652325. Existing edits: packageManager 11.19.0 -> 12.16.1 and lockfile additions.
- IdentityV-Assistant is public; destination already has v0.1.0 and v0.1.1. Old public repository has five releases v0.1.2 through v0.1.6, each with installer, signature, manifest and checksum file.
- CI run 41024287728 at baseline failed only at Check Rust formatting; 254 Python tests, dependency installs, TypeScript and frontend build passed. Rust library tests were skipped after formatting failure.
- Old-client addresses are embedded in tauri.conf.json, updates.rs and update_source.rs. Repointing only old latest.json to a new-repository installer is rejected by old URL validation.

## Work tracking

- [CI](issues/01-ci.md)
- Subsequent focused issues will record implementation, review and publication evidence.

## Completion

All deliverables are verified in [the final validation record](issues/05-final-validation.md), including public releases, the original signing trust chain and the old-repository bridge. User-excluded native installation and live gameplay remain untested. The old repository may now be publicly archived by the user; it was not deleted or archived by the agent.
