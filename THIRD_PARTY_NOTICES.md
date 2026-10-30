# Third-party components

Current first-party source and releases starting with v0.1.7 are licensed under
GPL-3.0-only, Copyright (C) 2026 WaltomAdaam2. This does not replace third-party
licenses, copyright notices or grants. Archived v0.1.0-v0.1.6 installers retain
the notices distributed inside those binaries; copying a historical installer
to a different release repository does not rewrite its contents or license.

The installer includes `engine/_internal/THIRD_PARTY_LICENSES.txt`, generated
from the installed Python dependencies, locked Rust crates, and production
JavaScript packages. This conservatively includes some build-only dependencies.
The Python runtime license and PyInstaller bootloader distribution exception
are included. PyInstaller's exception permits distribution of non-free programs.

The optional, independently installed login component is
[KKeygen/idv-login](https://github.com/KKeygen/idv-login), currently integrated
against [v6.3.2-stable](https://github.com/KKeygen/idv-login/tree/v6.3.2-stable).
Its original [GPL v3 license](https://github.com/KKeygen/idv-login/blob/v6.3.2-stable/LICENSE),
authors and notices remain applicable. The assistant does not claim authorship
of its login protocol implementation. The component is downloaded separately;
it is not embedded into the assistant's first-party code.

Launcher artwork, Identity V names, logos and game screenshots belong to their
respective rights holders. See [launcher artwork sources](frontend/public/launcher/SOURCES.md).
The application's GPL grant does not relicense those materials or user-imported
map packs. Map-pack authors retain their rights; obtain their permission before
redistributing their work.

The optional OpenCV FFmpeg video plugin is excluded from this installer; image
processing and screen capture do not require it. Shapely includes GEOS under
LGPL-2.1. This component remains under its original license, including rights to
modify and redistribute them and to reverse engineer the application for
debugging modifications to those components. Their shared libraries remain
separate files in the engine installation and can be replaced with compatible
modified versions. No application license restriction overrides these rights.

The installation includes the unmodified GEOS 3.13.1 sources and Shapely 2.1.2
sources/build scripts in `engine/_internal/third-party-sources/`, with SHA-256
checksums and upstream provenance in `SOURCES.txt`. The build rejects a changed
Shapely/GEOS version until its corresponding-source archive is updated.

Upstream source and build instructions:

- OpenCV wheels and their bundled FFmpeg build: https://github.com/opencv/opencv-python
- FFmpeg: https://ffmpeg.org/download.html
- Shapely wheels: https://github.com/shapely/shapely
- GEOS: https://libgeos.org/usage/download/
- Python: https://www.python.org/downloads/source/
- PyInstaller: https://github.com/pyinstaller/pyinstaller

The exact component versions and original notices are listed in the generated
file. The included source archives, rather than these links alone, accompany
the redistributed GEOS library.

Interface icons use Lucide from the Iconify collection (https://github.com/iconify/icon-sets).
Original ISC and MIT notices are included in `assets/icons/LUCIDE_LICENSE.txt`.
