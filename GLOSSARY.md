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
