# Final validation and signed publication

Status: claimed
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

## Remaining

- Commit/review the narrow final corrections, rebuild from clean source and await final CI.
- Finish historical migration, publish 0.1.7 and its old-repository bridge, verify public hashes/signatures/manifests and record source tags.
- Native installation/upgrade, file picker, authentication and gameplay are explicitly excluded by the user.
