"""Build the two bundled S0 packs from the five original user-supplied archives.

Usage: python scripts/merge_builtin_maps.py <source directory>
Images retain their original bytes. Source fingerprints protect customized imports.
"""
import hashlib
import json
import sys
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile, ZipInfo

SOURCES = ("s0_nightmare_maps.zip", "s0_nightmare_speedrun.zip", "S0噩梦新增_.zip",
           "s0_hard_president.zip", "s0_hard_speedrun.zip")


def normalized(raw):
    pack = {key: raw.get(key, raw.get(alias, "")) for key, alias in
            (("id", "packId"), ("name", "packName"), ("season_id", "seasonId"), ("season_name", "seasonName"))}
    pack.update(version=raw.get("version", ""), difficulty=raw["difficulty"], maps=[])
    for item in raw["maps"]:
        entry = {"index": item["index"], "category": item["category"], "name": item.get("name", "")}
        for layer in ("floors", "display"):
            values = dict(item.get(layer, {}))
            if "3" in values:
                values.setdefault("b1f", values.pop("3"))
            entry[layer] = values
        pack["maps"].append(entry)
    return pack


def build(source: Path, output: Path):
    pools, provenance = {}, []
    for name in SOURCES:
        with ZipFile(source / name) as archive:
            for descriptor in sorted(n for n in archive.namelist() if n.endswith("/pack.json") or n == "pack.json"):
                pack = normalized(json.loads(archive.read(descriptor).decode("utf-8-sig")))
                prefix = descriptor.removesuffix("pack.json")
                paths = {path for item in pack["maps"] for layer in ("floors", "display") for path in item[layer].values()}
                images = {path: archive.read(prefix + path) for path in paths}
                provenance.append({"pack": pack, "files": {p: hashlib.sha256(b).hexdigest() for p, b in sorted(images.items())}})
                pools[pack["id"]] = (pack, images)
    output.mkdir(parents=True, exist_ok=True)
    for difficulty, expected in (("hard", 29), ("nightmare", 26)):
        pack = dict(id=f"s0_{difficulty}_11-05", name=f"S0 {'困难' if difficulty == 'hard' else '噩梦'}地图",
                    version="2026.11.05", season_id="s0", season_name="S0 厄运之女", difficulty=difficulty, maps=[])
        merged, files = {}, {}
        for source_pack, images in pools.values():
            if source_pack["difficulty"] != difficulty:
                continue
            for item in source_pack["maps"]:
                key = (item["category"], item["index"])
                target = merged.setdefault(key, dict(index=key[1], category=key[0], name=item["name"], floors={}, display={}))
                for layer in ("floors", "display"):
                    # Base artwork supplies recognition floors; speedrun supplies display only.
                    if layer == "floors" and source_pack["id"].endswith("speedrun"):
                        continue
                    for floor, old in item[layer].items():
                        path = f"{key[0]}_{key[1]}/{layer}_{floor}{Path(old).suffix.lower()}"
                        assert floor not in target[layer], (key, layer, floor)
                        target[layer][floor] = path
                        files[path] = images[old]
        assert len(merged) == expected
        pack["maps"] = [merged[key] for key in sorted(merged)]
        floors = {"1", "2"} | ({"b1f"} if difficulty == "nightmare" else set())
        assert all(set(m[layer]) == floors for m in pack["maps"] for layer in ("floors", "display"))
        files["pack.json"] = json.dumps(pack, ensure_ascii=False, indent=2).encode("utf-8")
        with ZipFile(output / f"{pack['id']}.zip", "w") as archive:
            for path, data in sorted(files.items()):
                info = ZipInfo(path, (2026, 11, 5, 0, 0, 0))
                info.compress_type = ZIP_DEFLATED
                archive.writestr(info, data)
        print(pack["id"], len(merged), "maps", len(files) - 1, "images")
    (output / "sources.json").write_text(json.dumps(provenance, ensure_ascii=False, indent=2), encoding="utf-8")


if __name__ == "__main__":
    build(Path(sys.argv[1]), Path(__file__).resolve().parents[1] / "assets/journal")
