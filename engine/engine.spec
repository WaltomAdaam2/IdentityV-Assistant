# PyInstaller folder bundle installed next to the Tauri executable.
from pathlib import Path
from PyInstaller.utils.hooks import collect_all

root = Path(SPECPATH).parent
datas = [(str(root / "config" / "default.json"), "config")]
datas.append((str(root / "engine/build/THIRD_PARTY_LICENSES.txt"), "."))
datas.append((str(root / "engine/build/third-party-sources"), "third-party-sources"))
for file in (root / "assets" / "templates").rglob("*"):
    if file.is_file() and "reference" not in file.parts and file.suffix in (".png", ".json"):
        datas.append((str(file), str(file.parent.relative_to(root))))
binaries = []
hiddenimports = ["mss", "mss.windows", "capture.mss_capture", "capture.dxcam_capture",
                 "capture.wgc_capture", "win32api", "win32gui", "win32con", "win32process"]
for package in ("windows_capture", "dxcam", "rapidocr_onnxruntime", "onnxruntime"):
    package_data, package_binaries, package_imports = collect_all(package)
    datas += package_data
    binaries += package_binaries
    hiddenimports += package_imports

a = Analysis([str(Path(SPECPATH) / "main.py")], pathex=[SPECPATH],
             binaries=binaries, datas=datas, hiddenimports=hiddenimports,
             excludes=["pytest", "PyInstaller"], noarchive=False)
# This image/capture application never uses FFmpeg video decoding/encoding.
# OpenCV explicitly supports omitting this optional, dynamically loaded plugin.
a.binaries = [entry for entry in a.binaries
              if not Path(entry[0]).name.lower().startswith("opencv_videoio_ffmpeg")]
pyz = PYZ(a.pure)
exe = EXE(pyz, a.scripts, [], exclude_binaries=True,
          name="identityv-engine", console=True, upx=False)
coll = COLLECT(exe, a.binaries, a.datas, name="identityv-engine", upx=False)
