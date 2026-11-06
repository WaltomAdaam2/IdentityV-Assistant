# Bundled journal maps

The two archives merge the five map packages supplied for the 2026-11-05 update:

- `s0_hard_president.zip` and `s0_hard_speedrun.zip` → `s0_hard_11-05.zip` (29 maps).
- `s0_nightmare_maps.zip`, `s0_nightmare_speedrun.zip` and `S0噩梦新增_.zip` → `s0_nightmare_11-05.zip` (26 maps).

Each map retains base floor artwork for recognition and speedrun artwork for display. Categories and map numbers remain unchanged. `sources.json` records normalized original manifests and image SHA-256 fingerprints for safe migration; it does not contain local source paths.

Rebuild from a directory containing those five inputs:

```powershell
python scripts/merge_builtin_maps.py <input-directory>
```

The S1 collection supplied on 2026-11-06 contains `s1_hard` (14 maps) and
`s1_nightmare` (15 maps), both named **S1 磷光症**. The two bundled ZIPs preserve
their original descriptors and all 87 images byte for byte. Floor keys `1`, `2`
and `3` mean B1, B2 and B3 in S1; they must not use the S0 `3` → `b1f` alias.
There are no route images. S1 recognition uses the selected difficulty's B1
artwork; it does not infer difficulty from S0 floor-selector references.

Rebuild the separate S1 archives from the supplied combined ZIP:

```powershell
python scripts/merge_builtin_maps.py --s1 <combined-s1-archive>
```

Map artwork retains its original rights. The supplied archives do not specify an author or redistribution license; the application's GPL-3.0-only license does not relicense these images. See [third-party notices](../../THIRD_PARTY_NOTICES.md).
