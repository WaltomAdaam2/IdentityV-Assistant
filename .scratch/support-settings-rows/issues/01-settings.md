# Match backup and diagnostics to settings rows

Status: resolved
Type: improvement

## Progress

- The previous standalone panel duplicated settings styling and appeared above the settings form.
- Move SupportPanel into SettingsPage's recognition section through a render prop, preserving parent-owned save/restore coordination.
- Reuse SettingRow for export, import/preview, optional images and diagnostic preview. Keep previews directly below their actions.
- Also moved the existing failure-screenshot setting into Recognition and diagnostics, so new settings use the same section and SettingRow presentation.
- Type-check/build and existing restore/diagnostic regression passed: preview-before-restore, retained restored settings, separate image consent, invalidated preview after consent change, ZIP export and active-task restore guard. Chinese/English settings screenshots checked at 1000x680; native dialogs/export were mocked.
