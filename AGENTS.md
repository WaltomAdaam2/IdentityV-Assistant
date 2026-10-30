# IdentityV-Assistant agent instructions

Understand first, change surgically, verify through observable behavior. Preserve unrelated work.
Confirm the repository and branch before editing. Use the current branch unless the user requests otherwise.
Never force-push, rewrite history, change remotes, perform repository administration, or delete branches.
Do not use mass or recursive deletion. Delete only one necessary, explicitly named item per command.
Ask about checkpoint commits and pushes at the start of multi-step coding work unless already decided in the conversation.
Commit only task-related changes; summarize changes before committing and run relevant checks before pushing.

## Agent skills

### Required project context

Before implementation, debugging, review, or refactoring, read this file, `docs/agents/domain.md`,
`docs/agents/issue-tracker.md`, `docs/agents/triage-labels.md`, `GLOSSARY.md` when present,
the relevant `docs/adr/` entries, and the active feature's `.scratch/` specification and issues.
New agents and continued chats must also read relevant version history in `docs/releases/`,
`docs/agents/pr-format.md`, `docs/agents/release-format.md`, and `docs/updates.md` before version/PR/release work.
Update the relevant local issue with decisions, implementation progress, verification evidence,
and remaining work as the task progresses. Update glossary terms and ADRs when actual domain
knowledge or architectural decisions change. Do not invent documentation changes solely to satisfy this rule.
These requirements apply only to this repository and all agents working in it.

### Issue tracker

Use local Markdown under `.scratch/<feature>/`. See `docs/agents/issue-tracker.md`.
Do not publish issues, comments, or other messages without user authorization.

### Triage labels

Use `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, and `wontfix`.
See `docs/agents/triage-labels.md`.

### Domain docs

Use one root `GLOSSARY.md` and `docs/adr/`. See `docs/agents/domain.md`.
The frontend, Rust shell, and Python engine share one application domain.

### Engineering workflow

For every user instruction, read and apply the available **ponytail** and **frontend-design** skills.
Apply their guidance proportionately; a non-UI instruction does not require inventing UI changes.
Other skills are chosen by the agent when relevant or explicitly requested by the user.
Read a skill before applying it; do not claim an unavailable skill was used.
Agree meaningful test seams before TDD; write a failing behavior test, implement it, then verify.
Keep frontend game decisions in Python. Keep process ownership and lifecycle in Rust.
Review changes for concrete defects after implementation, with an independent reviewer when requested.
Do not treat unit, emulated, offline, or build checks as proof of live gameplay or login success.
For substantial coding work, report assumptions, changes, verification, and remaining risks concisely.

## GitHub version workflow

- Every version starts a new branch from current main named exactly `SNAPSHOT-v<version>`, for example `SNAPSHOT-v0.1.8`. This explicit project naming overrides a generic branch prefix.
- Confirm local/remote branch and PR state before editing. All version changes and verified checkpoint pushes go to that version's SNAPSHOT branch; submit a PR to main using [the PR format](docs/agents/pr-format.md).
- SNAPSHOT branches are permanent. Never delete them. Once their PR is merged into main, freeze the branch forever: no new commits, merges, rebases, resets, synchronization or ref updates. Later changes require a new version branch.
- Agents may prepare/upload **Draft Releases only**, following [the release format](docs/agents/release-format.md). The user reviews and personally publishes every release. Never publish, enable automatic publication, or change `draft` to false, even after a successful build or PR merge.
- Build signed assets from the clean, pushed SNAPSHOT commit, record that exact commit, and prepare the PR and draft for user review before merge. Keep the branch unchanged after merge. Publishing the reviewed draft uses the original verified build commit.
- Read and follow [the release procedure](docs/updates.md). Keep the original signing key, integrity checks and factual validation limits. Do not overwrite an existing release/tag or use the archived legacy download repository for new versions.
