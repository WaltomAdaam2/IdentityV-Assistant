# Version 0.1.6 game launcher

Status: ready-for-agent

## Requested result

- Upgrade first-party version declarations and displayed versions to 0.1.6.
- Add 游戏启动 between 任务 and 加页手记, with a matching English label.
- Discover the installed Identity V client and launch it directly, without launching the NetEase Fever platform.
- Establish a login proxy and support remembered account login in the manner of KKeygen/idv-login.
- Match the supplied launcher reference: game background, Identity V logo at top left, announcements at bottom left, launch button at bottom right.
- Configure Matt Pocock engineering skills for this repository only; future agents must read and maintain its context.
- Use TDD for behavioral changes, apply focused architecture improvements, and independently review the resulting changes.

## User-approved workflow

- Work on current main. Commit and push each narrowly verified milestone.
- Use local Markdown issues, not GitHub Issues.
- Setup draft approved: root AGENTS.md, single-context domain docs, five default triage labels.
- Approved test seams: game discovery/launch; proxy readiness, shutdown and failures; launcher navigation, status and announcement failures.
- Offline tests do not prove real-account login or live game behavior. Do not record credentials in source, logs, test fixtures, or issue documents.

## Reference and current evidence

- Screenshot: C:/Users/Walto/.codex/attachments/01679743-2ec7-4cdd-ae6d-7b5a599259ab/image-1.png.
- Baseline main: 02aceaaaba980a28bda5e81648e7b1e682a1714a; working tree initially clean.
- Upstream: https://github.com/KKeygen/idv-login, inspected 2026-10-29.
- Upstream game discovery reads HKCU Software/FeverGames/FeverGamesInstaller/game.
- Upstream h55 standalone arguments: --start_from_launcher=1 --is_multi_start.
- Proxy is established before game launch, with HTTP_PROXY/HTTPS_PROXY inherited by the game.
- Upstream exposes --uri idvlogin://start?game_id=h55, and account login remains subject to initial authentication and token expiry.
- Integration/packaging approach remains under investigation; do not equate a spawned helper with successful proxy readiness or login.

## Completion evidence required

- Version declarations agree; rendered version comes from package metadata.
- Automatic discovery, executable validation, direct start and proxy failure paths are exercised through approved interfaces.
- Proxy ownership and shutdown behavior are tested; unrelated applications are not stopped.
- Rendered launcher is inspected against the reference, including loading/error states and narrow windows.
- Relevant Python/frontend/Rust tests and packaging checks pass; independent review findings resolved.
- Record any unverified live behavior explicitly; do not claim full completion without requirement-specific evidence.

## Progress

- Completed source/reference discovery and repository-specific skill configuration approval.
- Architecture scan recommends one launcher module shared by existing Settings launch and the new page, with one owner for helper cleanup.
