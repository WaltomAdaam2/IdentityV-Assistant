# Component update dialog

Status: claimed
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
