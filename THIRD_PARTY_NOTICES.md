# Third-party notices

## AhabAssistantLimbusCompany (AALC)

Portions of `engine/window/windows.py` that manage a game window's frame,
topmost state, size, and restoration are adapted from AALC's
[`module/game_and_screen/screen.py`](https://github.com/KIYI671/AhabAssistantLimbusCompany/blob/main/module/game_and_screen/screen.py).

Copyright (c) the AhabAssistantLimbusCompany contributors.

The upstream project declares
[`AGPL-3.0-or-later`](https://github.com/KIYI671/AhabAssistantLimbusCompany/blob/main/pyproject.toml)
and includes the
[GNU Affero General Public License version 3](https://github.com/KIYI671/AhabAssistantLimbusCompany/blob/main/LICENSE).
The copied sequence was adapted to use this project's existing window handle
discovery and to restore the exact window state captured before automation
starts.

Source revision: `5492a9cddcad978cb69d967f7bba8908a6c01970` (screen.py).
The combined application is distributed under AGPL-3.0-or-later; original
Identity V Assistant code was published under GPL-3.0-only. Existing rights
to those earlier versions are unchanged.
