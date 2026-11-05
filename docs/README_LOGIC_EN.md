# Feature logic

**Language / 语言:** [中文](README_LOGIC.md) | English · [Back to README](../README_EN.md)

## Responsibilities

React edits settings and displays status and feedback. Rust owns configuration, windows, processes, map imports and updates. The Python automation engine owns capture, recognition, task decisions and input. The interface does not decide game actions.

The game launcher, login component and automation engine have separate lifecycles. Ordinary game launch neither changes game window layout nor starts match tasks. Proxy readiness does not prove account authentication.

## Game launcher and login component

The launcher locates the official `dwrg.exe`; its settings panel also accepts a manual path. Fever, installers and unrelated executables are rejected. Launch checks component updates, waits for proxy readiness, then starts the game directly.

The reviewed runtime contract is [KKeygen/idv-login v6.3.2-stable](https://github.com/KKeygen/idv-login/releases/tag/v6.3.2-stable). Setup first scans for a valid installed runtime, then offers an official download through the selected source. Select the runtime or its `python-embed/python.exe`, not the downloaded installer. The assistant verifies and extracts downloads without silently executing third-party installer scripts.

Initial login and expired credentials require the user. The component keeps its account data under `%LOCALAPPDATA%\IdentityV-Assistant\login-data\idv-login`; the assistant does not read or display account credentials. The upstream component configures login certificates and a URI handler, unlike the visual automation engine.

Component checks happen on game launch, with a separate manual entry, never at assistant startup. From 0.2.0 all module hotfixes are skipped. Only compatible full releases pinned by size and SHA-256 can be installed: one confirmation downloads, verifies and configures the component. Unsupported future releases wait for an assistant compatibility update.

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

Use Ctrl/Shift to select multiple ZIP files. Each pack imports independently; a failure does not stop later packs. Top notices stay for two seconds, then fade and collapse over half a second. Hover or keyboard focus retains the notice; manual dismissal is also available. Completion stays visible after scrolling or leaving the journal, and details remain in logs.

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

0.2.0 bundles `s0_hard_11-05.zip` (29 maps) and `s0_nightmare_11-05.zip` (26), retaining base and route layers. Startup silently fills missing content and records details in runtime logs. Verified original packs move to `journal/backups`; customized packs are preserved and take display priority, with conflicts logged.

### Recognize a first-floor map (0.2.0)

**L** cycles the assistant map floors while the game or assistant is foreground, without game input or focus changes. Nightmare cycles basement→1F→2F; hard omits basement. Unavailable floors are skipped, and assistant text fields do not trigger the shortcut.

Select the season and base/route display, expand the first-floor map in game, then choose **Recognize 1F map** or press **P** in the game. Capture checks the visible floor labels first: `1, 2` means hard; `-1, 1, 2` means nightmare. Missing or obscured labels stop recognition with retry guidance, without falling back to the manual filter. Results synchronize the difficulty filter; manual difficulty remains for browsing. P can be changed in Settings, preserves game input and never activates the assistant. Pressing P again cancels and reaps the old helper before recapturing; rapid presses coalesce to the latest request instead of waiting for the previous match to finish. Pause or stop running/starting automation first.

Recognition searches only first-floor templates for the selected season and detected difficulty, ignoring the door filter. Route mode requires an available first-floor route image and never silently substitutes a base display image. Door category and map number identify each candidate, so different doors can each have a Map 1. The interface shows up to four accepted candidates in match order, without accuracy percentages; logs and diagnostics retain the full results. **Automatically open the best match**, below the recognition button, defaults off and remembers your choice. When enabled, a successful match opens the first candidate in the full-window preview without activating the assistant or taking game focus. Automatic entry does not confirm that the candidate is correct.

Drag the black map area, or focus it and use arrow keys (Shift moves faster). All maps and floors share the saved position across restarts. Changing the image or window size restores the relative position while keeping at least 16 pixels clear at every edge. Close, floor and position controls stay above the map. Beside Close, choose top-left, top-right, center (default), bottom-left or bottom-right. Clicking the selected preset again keeps it selected; dragging or moving with arrow keys clears the preset selection and saves a custom position. Close the preview to return to the candidate list and choose another map.

A separate read-only process captures through WGC, without starting the automation service, changing game layout, sending game input or falling back to desktop capture. A tentative crop appears centered between the existing dividers during matching and updates when a better-matching region is found. **View recognized region** remains collapsed in the result. Each attempt keeps local diagnostic evidence as described below. Cancellation, changed recognition filters and leaving Journal invalidate the old request; late results cannot replace newer choices. Failed matches clear old candidates and ask you to open the first-floor map and retry. Requests time out after 120 seconds; image decoding is capped at 16,777,216 pixels.

Each recognition reuses normalized query edges and resized images, with a 32 MiB refinement cache released on completion or cancellation. This reuse does not change the scale search or match scores. Localization and geometry thresholds are heuristics. Limited exploration, occlusion, complex HUDs, game scaling and similar structures can cause missed or ambiguous matches. Generated fixtures and template-crop checks do not establish real gameplay accuracy. Check the candidates and recognized region before using the result.

The minimum region size applies both to proposals and to visible geometry after background cropping. Regions with a shorter side below 64 pixels are rejected so padding cannot make a small icon pass validation. Expand or zoom the map, or explore further, when the visible minimap is too small. This check does not exclude every larger HUD false match.

## Local data, logs and diagnostics

```text
%LOCALAPPDATA%\IdentityV-Assistant\
  config.json                 Settings and enabled tasks
  logs\YYYY-MM-DD.jsonl       Complete engine and UI events
  debug\captures\            Explicitly requested debug captures
  debug\failures\            Automatic failure screenshots
  debug\recognition\<id>\   Per-attempt reports and images
  backups\                   Settings saved before restoration
  journal\packs\<packId>\    Imported maps
  login-data\idv-login\      Independent login-component data
```

Dated logs retain the latest seven days. The interface shows the current session, up to 99,000 entries at 1,000 per page; clearing it does not delete disk logs. Normal mode simplifies presentation only; developer mode expands and searches detailed fields from the same redacted records. Recognition triggers, helper startup, capture, localization, matching, candidates, elapsed time, cancellation and failures enter the runtime log. Image binary and credentials are excluded from text logs. Check logs and screenshots for personal information before sharing them.

Diagnostics discover windows, capture frames manually and inspect templates. Diagnostic input needs separate enabling and is unavailable while a task runs or is paused.

Failure screenshots default on and can be disabled in Settings. An error pause or window-recovery timeout saves at most one most recent successful WGC game-window frame per failure, skipping when none is available. Automatic saving excludes DXcam/MSS screen-region frames to avoid capturing other windows covering the game. Filenames and logs preserve the actual capture time, which may precede the failure. Retention is limited to 20 images and 100 MiB total, with a 20 MiB per-image limit. The count limit applies only to failure images; all assistant-generated screenshots also expire after three days. Unrelated manually added files are preserved. Images stay local and may contain names or chat.

### Settings backup and restore

The Recognition and diagnostics section at the bottom of Settings exports assistant preferences and task parameters without accounts, maps or logs. Import accepts this feature's versioned JSON, up to 1 MiB, validating fields, types, version and hotkey. An unavailable imported game path retains the local value. Review Current / After apply differences in the centered dialog, then choose Confirm apply. Matching settings need no application. Running, paused or queued tasks block restoration; an idle engine stops and reads restored settings on its next start. Original settings are retained under `backups`; configuration writes use atomic replacement, and failed desktop preference application attempts rollback. Save exports outside the assistant data directory.

A configuration read failure blocks editing and offers Reload settings. Failed saves remain marked Settings not saved with a retry action. Result notices hold for two seconds, then fade over half a second; hover or keyboard focus retains them. Confirmation dialogs do not close automatically.

### Diagnostic bundles

The default filename is `IdentityV-diagnostics-YYYY-MM-DD-HHmmss-SSS.zip`, using the local date, time and milliseconds to distinguish exports.

Choose **Export diagnostics**, select sessions, preview, then choose a ZIP destination; nothing uploads automatically. No historical selection means the current session only. Multiple historical sessions may be added, with selections reset on each open. Logs cover all modules: startup, settings changes, tasks, updates, map operations and recognition. Each attempt has a separate `recognition/<request-id>/flow.redacted.json` with filters, stages, localization checks, per-template scores and acceptance/rejection reasons, timings and cancellation reasons. Every sliding-search position is not logged. Environment and redacted settings reflect the current preview, not historical snapshots. Account directories, credentials, private keys and component runtimes are excluded. Review the contents before sharing.

**Include screenshots in diagnostics** defaults off and remembers your choice. It applies to the current and all selected historical sessions. Enabled exports include available images from the past three days: full windows (`full.png`), crops (`region_*.png`), processed edges/masks (`edges_*.png`, `mask_*.png`), exact prepared template edges (`template_*.png`), final regions (`region.png`) and session debug/failure images. Later attempts do not replace earlier evidence; cancellation or failure retains evidence already received. Images are not text-redacted. Sessions aged three to seven days contain logs and recognition text reports only. Legacy logs without session IDs appear separately by date, without inferred image ownership.

Current-only ZIPs are limited to **100 MB**; adding historical sessions removes the total size limit. Logs and reports take priority; remaining capacity includes the newest complete image groups first. Oversized or missing groups are omitted as a whole, without reducing image quality. Preview and `manifest.json` list omissions and reasons. Preview streams files into a frozen temporary ZIP; return to selection and preview again to include later operations. Saving or closing releases that temporary file. Local screenshots expire after three days, logs and recognition reports after seven. The running app prunes only its own expired files and retries locked files later. Settings → Recognition and diagnostics offers log and screenshot folder buttons.

The recognition feedback button has been removed; previously saved feedback remains with its evidence. Developers can use the [offline replay command](development.md#offline-diagnostic-replay) with the captured frame and prepared templates, without operating the game or automatically adjusting thresholds.

## Updates and validation limits

Assistant and login-component updates are separate. Assistant startup checking can be disabled; a new version is shown for review before download or installation. All sources enforce signature, signed version, installer URL, SHA-256 and size validation. GitHub direct is the default, with three manually selected proxies. Version 0.1.8 removes automatic probes. Speed and remaining-time estimates use the last five seconds of actual byte changes; stalled transfers show an unknown remaining time.

Full component installation reuses assistant download progress: bytes, percentage, speed and estimated time, with separate extraction, verification, configuration and proxy recovery stages. While it runs, closing the dialog, leaving the page or exiting is blocked. Download/verification can be cancelled; configuration and rollback cannot. Completion or cancellation releases the dialog; minimizing remains available.

Assistant installers do not include or apply login-component hotfixes. Full components use separate version directories, preserve external installations, accounts and the old runtime, and attempt rollback if configuration or owned-proxy restoration fails. Reminder suppression applies to a full release version; new versions prompt again and manual checks remain available.

See [Updates and releases](updates_EN.md) for repository migration, legacy compatibility and publishing. Tests cover screenshot recognition, logic, mocked IPC and package integrity. Validation of 0.1.9 and 0.1.10 does not cover installation, native-app execution, live account authentication or live gameplay.
