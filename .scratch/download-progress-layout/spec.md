# Download progress metadata layout

Move speed and remaining time to the lower-right of the progress bar, alongside
downloaded/total bytes and percentage at the left. Use the shared component for
assistant and login-component updates. Preserve transfer calculations and allow
wrapping when the available width cannot fit one row.

Version 0.1.9 is merged and frozen. Implement on SNAPSHOT-v0.1.10 from main
eed2925; this request adjusts source layout, not the existing signed draft.

Verify frontend types, existing transfer behavior and browser geometry in both
languages. Do not launch the native assistant or alter unrelated pnpm edits.
