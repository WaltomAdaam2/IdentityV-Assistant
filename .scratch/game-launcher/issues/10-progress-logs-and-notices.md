# Useful progress logs and dismissible notices

Status: resolved
Type: task

## Scope

- User requests 1-2 assistant startup records and 3-4 game launch records, rather than hiding the entire process. Show UI readiness and loaded configuration, then update check, launch/proxy preparation and the existing native launch result.
- Top notices wait one second and fade slowly. Hover or keyboard focus retains them, including during fade-out; leaving restarts the timer. Manual close remains available.
- Reuse existing log storage, notice styling and browser test seams. Keep detailed component chatter in developer mode and preserve real errors. Apply frontend-design and ponytail without a broader UI rewrite.

## Progress

- Normal logs now start with UI readiness and successful configuration loading. Each game launch adds update checking and game/proxy preparation before the existing native launch result, giving three useful steps without duplicating upstream chatter.
- Notices wait 1000 ms and fade opacity over 600 ms. Pointer hover or keyboard focus cancels expiry and restores a fading notice; leaving restarts the wait. Manual close dismisses immediately. Reduced-motion users skip the fade.
- Removed the exit rule that disabled pointer events. This allows a notice already fading to receive hover. Long-error layout testing holds the notice while measuring instead of racing its new expiry.
- Startup-count regression failed before the fix. Three final browser cases passed: two startup plus three launch logs; hover rescue during fade and expiry after leaving; long-error layout. Seven log tests and frontend production build passed.
- Full raw session logs and daily logs remain intact. Native game geometry under the still-running old assistant is a separate unresolved runtime condition documented in window-restoration issue 03.
