# Bundled S0 maps

The two archives merge the five map packages supplied for the 2026-11-05 update:

- `s0_hard_president.zip` and `s0_hard_speedrun.zip` → `s0_hard_11-05.zip` (29 maps).
- `s0_nightmare_maps.zip`, `s0_nightmare_speedrun.zip` and `S0噩梦新增_.zip` → `s0_nightmare_11-05.zip` (26 maps).

Each map retains base floor artwork for recognition and speedrun artwork for display. Categories and map numbers remain unchanged. `sources.json` records normalized original manifests and image SHA-256 fingerprints for safe migration; it does not contain local source paths.

Rebuild from a directory containing those five inputs:

```powershell
python scripts/merge_builtin_maps.py <input-directory>
```

Map artwork retains its original rights. The supplied archives do not specify an author or redistribution license; the application's GPL-3.0-only license does not relicense these images. See [third-party notices](../../THIRD_PARTY_NOTICES.md).
