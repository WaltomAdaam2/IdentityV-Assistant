# ADR 0002: Publish releases beside their source

Status: accepted
Date: 2026-10-30

## Context

Versions 0.1.2–0.1.6 used the separate `desktop-releases` repository. Its removal would break embedded update URLs, and its tags identify distribution metadata rather than application source. Existing clients strictly validate the repository and installer filename.

## Decision

From 0.1.7, source, stable/prerelease metadata, installers, the automatic-source probe, and project links use `WaltomAdaam2/IdentityV-Assistant`. Release tags identify actual clean build commits. Keep the existing signing key and signed-version, hash, size, and URL checks.

Migrate historical installers and signatures without changing their bytes. Preserve original metadata in the migration archive; new manifests use destination URLs and complete release notes. Historical licenses remain inside historical binaries. Current first-party code uses GPL-3.0-only.

Publish one 0.1.7 bridge in the old repository with the identical signed installer and an old-repository manifest. A hexadecimal filename alias supports the stricter 0.1.2 client. Versions 0.1.0 and 0.1.1 need manual installation. Removing the old repository remains a separate user action; clients not migrated by then must install manually.

## Consequences

The existing updater and release scripts retain ownership; no new release framework or configuration layer is needed. Release bodies contain complete factual notes, a source-history comparison, then asset SHA-256 values. Upload drafts completely before publishing. Never replace an existing tag or release automatically. Builds and offline checks do not establish successful native installation or live gameplay.

## 0.1.8 superseding workflow

Version 0.1.8 removes automatic source probing entirely; legacy auto settings resolve to direct GitHub without network selection. Manual proxy choices remain.

Signed assets are built from a clean, pushed `SNAPSHOT-v<version>` commit before its PR merges. A Draft Release identifies that verified build commit and is published manually after review. This supersedes the earlier automatic draft-to-public step, without changing signing or integrity validation. See [the maintained workflow](../updates_EN.md).
