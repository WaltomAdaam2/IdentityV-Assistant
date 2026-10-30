# Final validation and signed publication

Status: resolved
Type: task

## Evidence

- Checkpoint cb9397fcad77ec84fbd66fac8f17cdb5525cba88 passed GitHub CI run 41066225895. A transient GitHub DNS route failed twice; a per-command Git curl resolution using independently resolved 140.82.121.3 succeeded, without changing remotes or persistent networking settings.
- Complete rendered-browser suite: 18/19 passed. The remaining developer-mode test read the Settings DOM before it rendered; add a predicate wait, then its isolated rerun passed. All map-import/fade/focus/scroll cases passed in the full run. Native IPC was mocked throughout.
- Clean release checkout D:/.Others/Release/work/IdentityV-Assistant-v0.1.6 was fast-forwarded to cb9397f and uses the committed pnpm 11.19.0 lockfile. Root user pnpm 12.16.1 and lockfile edits remain excluded.
- Fresh frozen-engine build passed 43-template self-test, Unicode installation-path simulation, offline models, no Python PATH and hidden protocol startup/shutdown tests. This did not install or start the native assistant or a game.
- Optimized Rust/NSIS build produced an 83.13 MiB signed 0.1.7 installer. Final metadata generation in the nested Windows PowerShell process failed because Get-FileHash was unavailable; invoking that step independently succeeded. Explicitly reload the standard Utility module before metadata generation and repeat the complete build before publication.
- The same investigation demonstrated that `tauri build --help` used to regenerate metadata. Exclude help/version flags from build-only side effects. Windows PowerShell metadata fixture and help-only wrapper checks pass.

- Follow-up independent review caught PowerShell case-insensitive matching confusing verbose `-v` with version `-V`; changed to `-cin`. Seven actual-condition cases passed: build/-v/--verbose true; -V/--version/-h/--help false.
- All seven historical releases now have matching template bodies. All five migrated releases retain original installer/signature bytes and verified source tags; all 20 uploaded asset digests match the migration plan. Latest temporarily remains v0.1.6 until 0.1.7 is ready.

- Second complete build at 62f2f58 reproduced Get-FileHash unavailability despite explicit Utility import. Replace this dependency with a shared 11-line .NET SHA256 stream helper used by metadata and publishing (no new packages). Both PowerShell 5/7 matched the original cmdlet for the actual installer; exclusive reopening confirmed handle cleanup. Independent review: no findings. A third clean build will verify the complete path.
- New-repository live automatic-source probe selected proxy successfully; prerelease metadata check passed. Extracted original URL validators from all five historical source commits and ran them offline against the bridge alias; all passed. Their updater public keys and signed-version requirements match the current key.

## Published result

- Final clean source/build commit: f59581c2e17d1dba2fa680d02abfed7a186bcd33. Complete `pnpm tauri build` exited 0, including frozen-engine self-check and update metadata generation. Log: D:/.Others/Release/work/IdentityV-Assistant-Releases/build-0.1.7-verified.log.
- Final build source CI: https://github.com/WaltomAdaam2/IdentityV-Assistant/actions/runs/41069343183 — success.
- Source release: https://github.com/WaltomAdaam2/IdentityV-Assistant/releases/tag/v0.1.7 (ID 429124130); tag points to the actual clean build commit. Four uploaded asset hashes, body and Latest state verified.
- Bridge release: https://github.com/WaltomAdaam2/desktop-releases/releases/tag/v0.1.7 (ID 429128630); six assets include the canonical installer/signature plus a compatible hexadecimal alias/signature. The old repository tag points to its distribution main; its manifest records the actual application source commit.
- Installer: IdentityV-Assistant-0.1.7-Windows-x64-Setup.exe, 87,163,816 bytes. SHA-256: f5c831be0f07e4211169339c7346947f4f6928c04e25df7122a748b230b7c069. Original-key signature and trusted version verified before publication using the updater's existing Minisign library.
- Git remote reads repeatedly timed out, so a one-time copy of the reviewed publisher used equivalent GitHub API reads for remote commit/tag checks. All other provenance and artifact gates remained intact. Two GitHub CLI create attempts returned HTTP 500; fresh API reads confirmed no draft/tag existed. Direct REST draft creation succeeded, followed by CLI asset uploads, digest verification and publication. No tags or uploaded assets were overwritten or deleted; existing historical release descriptions were reformatted as authorized.
- The actual updater SDK downloaded public 0.1.7 through the supported proxy and verified repository URL, signed version, signature, SHA-256 and size (255.44 seconds). The ignored network test now asserts the manifest version equals the package version so a stale cached historical release cannot give a false pass.
- Public old Latest manifest downloaded and matched the prepared bridge byte for byte. Its alias URL returned 262,144 bytes identical to the start of the verified new installer; GitHub's full-asset digest equals the canonical installer. This is a bounded old-URL download plus full remote-digest verification, not a second complete installer download.
- Original installers/signatures for 0.1.2–0.1.6 independently passed cryptographic verification. All historical release bodies and migrated source tags are verified, and old original bytes remain archived under migration-0.1.7/original-v*.
- Published assets and metadata are archived under D:/.Others/Release/work/IdentityV-Assistant-Releases/v0.1.7. The original pnpm 12.16.1/lockfile edits remain unstaged in the user's primary checkout.
- The user asked about archiving the old repository. Public archival after verification preserves the read-only migration endpoint; this is now documented in both languages. No repository administration was performed.

## Verification limits

Native installation/upgrade, the actual file picker, account authentication and gameplay were explicitly excluded by the user. Browser interaction used the real React app with mocked native IPC; engine, process and packaged-engine tests were offline fixtures. No claim of live-game success is made.
