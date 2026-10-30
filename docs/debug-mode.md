# Debug mode

Tasks send real mouse and keyboard input through Windows SendInput after checking the foreground window. There is no default dry-run mode. Offline tests replace input and capture with fixtures; those tests do not prove live game behavior.

The Debug page provides engine diagnostics, window detection, one-shot capture, recognition evidence, template checks, and the latest frame metadata. Frames are saved only for an explicit capture request, under `%LOCALAPPDATA%\IdentityV-Assistant\debug\captures`. Failures and timeouts do not automatically save screenshots. The old `save_debug_screenshots` configuration field remains readable for compatibility but is not an active feature.

Developer input testing is separate from task execution and starts disabled every time the engine launches. The Debug page must send an explicit `set_enabled` action before a click or key test. Enable it only with a harmless test application in the foreground. Stopping, pausing, errors, and shutdown release every tracked key and mouse button.
