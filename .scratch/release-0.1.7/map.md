# Source and verification map

The project has one application domain across React, Rust, and Python. Exploration covered the source tree, existing feature specifications/issues, domain docs, release history and actual GitHub failure logs. This is evidence-based understanding, not a promise that unexecuted live behavior is defect-free.

| Area | Ownership / relevant evidence |
| --- | --- |
| UI, notices, logs | App, Journal, launcher/update panels; rendered React tests with mocked IPC, logs tests and screenshot inspection |
| Rust shell | configuration migration, engine supervision, window/tray lifecycle, launcher/component ownership, journal archive validation and updater validation; 60 library tests pass |
| Python engine | capture, foreground input, visual/OCR evidence, task routing, pause/recovery and timers; 254 fixture/offline tests pass |
| Game launcher | existing ADR 0001 and game-launcher specifications; pinned component contract, process/port ownership and account-data isolation |
| Window restoration | existing restoration issues and source; diagnostic startup is distinct from explicit layout-enabled automation |
| Map import | fixed-page paragraphs were historical behavior; silent preference-save optimization did not remove an import popup; new global notices cover async completion and repeated results |
| CI | original failure was rustfmt; 78cee99 fixed it and GitHub run 41034910716 succeeded |
| Releases | archived five source-distribution releases with all original digests; tags derived from clean source commits in manifests; one bridge for old URL/filename validation |
| Documentation | separate Chinese/English README, feature logic, development and release docs; corrected dry-run, capture, recovery and IPC claims; local links checked |
| Licensing | official GPL v3 text, GPL-3.0-only metadata, preserved historical binary terms and third-party attribution/source obligations |

Independent final defect review found no remaining actionable regressions after the sticky-notice fix. All 19 browser cases were verified (18 in the full run, one after a render-wait correction). The clean signed build and both published 0.1.7 releases are verified; see issues/05-final-validation.md. Native picker, installation/upgrade, account authentication and live gameplay remain deliberately untested under the user's latest decision.

The architecture report recommends reusing the current notice owner and keeping release identity consistent. No general event bus, release framework or unrelated rewrite was introduced.
