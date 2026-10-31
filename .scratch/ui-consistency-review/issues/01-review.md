# Confirm UI decisions and implement the agreed surfaces

Status: claimed
Type: improvement

## Progress

- Read all six screenshots and invoked the requested skill workflow. First six
  concrete questions sent through the structured question popup. Q1 confirmed:
  preserve the existing cream/olive design. Q2-Q6 were reissued as selectable
  popups on request and all recommended options explicitly confirmed.
- Grilling-directed read-only investigations delegated for full UI/motion inventory
  and repeated component update prompts. No runtime/game operations authorized.
- Impeccable context loaded successfully: incumbent CSS/components are authority;
  no PRODUCT.md/DESIGN.md, scoped refinement allowed. Mechanical detector required
  after UI implementation. No design hook installed or enabled.
- Repository instructions updated with all six phases and ponytail on code changes.
- UI implementation, final render/accessibility/motion checks and user decision
  completion remain outstanding. Goal is not complete.
- Read-only component investigation found a provable false-success risk: the
  worker verifies only upstream in-memory records, while upstream cached writes
  may fail without raising. It omits the upstream cache-write probe. Proposed
  behavior: probe before mutation, verify persisted records/pending IDs before
  reporting success, and cover write failure with an isolated fixture.
- Local logs contain two update-success events at 06:15:06 and 08:17:15 UTC on
  2026-10-31, but no per-module persistence evidence. A whitelisted read of the
  component's config still failed at the Windows ACL; no credentials were read
  and no permissions were changed. This does not establish the local root cause.
- Normal pending/applied/skipped IDs already suppress candidate updates; both
  layers use the same version/module/commit ID. Any new prompt preference should
  remain distinct from the official hotfix_skipped rollback mechanism.
- Q7-Q32 decisions recorded, including Q16's explicit start button after successful
  retry, retaining/extending decorative card/bubble motion, and locking the update
  dialog while downloading until cancellation. No product-choice questions remain.
- First implementation slice: import/diagnostic previews are native centered
  dialogs, import labels are readable and use Apply, no-difference preview has
  Close only, and operation results go through the shared top notice. Screenshot
  inclusion remains a separate opt-in; original settings backup remains intact.
- Automatic notices now hold 2 seconds and fade for 500ms, collapsing their grid
  row/margins during the same interval. Settings-saved feedback uses the same
  lifetime. Hover/focus can rescue a fading notice and restart its hold duration.
- Verification: frontend typecheck/production build passed. Five focused browser
  regressions passed: notice close, repeated/focused notices, exact hold/fade with
  continuous layout collapse, import/diagnostic consent, and no-difference/error
  feedback. The timing regression failed against the old behavior before the fix.
- Inspected Chinese light previews at 1000x680 and English dark diagnostics at
  667x453 CSS px. No horizontal modal overflow; Escape returns to the import
  trigger. This is mock IPC/browser evidence, not native-file-picker validation.
- Independent review of this slice is in progress; component update persistence,
  suppression, launcher motion and the remaining audit items are still incomplete.
- Independent review found two concrete defects in the first slice, both fixed:
  enum translations are restricted to enum fields (literal window keywords such
  as dark/light remain unchanged), and manual dismissal cannot be undone by
  hover/focus during fade. Focused regressions passed for both.
- Component update suppression now records the exact version/module/commit batch
  separately from upstream rollback data. A browser regression passed for ignored
  launch prompts, manual availability and a new commit prompting again.
- Hotfix worker now strictly writes the existing hotfix_probed key before module
  mutation, using require_saved=True verified in official v6.3.2-stable envmgr.py.
  After application it reads config.json and compares requested records plus
  pending IDs/restart bookkeeping. Six offline tests passed; the optional official
  runtime fixture remains skipped. The old worker failed the missing-record cases.
- Latest frontend production build and diff check passed. This is a verified
  checkpoint, not completion: launcher/settings/task controls, motion and the
  remaining Q7-Q32 work still require implementation and validation.
