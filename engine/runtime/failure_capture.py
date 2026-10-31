from pathlib import Path
import re
from uuid import uuid4


def save_failure_image(image, captured_at, directory: Path):
    """Retain only this feature's own images; never prune manual captures."""
    import cv2
    if directory.is_symlink():
        raise ValueError("Failure capture directory must not be a link")
    directory.mkdir(parents=True, exist_ok=True)
    ok, encoded = cv2.imencode(".png", image)
    if not ok or len(encoded) > 20 * 1024 * 1024:
        raise ValueError("Failure screenshot could not be encoded within 20 MiB")
    existing = sorted((p for p in directory.iterdir()
                       if re.fullmatch(r"failure-\d{8}T\d{12}Z-[a-f0-9]{8}\.png", p.name)
                       and not p.is_symlink() and p.is_file()), key=lambda p: p.name)
    total = sum(p.stat().st_size for p in existing)
    while existing and (len(existing) >= 20 or total + len(encoded) > 100 * 1024 * 1024):
        oldest = existing.pop(0)
        size = oldest.stat().st_size
        oldest.unlink()
        total -= size
    path = directory / f"failure-{captured_at:%Y%m%dT%H%M%S%fZ}-{uuid4().hex[:8]}.png"
    with path.open("xb") as target:
        target.write(encoded.tobytes())
    return path
