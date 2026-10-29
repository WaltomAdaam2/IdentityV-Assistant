"""Collect license texts from the exact installed/locked build dependencies."""
from __future__ import annotations

import json
import hashlib
import os
from importlib.metadata import distribution
from pathlib import Path
import sys
import tomllib
import urllib.request

from packaging.requirements import Requirement

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "engine/build/THIRD_PARTY_LICENSES.txt"


def collect_lgpl_sources() -> None:
    import shapely

    if shapely.__version__ != "2.1.2" or shapely.geos_version_string != "3.13.1":
        raise RuntimeError("Update the corresponding-source versions/hashes for the new Shapely/GEOS wheel before releasing")
    sources = [
        ("geos-3.13.1.tar.bz2", "https://download.osgeo.org/geos/geos-3.13.1.tar.bz2", "df2c50503295f325e7c8d7b783aca8ba4773919cde984193850cf9e361dfd28c"),
        ("shapely-2.1.2.tar.gz", "https://codeload.github.com/shapely/shapely/tar.gz/refs/tags/2.1.2", "adbd4060635098d5109e04668edcc015117a133c39da2899cc3ce9b665b76439"),
    ]
    directory = OUTPUT.parent / "third-party-sources"
    directory.mkdir(parents=True, exist_ok=True)
    for name, url, expected in sources:
        path = directory / name
        if not path.is_file():
            with urllib.request.urlopen(url, timeout=120) as response:
                data = response.read()
            if hashlib.sha256(data).hexdigest() != expected:
                raise RuntimeError(f"Corresponding-source download hash mismatch: {name}")
            path.write_bytes(data)
        if hashlib.sha256(path.read_bytes()).hexdigest() != expected:
            raise RuntimeError(f"Corresponding-source cache hash mismatch: {name}")
    (directory / "SOURCES.txt").write_text(
        "Unmodified GEOS 3.13.1 and Shapely 2.1.2 corresponding sources.\n"
        "The Shapely archive includes ci/install_geos.cmd and .github/workflows/release.yml,\n"
        "the upstream Windows MSVC build and wheel-repair instructions used for this wheel.\n"
        "Use those scripts and GEOS source with a compatible Windows x64 compiler.\n"
        "GEOS DLLs in the installation's engine/_internal/shapely.libs can be replaced\n"
        "with ABI-compatible modified builds (retain the installed DLL filenames).\n"
        "Reverse engineering for debugging these modifications is permitted.\n"
        "The upstream archives include their original license and copyright notices.\n\n"
        + "\n".join(f"{name}\nURL: {url}\nSHA256: {digest}\n" for name, url, digest in sources),
        encoding="utf-8",
    )


def is_notice(path: Path) -> bool:
    name = path.name.lower()
    return name.startswith(("license", "licence", "copying", "copyright", "notice", "thirdpartynotice")) and path.suffix.lower() not in {".py", ".pyc", ".rs", ".html"}


def collect() -> str:
    sections = [(ROOT / "THIRD_PARTY_NOTICES.md").read_text(encoding="utf-8")]

    def add(label: str, metadata: str, paths: list[Path]) -> None:
        texts = set()
        for path in sorted(set(paths)):
            if path.is_file():
                texts.add(path.read_text(encoding="utf-8", errors="replace"))
        if not texts and not metadata:
            raise RuntimeError(f"No license information found: {label}")
        sections.append(f"\n{'=' * 72}\n{label}\n{metadata}\n" + "\n\n".join(sorted(texts)))

    project = tomllib.loads((ROOT / "engine/pyproject.toml").read_text(encoding="utf-8"))["project"]
    pending = project["dependencies"] + project["optional-dependencies"]["windows"] + project["optional-dependencies"]["ocr"] + ["pyinstaller"]
    seen = set()
    while pending:
        requirement = Requirement(pending.pop())
        if requirement.marker and not requirement.marker.evaluate({"extra": ""}):
            continue
        dist = distribution(requirement.name)
        name = dist.metadata["Name"]
        if not name:
            raise RuntimeError(f"Cannot read dependency metadata: {requirement.name}")
        if name.lower() in seen:
            continue
        seen.add(name.lower())
        pending.extend(dist.requires or [])
        paths = [Path(dist.locate_file(file)) for file in dist.files or [] if is_notice(Path(file))]
        license_name = dist.metadata.get("License-Expression") or dist.metadata.get("License") or ""
        urls = dist.metadata.get_all("Project-URL") or []
        add(f"Python: {name} {dist.version}", license_name + "\n" + "\n".join(urls), paths)

    python_license = Path(sys.base_prefix) / "LICENSE.txt"
    if not python_license.is_file():
        raise RuntimeError("Python runtime LICENSE.txt is missing")
    add(f"Python runtime {sys.version.split()[0]}", "", [python_license])

    # Include all locked crates, including build dependencies, conservatively.
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    caches = list((cargo_home / "registry/src").glob("*"))
    lock = tomllib.loads((ROOT / "src-tauri/Cargo.lock").read_text(encoding="utf-8"))
    for package in lock["package"]:
        if not package.get("source", "").startswith("registry+"):
            continue
        key = f"{package['name']}-{package['version']}"
        directory = next((cache / key for cache in caches if (cache / key).is_dir()), None)
        if directory is None:
            raise RuntimeError(f"Fetch Cargo dependencies before collecting licenses: {key}")
        manifest = tomllib.loads((directory / "Cargo.toml").read_text(encoding="utf-8"))["package"]
        paths = [file for file in directory.rglob("*") if file.is_file() and is_notice(file)]
        if manifest.get("license-file"):
            paths.append(directory / manifest["license-file"])
        add(f"Rust: {key}", manifest.get("license", "") + "\n" + manifest.get("repository", ""), paths)

    # Production frontend dependencies and their transitive dependencies only.
    pending_packages = [ROOT / "frontend/node_modules" / name for name in json.loads((ROOT / "frontend/package.json").read_text())["dependencies"]]
    seen_paths = set()
    while pending_packages:
        directory = pending_packages.pop().resolve()
        if directory in seen_paths:
            continue
        seen_paths.add(directory)
        package = json.loads((directory / "package.json").read_text(encoding="utf-8"))
        paths = [file for file in directory.rglob("*") if file.is_file() and is_notice(file)]
        add(f"JavaScript: {package['name']} {package['version']}", str(package.get("license", "")), paths)
        for name in package.get("dependencies", {}):
            candidate = next((parent / "node_modules" / name for parent in [directory, *directory.parents] if (parent / "node_modules" / name / "package.json").is_file()), None)
            if candidate is None:
                raise RuntimeError(f"Missing JavaScript dependency: {name}")
            pending_packages.append(candidate)
    return "\n".join(sections)


if __name__ == "__main__":
    collect_lgpl_sources()
    text = collect()
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(text, encoding="utf-8")
    print(f"Collected third-party notices: {OUTPUT} ({len(text):,} characters)")
