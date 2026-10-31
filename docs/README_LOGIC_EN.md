# Feature logic

**Language / 语言:** [中文](README_LOGIC.md) | English · [Back to README](../README_EN.md)

## Responsibilities

React edits settings and displays status and feedback. Rust owns configuration, windows, processes, map imports and updates. The Python automation engine owns capture, recognition, task decisions and input. The interface does not decide game actions.

The game launcher, login component and automation engine have separate lifecycles. Ordinary game launch neither changes game window layout nor starts match tasks. Proxy readiness does not prove account authentication.

## Game launcher and login component

The launcher locates the official `dwrg.exe`; its settings panel also accepts a manual path. Fever, installers and unrelated executables are rejected. Launch checks component updates, waits for proxy readiness, then starts the game directly.

The reviewed runtime contract is [KKeygen/idv-login v6.3.2-stable](https://github.com/KKeygen/idv-login/releases/tag/v6.3.2-stable). Setup first scans for a valid installed runtime, then offers an official download through the selected source. Select the runtime or its `python-embed/python.exe`, not the downloaded installer. The assistant verifies and extracts downloads without silently executing third-party installer scripts.

Initial login and expired credentials require the user. The component keeps its account data under `%LOCALAPPDATA%\IdentityV-Assistant\login-data\idv-login`; the assistant does not read or display account credentials. The upstream component configures login certificates and a URI handler, unlike the visual automation engine.

Component checks happen on game launch, with a separate manual entry. They do not run automatically at assistant startup. Hotfixes require confirmation, pause the owned proxy, and attempt to restore its port afterward. Unknown full component versions are not silently installed across the reviewed compatibility contract.

The assistant manages only processes it started and can identify. A pre-existing independent component must be closed by the user; it is not adopted or terminated by executable name. Assistant exit or update closes its owned proxy and attempts to restore the prior URI registration. The game remains open but loses that proxy.

## Tasks and queue

| Task | Behavior |
| --- | --- |
| Lobby login | Confirms known login screens twice, selects the platform, handles prompts and waits for home; configurable 30–900-second deadline, default 300 |
| Blackjack | Resumes from known home, matchmaking, preparation, match or result screens; selects actions by detected role and counts a cycle after returning home |
| Hunter match | Enters through diary deduction and uses the shared hunter match logic |
| Hunter rank | Enters through rank and traditional rank, sharing hunter combat decisions |
| Hide and Seek | Selects Bad Child, starts movement and camera together in confirmed living stages, and handles fixing, elimination and return flow |

Enabled tasks run in list order, starting the engine if needed. Infinite match loops default off. Pause releases held input and excludes paused time from deadlines; resume captures and recognizes the current screen before continuing.

From 0.1.9, a new task or queue checks the game window, responsiveness and capture before starting. Failure blocks the start and offers recheck and capture-diagnostic actions. Actual starts retain existing engine layout handling; resolution or foreground differences produce warnings without switching capture backends. Match loops do not repeat preflight. After a passed retry, use Start requested task to continue; the check itself never starts it automatically. Stop task and queue stops the current task and clears the queue. The Overview task progress card shows the current stage, task/stage active time, pause reason and next-step guidance. Paused time is excluded; timings do not persist across engine restarts.

Ordinary unrecoverable failures and login timeouts pause the current task and preserve the queue. The four match tasks attempt foreground/capture recovery for up to 300 active seconds; exhaustion stops the task and clears the queue while the engine remains running. The task-row Stop button also clears pending tasks. Recognized matchmaking/result waits differ from unknown-screen timeouts.

Hunter tasks attempt surrender when the recognized game timer reaches six minutes. If valid OCR has been unavailable for at least 30 seconds, a local six-minute active-combat timer can provide a fallback; actual clicks still require screen confirmation. Detailed stages and action schedules are in [state-machine.md](state-machine.md).

## Capture, input and windows

The default backend is WGC, capturing the game client area by window handle. Only explicit `auto` configuration tries WGC, DXcam, then MSS. Occlusion, monitor and multi-display behavior differs between backends.

**Capturing an obscured window does not mean input can run in the background.** Tasks send real Windows SendInput after checking foreground ownership, tracking held keys/buttons for cleanup. There is no default dry-run mode.

Explicit engine startup, including F3, or task startup activates the game layout: 1600×900 at the top right by default. Temporary engine startup for diagnostics does not activate layout. Shutdown releases input and attempts to restore the original window, with up to ten seconds for supervised cleanup. Ordinary game launch preserves existing window style and placement.

## Journal and map-pack format

Use Ctrl/Shift to select multiple ZIP files. Each pack imports independently; a failure does not stop later packs. Top notices show normal results for one second and errors for eight seconds, then fade over 600 ms. Hover or keyboard focus retains the notice; Close dismisses immediately. Completion stays visible after scrolling or leaving the journal, and details remain in logs.

Import reads only PNG/JPEG/WebP files referenced by `pack.json`. It does not replace an existing `packId`, modify source ZIPs or read another application's data. Root or nested `pack.json` is supported; legacy `manifest.json` is not. Traversal, links, duplicate entries and excessive extraction are rejected. Current bounds include 20 MiB per image, 512 MiB per pack, 2 MiB per descriptor, 20,000 entries and 16 path levels.

Minimal example, with an actual `maps/1f.png` inside the ZIP:

```json
{
  "packId": "example-s0-hard",
  "packName": "Example maps",
  "seasonId": "s0",
  "difficulty": "hard",
  "maps": [
    {"index": 1, "category": "lf", "floors": {"1": "maps/1f.png"}}
  ]
}
```

Door categories are `lf`, `rf`, `lr`, `triple` and `td`. `floors` maps base images; optional `display` maps route images. Use `1`, `2`, `b1f` floor keys and `/` in image paths. Legacy floor key `3` is read as basement.

Season, difficulty, doors and display mode save silently. Reopening the page resets floor to 1F and expands filters. Detail and preview views stay inside the assistant, not over the game. Esc leaves preview first, then returns to the list.

## Local data, logs and diagnostics

```text
%LOCALAPPDATA%\IdentityV-Assistant\
  config.json                 Settings and enabled tasks
  logs\YYYY-MM-DD.jsonl       Complete engine and UI events
  debug\captures\            Explicitly requested debug captures
  debug\failures\            Automatic failure screenshots
  backups\                   Settings saved before restoration
  journal\packs\<packId>\    Imported maps
  login-data\idv-login\      Independent login-component data
```

Dated logs retain the latest thirty days. The interface shows the current session, up to 99,000 entries at 1,000 per page; clearing it does not delete disk logs. Developer mode reveals retained details. Check logs and screenshots for personal information before sharing them.

Diagnostics discover windows, capture frames manually and inspect templates. Diagnostic input needs separate enabling and is unavailable while a task runs or is paused.

Failure screenshots default on and can be disabled in Settings. An error pause or window-recovery timeout saves at most one most recent successful WGC game-window frame per failure, skipping when none is available. Automatic saving excludes DXcam/MSS screen-region frames to avoid capturing other windows covering the game. Filenames and logs preserve the actual capture time, which may precede the failure. Retention is limited to 20 images and 100 MiB total, with a 20 MiB per-image limit. Only generated failure images are pruned; manual captures remain untouched. Images stay local and may contain names or chat.

### Settings backup and restore

The Recognition and diagnostics section at the bottom of Settings exports assistant preferences and task parameters without accounts, maps or logs. Import accepts this feature's versioned JSON, up to 1 MiB, validating fields, types, version and hotkey. An unavailable imported game path retains the local value. Review Current / After apply differences in the centered dialog, then choose Confirm apply. Matching settings need no application. Running, paused or queued tasks block restoration; an idle engine stops and reads restored settings on its next start. Original settings are retained under `backups`; configuration writes use atomic replacement, and failed desktop preference application attempts rollback. Save exports outside the assistant data directory.

A configuration read failure blocks editing and offers Reload settings. Failed saves remain marked Settings not saved with a retry action. Result notices hold for two seconds, then fade over half a second; hover or keyboard focus retains them. Confirmation dialogs do not close automatically.

### Diagnostic bundles

The centered preview lists files, total size and whether screenshots are included. Choose a ZIP destination; nothing uploads automatically. Defaults include assistant/engine state and versions, necessary environment, up to 4 MiB of the current assistant session's logs, and redacted settings. Account directories, credentials, private keys, maps and component runtimes are excluded. Sensitive fields and text containing paths, URLs or credential markers are filtered, but review logs before sharing. Screenshots require a separate selection and include only recent application-generated debug/failure images, up to 20 images / 100 MiB. Images are not text-redacted. The preview reports when the log limit was reached.

## Updates and validation limits

Assistant and login-component updates are separate. Assistant startup checking can be disabled; a new version is shown for review before download or installation. All sources enforce signature, signed version, installer URL, SHA-256 and size validation. GitHub direct is the default, with three manually selected proxies. Version 0.1.8 removes automatic probes. Speed and remaining-time estimates use the last five seconds of actual byte changes; stalled transfers show an unknown remaining time.

Component hotfixes in 0.1.9 also show actual per-module byte counts and download speed. Reliable server lengths enable totals and percentages; otherwise the total remains unknown. Applying files and restoring the proxy have separate stages and are not presented as downloading.

See [Updates and releases](updates_EN.md) for repository migration, legacy compatibility and publishing. Tests cover screenshot recognition, logic, mocked IPC and package integrity. Validation of 0.1.9 and 0.1.10 does not cover installation, native-app execution, live account authentication or live gameplay.
