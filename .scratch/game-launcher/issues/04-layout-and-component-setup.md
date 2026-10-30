# Launcher layout and automatic component setup

Status: resolved
Type: task

## Scope and decisions

- Limit announcement panel to half the launcher width, truncate long titles, retain full title in hover text.
- Use the current upstream GUI's high-resolution background and colored logo, without a monochrome filter.
- Shorter launch/settings buttons: lift together on hover, depress only the pressed segment; remove white focus outline, retain color-based keyboard focus.
- Scan official `install_root.flag`, D:/ProgramData/IDV-Login, C:/ProgramData/IDV-Login, Public Documents and the assistant's component directory. Select a component folder rather than its interpreter.
- If absent, download the pinned official release using the user's existing download-source setting and extract into the assistant program directory. Verify size/SHA-256 before use; do not run the upstream installer scripts.
- User has authorized recommended choices, checkpoint commits/pushes on main, and existing launcher test seams. Extend these to automatic setup success/failure and visible layout behavior.
- Preserve pre-existing package.json and pnpm-lock.yaml edits.

## Evidence

- Rechecked official v6.3.2-stable release, startup BAT, Inno script and upstream launcher HTML on 2026-10-30.
- Official BAT invokes python-embed/python.exe with src/main.pyc. Official default install directories differ from the previous Documents-only discovery.
- Existing install_root.flag on this machine points to D:/ProgramData/IDV-Login; expected runtime files and BAT exist there.
- Initial inspection hit the legacy fallback HTML (main_image, 480x270). Actual get_login_page prioritizes login_base64_page_fever, whose UI uses background_image (1920x1080) and game_library_logo (colored). The low-resolution candidate was rejected before commit.

## Verification

- Layout test reproduced the defect (470 px panel on an 816 px launcher; long row extended to x=3287; button height 58 px). It now passes: panel <= half width, title clipped, row inside panel, button <=48 px, unified hover movement, independent press and zero-width outline.
- TypeScript passed. The 480x270 legacy image was rejected; final background is 1920x1080 and logo is the current colored game_library_logo without white filtering.
- Folder/BAT selection test was red before runtime_from_selection and is now green. Download/setup implementation remains in progress.

## Follow-up implementation

- Official download and Auto configure are separate adjacent buttons. Auto configure scans/saves; Official download reuses a compatible runtime or downloads/extracts/configures the pinned package, including when the discovered version is unsupported.
- Removed the two redundant Settings rows. Added startup/setup cancellation and phase text, real health polling, console-free managed bootstrap and important application-log forwarding.
- Independent review found health-poll lock contention and unsupported-discovered-version blocking download. Both corrected; added lock-contention regression test.
- Seven browser interaction tests passed; TypeScript and production Vite build passed (Vite needed normal host access because sandbox parent-directory enumeration was denied).
- Rust full suite at this checkpoint: 48 passed / 6 ignored; subsequent launcher suite passed all 12 tests, including the new lock-contention regression. Lifecycle fixture confirms no console window, graceful cancellation/exit, health loss and external process preservation.
- Bootstrap tests: four passed using the actual extracted official embedded Python, including real module loading with the main login function stubbed. No real account login is claimed.
- Package-only cargo clean completed; a fresh Windows executable build passed and the native assistant was launched. The native overview was visually checked; elevated-window input was unavailable to the computer-use helper, so launcher interactions remain browser-harness evidence. Separate native exit-warning investigation is tracked in issue 05.
