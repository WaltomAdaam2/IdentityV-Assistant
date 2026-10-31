# Align download metadata below the progress bar

Status: resolved
Type: improvement

## Progress

- Confirmed the previous version is merged/frozen; created SNAPSHOT-v0.1.10 from current main.
- Moved the shared progress details into one wrapping flex row: size/percentage left, inline speed/time right. Calculations and update lifecycle unchanged.
- `pnpm typecheck`, three existing download-progress tests and `git diff --check` passed.
- Mocked Chromium page: Chinese and English size/percentage, speed and remaining time share the same baseline below the full-width bar; the transfer group ends at the bar's right edge.
- At 240 px component width, the transfer group wraps intact, remains right-aligned and does not overflow. Screenshot visually checked in Chinese; geometry checked in both languages.
- No native assistant launch, installation or live download was performed. Existing signed v0.1.9 draft is unchanged; this is source preparation for the next version.
