# README presentation and authorized merge

Status: resolved
Type: task

## Progress

- User supplied two replacement screenshots; copy original bytes to docs/images/overview.png and journal.png, shared by both READMEs. The screenshots intentionally show the user's existing 0.1.7 UI; do not retouch version text.
- Match the reference's centered icon/title, concise description, navigation and badges using this project's existing book icon, real GPL-3.0-only license and installed stack. No copied flower branding, fabricated star count, or unsupported documentation links.
- The user explicitly authorized merge after checks pass and requested no more questions. Existing draft stays draft; merge must retain/freeze SNAPSHOT-v0.1.8. These are documentation changes only, so the verified installer remains built from 7eabc9c.
- Check exact image hashes, local links and real GitHub-rendered Markdown layout in both languages. Await CI and review before merge.

## Verification and merge gate

- Both replacement PNGs are byte-identical to the user attachments: overview 1499x1018, SHA-256 714385fe847cdb4e490d473218d8ee950d346cb1b6863fe0947e938f11d2fcc4; journal 1498x1019, SHA-256 ed1890484124e86d7c06c86f745b41b7d46016b6a141c0631146ad350404b687.
- Both READMEs passed GitHub's Markdown API rendering. Local browser inspection of that HTML with GitHub-like typography confirms centered headings, loaded logo/screenshots/seven badges, correct GPL-3.0-only/React 18/Tauri 2/Rust 2021/Python 3.12 labels, and no horizontal overflow at 390px. Actual badge release is v0.1.7 until the user publishes the draft; no fabricated stars or future release count.
- Local navigation targets exist. `git diff --check` passes. These are documentation/image-only changes; no extra application rebuild or native execution is needed.
- Merge PR #1 only after its latest checks pass, retaining the SNAPSHOT branch. Record merge outcome on GitHub and in the final response; do not append commits or synchronize this branch after merge.
- After merge, update the existing Draft Release's full notes and only its latest.json metadata asset to include the README change. Keep installer/signature/checksum bytes and actual build commit 7eabc9c unchanged. Archive old metadata first, recalculate all body hashes, verify draft=true and unchanged public Latest. The user alone publishes.
