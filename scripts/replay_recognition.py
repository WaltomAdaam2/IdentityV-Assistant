"""Offline replay of one diagnostic session; no capture, game input or extraction.

Run with engine/.venv/Scripts/python.exe scripts/replay_recognition.py bundle.zip
Uses the captured frame and the exact prepared template edges in the bundle.
"""
from __future__ import annotations

import argparse
import json
import math
from pathlib import Path, PurePosixPath
import struct
import sys
from time import perf_counter
from zipfile import BadZipFile, ZipFile

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "engine"))
import cv2
import numpy as np
from vision.journal_matcher import MAX_PIXELS, TEMPLATE_SIDE, Template, recognize

def read_member(bundle, name, limit):
    info = bundle.getinfo(name)
    if info.file_size > limit:
        raise ValueError(f"Member exceeds its size limit: {name}")
    with bundle.open(info) as source:
        data = source.read(limit + 1)
    if len(data) > limit:
        raise ValueError(f"Member exceeds its size limit: {name}")
    return data


def read_image(bundle, name, grayscale=False):
    data = read_member(bundle, name, 64 * 1024 * 1024)
    if len(data) < 24 or data[:8] != b"\x89PNG\r\n\x1a\n" or data[12:16] != b"IHDR":
        raise ValueError(f"Invalid PNG: {name}")
    width, height = struct.unpack(">II", data[16:24])
    if min(width, height) < 1 or width * height > MAX_PIXELS:
        raise ValueError(f"Image dimensions exceed limit: {name}")
    image = cv2.imdecode(np.frombuffer(data, np.uint8), cv2.IMREAD_GRAYSCALE if grayscale else cv2.IMREAD_COLOR)
    if image is None:
        raise ValueError(f"Cannot decode: {name}")
    return image


def replay_attempt(bundle, name):
    report = json.loads(read_member(bundle, name, 8 * 1024 * 1024))
    prefix = str(PurePosixPath(name).parent)
    if report.get("status") != "completed":
        raise ValueError("Original recognition did not complete; its template set may be incomplete")
    steps = report["steps"]
    indexed = next(step for step in steps if step.get("stage") == "indexed")
    prepared = [step for step in steps if step.get("stage") == "template_prepared"]
    rejected = [step for step in steps if step.get("stage") == "template_rejected"]
    frame = read_image(bundle, f"{prefix}/full.png")

    def templates():
        # With no usable ROI the matcher never consumes templates. This also
        # replays localization failures, but rejects missing data if it is needed.
        if len(prepared) + len(rejected) != indexed["template_count"]:
            raise ValueError("Missing template evidence; cannot reproduce the full search")
        for step in prepared:
            image = step["image"]
            if not isinstance(image, str) or PurePosixPath(image).name != image:
                raise ValueError("Invalid template member name")
            edges = read_image(bundle, f"{prefix}/{image}", grayscale=True)
            if max(edges.shape) != TEMPLATE_SIDE or not np.isin(edges, (0, 255)).all():
                raise ValueError("Invalid prepared template edges")
            distance = cv2.distanceTransform(255 - edges, cv2.DIST_L2, 3)
            coarse = cv2.resize(distance, (max(1, edges.shape[1] // 4), max(1, edges.shape[0] // 4)), interpolation=cv2.INTER_AREA) / 4
            yield Template(step["key"], edges, distance, coarse)

    before = next(step for step in reversed(steps) if step.get("stage") == "completed")
    started = perf_counter()
    results = recognize(frame, templates(), template_count=len(prepared))
    matches = [{"key": m.key, "region": vars(m.region), "geometry_cost": m.score,
                "coverage": m.coverage, "unexplained": m.unexplained} for m in results]
    same = len(matches) == len(before["matches"]) and all(
        a["key"] == b["key"] and a["region"] == b["region"]
        and all(math.isclose(a[k], b[k], rel_tol=1e-12, abs_tol=1e-12)
                for k in ("geometry_cost", "coverage", "unexplained"))
        for a, b in zip(matches, before["matches"]))
    return {"request_id": report["request_id"], "filters": report["filters"], "feedback": report.get("feedback"),
            "original_keys": before["keys"], "replayed_keys": [m.key for m in results],
            "exact_match": same, "comparison_tolerance": 1e-12, "original_matches": before["matches"], "replayed_matches": matches,
            "replay_ms": round((perf_counter() - started) * 1000), "original_total_ms": before["elapsed_ms"],
            "timing_note": "Replay excludes capture and evidence persistence. Original total starts inside recognition and includes capture, template preparation and emitted evidence; neither timing includes helper startup.",
            "scope": "Localization and geometry matching from stored pixels and prepared templates; no native capture or gameplay."}


def replay(path: Path, attempt: str | None = None):
    output = []
    with ZipFile(path) as bundle:
        members = bundle.infolist()
        names = [member.filename for member in members]
        # Historical-session bundles have no aggregate size limit. Each frame
        # and report is still bounded and consumed one attempt at a time.
        if len(names) != len(set(names)):
            raise ValueError("Duplicate members")
        if any(member.flag_bits & 1 or ".." in PurePosixPath(member.filename).parts or "\\" in member.filename
               or PurePosixPath(member.filename).is_absolute() for member in members):
            raise ValueError("Unsupported or unsafe ZIP members")
        reports = sorted(name for name in names if name.startswith("recognition/") and name.endswith("/flow.redacted.json")
                         and (attempt is None or PurePosixPath(name).parent.name == attempt))
        if not reports:
            raise ValueError("No matching per-attempt report. Older bundles cannot be replayed completely.")
        for name in reports:
            try:
                output.append({"status": "replayed", **replay_attempt(bundle, name)})
            except (KeyError, ValueError, TypeError, StopIteration, cv2.error) as error:
                output.append({"request_id": PurePosixPath(name).parent.name, "status": "not_replayable", "reason": str(error) or "Missing completion evidence"})
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("bundle", type=Path)
    parser.add_argument("--attempt", help="Replay only this request ID")
    parser.add_argument("--output", type=Path, help="Write a JSON comparison report")
    args = parser.parse_args()
    try:
        results = replay(args.bundle, args.attempt)
    except (OSError, ValueError, BadZipFile) as error:
        parser.exit(1, f"Cannot replay: {error}\n")
    text = json.dumps(results, ensure_ascii=False, indent=2)
    if args.output:
        args.output.write_text(text + "\n", encoding="utf-8")
    else:
        print(text)
    return int(any(result["status"] != "replayed" or not result["exact_match"] for result in results))


if __name__ == "__main__":
    raise SystemExit(main())
