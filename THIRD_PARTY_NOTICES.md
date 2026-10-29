# Third-party components

The proprietary license for this application's original code does not replace
the licenses of third-party components. Their copyright notices, license texts,
and license grants remain applicable. Earlier GPL-licensed releases retain
their original license grants.

The installer includes `engine/_internal/THIRD_PARTY_LICENSES.txt`, generated
from the installed Python dependencies, locked Rust crates, and production
JavaScript packages. This conservatively includes some build-only dependencies.
The Python runtime license and PyInstaller bootloader distribution exception
are included. PyInstaller's exception permits distribution of non-free programs.

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
