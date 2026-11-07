# Identity V Assistant

The desktop assistant provides visual automation, a game launcher, and journal map browsing for Identity V.

## Language

**Game launcher**:
The application module that locates the Identity V executable and coordinates direct launch with the login component. It is shared by the launch page and Settings.
_Avoid_: Fever launcher, automation engine

**Login component**:
An independently running upstream idv-login program that handles game login traffic and remembered accounts. Its readiness does not prove successful account authentication.
_Avoid_: acceleration proxy, automatic account login

**Automation engine**:
The Python process that recognizes game screens and performs configured visual tasks. Its lifecycle is separate from the login component.
_Avoid_: game launcher, login component

**Assistant update**:
A signed installer update of this application, distinct from an update to the independently managed login component.

**Migration bridge**:
The one-time 0.1.7 release in the old download repository. Its old-repository manifest lets compatible existing clients install the same signed binary that switches future checks to the source repository.

**Task preflight**:
A window/response/capture check before a newly requested task or queue. It does not authenticate accounts or prove gameplay readiness.

**Failure screenshot**:
The most recent successful WGC game-window frame saved when a task fails. Its capture timestamp may precede the failure; screen-region backends are excluded.

**Diagnostic bundle**:
A user-saved ZIP containing a frozen snapshot of redacted diagnostics. It includes the current session by default; retained historical sessions can be added explicitly before preview. Current-only exports are limited to 100 MB; selecting history removes the total size limit. Screenshots require separate selection; login-component account data is excluded.

**Journal map identity**:
A map's season, difficulty, door category and number. Different floor views and base/route illustrations can belong to the same map.

**Recognition candidate**:
A journal map whose recognition-floor geometry plausibly matches the visible game map: 1F for S0, the detected B1/B2/B3 for S1. S0 Auto detects difficulty from captured floor labels; manual Hard/Nightmare selects that pool directly. S1 uses the selected difficulty because its difficulties share underground floors. Unclear or unsupported selections stop automatic floor detection. Candidates and automatic entry display the recognition floor. The interface shows up to four in match order. Optional automatic entry opens the first candidate; this is not a confirmed identification or an accuracy percentage.

**Recognition evidence**:
The captured frame, localized and processed regions, exact prepared templates and per-map analysis for one request. Images expire after three days, reports after seven; diagnostic exports select the current app session and any explicitly added historical sessions.

**Recognition feedback**:
A local assessment saved by earlier builds, optionally naming the correct map. Existing records remain part of exported evidence and offline replay; the interactive feedback entry has been removed. Records are not uploaded or used to adjust thresholds.
