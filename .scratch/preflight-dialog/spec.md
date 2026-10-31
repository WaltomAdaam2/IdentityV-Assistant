# Task preflight dialog

Replace the persistent task preflight card with a dismissible native modal dialog.
Keep the existing palette and controls, with more space between title, result,
optional warning and actions. Close with Escape, the close button or the corner
button; entering capture diagnostics also dismisses the dialog.

Successful automatic preflight continues without a modal, including non-blocking
window warnings. Failed automatic and manual checks show their result. Engine
checks and task-start decisions remain unchanged. Work on SNAPSHOT-v0.1.10;
preserve unrelated pnpm edits and the signed v0.1.9 draft.

Verify the existing mock-IPC preflight regression, dialog dismissal/navigation,
automatic-success behavior, frontend types and screenshot spacing. No native run.
