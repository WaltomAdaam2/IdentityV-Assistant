# Journal entry defaults and concise component logs

Status: resolved
Type: task

## Scope

- Open map filters on each journal entry; do not reuse a stored collapsed state. Keep map choices persistent and the floor temporary.
- Reuse the normal/full log views. Normal mode presents the existing launcher outcome instead of repetitive component startup instructions; retain actionable warnings, errors and login prompts. Developer mode shows the original detailed session history.
- Verify journal reentry through the existing browser seam and log filtering through its existing exported interface, using the user's recommended-default authorization. Preserve archived/full log data and unrelated package changes.

## Progress

- Journal filter collapse is now local page state, defaulting open even with an old stored true value. Map choices still save silently; floor resets to 1F and collapse resets to open on reentry.
- Normal logs hide recognized idv-login startup/config/certificate/URI chatter and its fixed standalone-launch instructions. The existing launcher result provides the summary. Unknown information, login prompts, actionable warnings, errors and critical errors remain visible.
- Developer mode directly selects the full retained session history; removed the redundant detailed-log switch. Filtering never mutates raw history or daily files. Updated the settings help text, including task-only window layout behavior.
- Journal and log-filter regressions failed before their respective fixes, then passed. Seven log unit tests and two browser cases passed (journal defaults/reentry; changing to developer mode reveals retained detail and preserves the result). Frontend production build passed.
- Text suppression is deliberately limited to recognized upstream messages; new or changed upstream wording remains visible until reviewed. Browser tests use the mocked native bridge, not a live login session.
