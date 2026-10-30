# Updates and releases

**Language:** [中文](updates.md) | English · [README](../README_EN.md)

## Installation and migration

Source and releases now share [WaltomAdaam2/IdentityV-Assistant](https://github.com/WaltomAdaam2/IdentityV-Assistant/releases). Starting with 0.1.7, update manifests, prerelease discovery, source speed probes, and in-app project links use this repository.

| Installed version | Migration |
| --- | --- |
| 0.1.0 | No built-in updater; install 0.1.7 manually |
| 0.1.1 | Accepts an early fixed asset name only; install 0.1.7 manually |
| 0.1.2–0.1.6 | Install the one-time 0.1.7 bridge from `desktop-releases`; subsequent updates use this repository |
| 0.1.7 onward | Use this repository directly |

The bridge uses the identical signed installer. The old repository also provides a 32-character hexadecimal filename accepted by 0.1.2, and its manifest points to that alias. Copying the new manifest unchanged to the old repository would fail older clients' URL checks. After the old repository is removed, clients that have not migrated must install manually. Historical installers retain their embedded links and license notices; migration does not modify their bytes.

After publishing and verifying the bridge, the old repository can remain public and be archived. Archival makes the repository and releases read-only; retaining its name, tags, and assets preserves the migration endpoint for older clients. Unarchive it before making further changes. See [GitHub archival documentation](https://docs.github.com/en/repositories/archiving-a-github-repository/archiving-repositories).

## In-app updates

Settings offer direct GitHub access, third-party GH-Proxy, or automatic source selection. Startup checks can be disabled. Release notes appear before the user chooses to download or install. Ignoring a version's startup reminder does not prevent a manual check. Prereleases are disabled by default.

Downloads must pass repository/version URL validation, release-signature verification, signed-version validation, SHA-256, and size checks. Cancellation discards the partial download; retry starts over. Installation stops automation, releases input, cleans up the assistant's owned login proxy, and exits before starting the installer. Settings, logs, and imported maps remain in AppData.

Automatic selection reads at most 256 KiB of the same fixed public installer from each source and compares latency and speed. Each attempt has an eight-second deadline; results are cached for five minutes. If all sources fail, choose one manually. A probe does not guarantee the fastest complete download or bypass integrity checks. The selection also applies to the separate login component, whose updates have their own lifecycle.

## Building

Prepare the [development environment](development.md), then build from a clean, pushed `main`:

```powershell
pnpm install --frozen-lockfile
pnpm tauri build
```

`打包.bat` provides an interactive entry point. Packaging builds a standalone Python engine and runs its offline self-check, without requiring a game or account.

The original signing key defaults to `%LOCALAPPDATA%\IdentityV-Assistant\release-keys\updater.key`. Alternatively, point `TAURI_SIGNING_PRIVATE_KEY` to the original key file; encrypted keys also need `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Scripts neither print the key nor create a replacement. Back it up securely and never commit or upload it. Changing the key breaks verification in existing clients. Only the public key belongs in the repository.

Release assets under `src-tauri/target/release/bundle/nsis/`:

- `IdentityV-Assistant-<version>-Windows-x64-Setup.exe`
- Matching `.exe.sig`
- `latest.json`: version, notes, URL, signature, SHA-256, size, and source commit
- `SHA256SUMS.txt`: installer checksum

Use assets from the same build. Tauri CLI 2.12+ includes the version in the signature's trusted comment, which the client validates. Regenerating metadata is not a substitute for a fresh build; keep actual build and validation evidence. Packaging does not publish automatically.

## Publishing and release notes

1. Update all versions, pass checks, commit, and push `main`.
2. Rebuild from that clean commit and verify signatures and offline checks.
3. Write a UTF-8 release-notes file containing real changes.
4. With GitHub CLI authenticated, run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/publish_release.ps1 -NotesFile docs/releases/0.1.7.md
```

Use a title, summary, `## 新增与优化`, `## 修复内容`, and a `**Full Changelog**` comparison link. The script stores the complete notes in the manifest and GitHub Release, then appends a final `## SHA-256` section covering all four assets. Fewer than three factual bullets are acceptable; the first release links to its actual history. Do not pre-add checksums to NotesFile: that would duplicate the section or make the manifest checksum self-referential.

The script checks clean `main`, matching remote/build commits, version, checksums, size, and signature text. The tag identifies the actual source commit. Assets upload to a draft before it becomes the latest release. Existing tags or releases, including drafts, cause a stop rather than replacement or deletion. After an uncertain network response, inspect the existing GitHub state before resuming.

Updater signatures are not Windows Authenticode signatures; Windows may report an unknown publisher. Current first-party source and releases from 0.1.7 use GPL-3.0-only. Historical binaries and third-party materials retain their original terms; see [Third-party notices](../THIRD_PARTY_NOTICES.md).
