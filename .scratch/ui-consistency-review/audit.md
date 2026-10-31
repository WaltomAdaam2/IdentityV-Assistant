# Frontend decision inventory

Baseline: SNAPSHOT-v0.1.10, e7713e1. Read-only review; no native app or game run.
These are findings/proposals, not approved changes. Q1 preserves the incumbent
cream/olive palette, typography and settings rows. Q2-Q6 recommendations were
subsequently confirmed; additional proposals below remain pending.

The six screenshot topics are tracked in spec.md. The additional findings below
must be presented individually for user decisions before optional implementation.
Line numbers refer to the baseline, not later changes.

## Behavior and accessibility findings

| ID | Evidence | Proposed decision |
| --- | --- | --- |
| A01 | App.tsx:1017 swallows configuration-load failure; defaults remain visible and autosave never enables. | Show a persistent load error and retry; avoid editing values that cannot save. |
| A02 | App.tsx:1285 swallows event-subscription failure; stale status can remain interactive. | Show status unavailable and a reconnect action, without repeated dialogs. |
| A03 | App.tsx:952,1120,1353,2053 send errors through default non-error notices (1 second/status). | Route failures through the existing error channel (8 seconds/alert). |
| A04 | App.tsx:1353 leaves failed autosave edits looking saved; :2417 puts bottom-operation errors at the page top. | Mark unsaved values persistently; show action errors where they occur or through agreed feedback. |
| A05 | App.tsx:1853 task switches have no associated task-name label. | Add accessible enable-task labels. |
| A06 | App.tsx:2002,2312,2326 filters/command fields lack persistent accessible names. | Label them and associate JSON validation errors. |
| A07 | App.tsx:1927-1938 clamps numbers during each keystroke; other settings turn empty strings into zero. | Keep temporary input text; validate on blur/confirmation. |
| A08 | App.tsx:2410,2513,2522,2531 splits/trims/filters keywords on each keystroke. | Preserve typed delimiters until committing the field. |
| A09 | App.tsx:1802,1868,2041 only disables task/diagnostic actions while pending. | Add operation-specific checking/capturing labels and busy state. |
| A10 | App.tsx:1615 allows repeated preflight retries; successful retry only offers close. | Prevent duplicate checks; ask whether success should offer starting the original task. |
| A11 | App.tsx:1900 queued-card Stop actually stops the active task and clears the entire queue. | State the global effect or limit the visible stop action to the active task; no new queue-removal feature. |
| A12 | App.tsx:1665,1676 allow repeated lifecycle submissions; restart is always enabled. | Guard pending lifecycle actions; ask whether restarting an active task requires confirmation. |
| A13 | Journal.tsx:175,183 unmounts the focused map button without moving/restoring focus. | Manage focus between list and details. |
| A14 | Journal.tsx:93,191 uses a custom modal without inert background; capture-phase key handlers can compete with a native dialog. | Reuse native modal behavior or restrict keys to the top layer; reproduce stacked layers before claiming a runtime failure. |
| A15 | index.html:2 fixes lang=zh-CN; App.tsx:1303 only updates theme. | Synchronize document language with the chosen UI language. |
| A16 | styles.css:183 muted text #8a7c6d measures 3.76:1 on #faf6ef and 3.31:1 on the background. | Darken only the small-text token to meet 4.5:1; preserve the palette. |
| A17 | App.tsx:2565 exposes idle_fps; engine references are schema/loader/tests with no runtime consumer. | Hide or explicitly disable the unused control; do not add scheduling behavior for a UI audit. |
| A18 | App.tsx:2641 labels a repository link, while commands/mod.rs:7 opens releases. | Align the label/destination. UpdatePanel's release link already reaches releases. |

## Motion and discovery decisions

| ID | Evidence | Proposed decision |
| --- | --- | --- |
| A19 | styles.css:124 exit state prevents pointer events but not keyboard interaction. | Make departing content inert for the transition. |
| A20 | UpdatePanel.tsx:120 and App.tsx:1609 close native dialogs immediately after animated entry. | Add a short consistent exit; immediate close under reduced motion. |
| A21 | styles.css:123,125,148 layers page/header/panel entry motions with 420-640 ms durations. | Retain one restrained main transition; evaluate rendered timing. |
| A22 | styles.css:145,147,178 adds sweep, hover lift and perpetual icon movement to noninteractive content. | Ask whether to remove/reduce these decorative effects. |
| A23 | Journal.tsx:156 and journal.css:2 hide filter content immediately while width animates for 460 ms. | Make open/close symmetrical or use direct switching. |
| A24 | UpdatePanel.tsx:120,168 allows closing during download; Settings still says Check for updates. | Keep a discoverable View download progress/Ready to install entry. |

## Risks requiring rendered evidence

- R01: Overview/service/diagnostics retain two columns at narrow content widths
  (styles.css:154,170). Check 1000x680 plus 150% zoom before changing breakpoints.
- R02: Logs have a fixed 260px search field, nonwrapping toolbar and fixed-width
  metadata columns (styles.css:173). Check long paths and narrow windows.
- R03: Launcher stacked layout retains a half-width news column and absolute
  logo/status positioning (launcher.css:53). Check English and zoomed layout.
- R04: No color-scheme declaration. Check dark native controls/scrollbars in the
  mock browser; browser evidence does not prove native WebView rendering.

## Existing behavior to preserve

- Map fullscreen already has a Tab loop, Escape, repeat-key guard, return focus
  and a 220ms exit. Avoid claiming it lacks all focus/keyboard behavior.
- Global reduced-motion rules exist. Logs and transfer numbers need no animation.
- Persistent map-library errors in details, diagnostic parameters, and local
  field-validation errors are useful inline content, not automatic modal candidates.
- Updates and preflight already use native modal semantics. SettingRow already
  provides title/description ARIA relationships.

Guideline reference checked during this audit:
https://raw.githubusercontent.com/vercel-labs/web-interface-guidelines/main/command.md

## Mock-browser follow-up

At 1000x680, the overview renders without horizontal overflow. A 667x453 CSS
viewport approximates the content area of 150% zoom; this is not a native DPI test.
Overview content becomes dense but no clipping was demonstrated, so R01 remains
an optional layout question rather than a confirmed fault.

R02 reproduced: the clear-session-log button wraps into a tall single-character
column next to the fixed-width search controls. Horizontal scroll width alone
does not detect this. R03 reproduced as a visual density issue: in the stacked
layout, the news column remains 213.5px within a 483px workspace; the Chinese
header and setup controls did not overlap in this fixture. English is pending.

Screenshots (local artifacts, not source assets):
`C:/Users/Walto/.codex/visualizations/2026/10/30/01a18c21-06f4-7c92-86c5-77745992fe0f/ui-audit/`
- overview-1000.png, overview-narrow.png
- logs-narrow.png, logs-long-path.png
- launcher-narrow.png, launcher-setup-narrow.png
