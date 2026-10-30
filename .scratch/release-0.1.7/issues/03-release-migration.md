# Consolidate release delivery into the source repository

Status: resolved
Type: task

## Decisions

- Destination: WaltomAdaam2/IdentityV-Assistant, already public, already contains v0.1.0 and v0.1.1. Source distribution repository contains v0.1.2 through v0.1.6; all are stable published releases with four assets each.
- Preserve original installer/signature bytes and original metadata in a local archive. New-repository manifests may change only release-facing metadata/URLs and need their own hashes. Historical binaries retain their original license terms and embedded updater behavior.
- Use actual source build commits for migrated source tags, derived from original manifests/history; do not point every tag at current main or the old distribution-only commit.
- Publish 0.1.7 to the source repository with full notes, then identical installer/signature to the old repository with an old-URL bridge manifest. Keep original signing key. Future new-client paths use only the source repository; old repository deletion remains the user's action.
- Format release bodies as the supplied template with real changes and a final SHA-256 section after Full Changelog. Permit fewer than three bullets when evidence does not support more.

## Progress

- Saved full original release metadata to D:/.Others/Release/work/IdentityV-Assistant-Releases/migration-0.1.7/desktop-releases-original.json.
- Original v0.1.6 installer already archived; its SHA-256 matches current GitHub asset digest b678265f91e77264222b515230fdc56ec71784419ea02e8a9518ef1a3d7f2bc0. Reused it after stopping only the redundant task-owned gh download process. Other archive downloads pending.
- Runtime paths requiring migration: strict installer allowlist and preview metadata in updates.rs, updater config/homepage, automatic-source probe, native project link, frontend update/about links, manifest generation and publish target.
- Existing publish script drops complete notes from GitHub body and targets the distribution main commit. Both must be corrected. A generated manifest alone is not proof of a fresh build; retain actual clean-build provenance for 0.1.7.

- All 20 historical assets (0.1.2–0.1.6) now archived and verified against original GitHub size/digests. Preserved source-repository 0.1.0/0.1.1 metadata before reformatting.
- 0.1.2 accepts only 32 lowercase hex filenames; 0.1.3–0.1.6 accept that alias and readable names. The bridge will use a hex alias of the identical 0.1.7 installer. 0.1.0 lacks an updater; 0.1.1 requires its early fixed name and is documented as manual migration.
- Runtime/about/probe/preview/config/build/publish links switched to the source repository; Rust regression rejects legacy-repository assets in new clients. Complete notes and all four asset hashes replace the former hash-only body.
- Updater targeted tests passed (7); complete Rust suite passed (60, with network/native tests explicitly ignored). Historical migration is in progress; source tag and uploaded digest verification follows each draft upload.

## Completion

- All seven historical releases in IdentityV-Assistant use the requested template and final asset SHA-256 sections. Five migrated releases use manifest-derived source commits; original installer/signature bytes and old metadata remain archived.
- 0.1.7 is published as Latest in both repositories. New source tag/build commit is f59581c2e17d1dba2fa680d02abfed7a186bcd33. The legacy manifest uses its own old-repository hex alias; all remote asset digests and update trust checks passed. See [final validation](05-final-validation.md) for exact evidence and network recovery details.
- Public archival of desktop-releases is supported after this verified bridge; the user performs that action.
