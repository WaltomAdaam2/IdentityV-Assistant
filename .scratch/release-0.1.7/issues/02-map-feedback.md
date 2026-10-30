# Map import feedback through the existing top notice

Status: resolved
Type: bug

## Decisions and test seam

- User approved unified notices and autonomous minimal implementation; latest validation scope is build/offline only. Use the existing rendered React app with mocked native IPC. This covers click/results, timing, focus, page changes and layout; it does not prove native file-picker behavior or installation.
- Apply one TDD slice at a time, starting with a successful import producing a top notice rather than a fixed paragraph. Then cover partial failure, cancellation, repeated identical notices and navigation during import.
- Reuse App's existing notice owner. Persistent library validation errors stay on the journal page. Record completed import results in application logs so details remain accessible after fade.

## Evidence

- Journal.tsx currently owns message/error strings and renders fixed paragraphs; App supplies no notice callback.
- Historical commit 0d05305 already used fixed import paragraphs. a83b131 suppresses only the separate settings-saved toast for journal preferences; it did not remove an import popup.
- App's notice uses string identity for its timer, so the same message twice while visible does not restart its lifetime.

## Design

- Preserve the supplied reference and existing theme: paper #f1e7d8, notice #e4d8c7, outline #c8ae8d, text #513b2d, error #b85d50. Existing dark-theme variables remain supported.
- Keep Microsoft YaHei UI at the existing 13px text scale. Left-aligned message, close button at right: [ outcome and wrapped details                         x ].
- Same single top strip, no modal, new card layout or decorative animation. Keep 600ms fade, hover/focus rescue and reduced-motion support. Error outcomes get longer reading time, and long text wraps within the window.
- Review against reference: deliberately retain the existing palette/shape, because the user supplied that exact style; avoid introducing a new visual identity for one feedback path.

## Progress

- Investigation complete; implementation and verification pending.
- Successful-import top-notice regression failed on the original fixed paragraphs, then passed after routing outcomes through App. Notifications now carry fresh object identity so repeated identical messages restart expiry; import errors last eight seconds and results are retained in logs.
- Five focused offline browser tests passed: success/dismissal, mixed and total failure/cancel/log retention, completion after navigation, identical messages/focus, hover rescue during fade. TypeScript, production frontend build and seven log tests passed.
- Independent reviewer found an offscreen-result regression when completion occurs in a scrolled workspace. Added a deferred-import -> scrolled Settings regression, observed failure, then made the existing notice sticky inside its workspace. Rerun pending. No other actionable review findings.
- Scrolled-completion regression now passes, along with the existing silent journal-preference/floor-reset regression (2 passed). Reviewed 1000x680 screenshot of mixed import results: visible top strip, wrapped details, accessible close button, unchanged map layout. Screenshot: OS temp IdentityV-Assistant-v017-map-notice.png.
- Source coverage for results includes chooser cancellation, successful imports, partial/total errors, callback exceptions and open-directory/list errors. Browser/native-IPC mock tests do not prove real picker or desktop installation behavior.
