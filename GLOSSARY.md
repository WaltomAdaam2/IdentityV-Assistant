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
A user-saved ZIP containing a bounded snapshot of redacted current-session diagnostics. Screenshots require separate selection; login-component account data is excluded.

**Journal map identity**:
A map's season, difficulty, door category and number. Different floor views and base/route illustrations can belong to the same map.

**Recognition candidate**:
A journal map whose first-floor geometry plausibly matches the visible game map. A candidate requires the user's selection; it is not a confirmed identification or an accuracy percentage.
