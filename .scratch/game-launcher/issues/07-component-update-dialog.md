# Component update dialog

Status: complete
Type: task

## Scope

- Present official component full updates and hotfixes using the same dialog appearance as assistant updates.
- Select GitHub direct, GH-Proxy, GHFast or ghproxy.net inside the dialog.
- Add a manual check button in launch settings.
- When automatic checking is enabled, check at game launch only, never at assistant startup. Preserve the assistant's own update preference and behavior.
- Reuse upstream update/hotfix implementation where possible; preserve accounts, hidden process ownership and cancellation. Do not confuse the game's launcher-update API with component updates.

## Evidence

- Official v6.3.2-stable has no component update CLI/HTTP endpoint. CloudRes exposes version/downloadUrl/detail/critical_update and hotfix entries. Latest verified release is still v6.3.2-stable.
- Existing UpdatePanel provides the required dialog styling and source list.

## Implementation and verification

- Added a separate component update dialog reusing the assistant's update styles and layout. Source changes persist through the existing download-source preference.
- The independent component auto-check preference defaults on and runs only from the game launch action. Page mount and assistant startup do not check. Manual checks live in launch settings. Closing the prompt cancels the pending launch; the explicit continue button or successful hotfix application resumes it.
- Read-only checks fetch official GitHub release/cloud metadata and filter the confirmed snapshot against the component's local applied/skipped records. The worker receives only confirmed module/commit items and uses the official hotfix implementation for backups, compilation, replacement and validation records. It uses the selected source without silently falling back to a different host.
- Bootstrap delegates automatic update decisions to the assistant, avoiding a second Qt prompt/countdown. Official rollback/import hooks remain active.
- Applying pauses only the owned proxy and restores it on the same port after success or failure. Exit requests cancel worker downloads before module replacement; completed replacements retain pending validation records.
- Full-release entry opens the official Windows asset through the selected source. Automatic replacement of a future, unreviewed component version is not implemented; the existing v6.3.2-stable compatibility boundary remains explicit in the dialog. No official installer is executed.
- Browser regression initially failed on the absent prompt, then passed. Verified no app/page startup check, check-before-launch, source passed to hotfix apply, manual check and disabled-auto-check persistence. Fixed a cancellation UI race found by the existing suite; targeted rerun passed.
- Rust final library run: 57 passed, 7 intentionally ignored. Separately ran the public metadata test through GH-Proxy successfully. Official embedded Python: 4 bootstrap tests and 3 hotfix-worker tests passed, including harmless temporary-module backup/replacement and stdin cancellation before mutation.
- TypeScript and Vite production build passed. Rendered popup inspected. Independent review findings (worker exit wait and failed-update proxy recovery) were fixed and reviewed again with no remaining concrete findings.
- Real account login, real installation hotfix application and native shutdown during a live update were not exercised. User's earlier successful proxy login is separate evidence.

## Follow-up

- Issue 08 supersedes the optional component-check preference: remove the toggle and legacy values, always check on game launch, and retain the update confirmation dialog. The assistant's own startup update preference is unchanged.
