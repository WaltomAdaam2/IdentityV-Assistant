# Debug mode

Dry run is enabled by default. Capture and recognition continue, while the input controller reports planned actions and sends no mouse or keyboard input.

The Debug page provides engine diagnostics, window detection, one-shot capture, recognition evidence, template checks, and the latest frame metadata. Frames are saved only for explicit capture, recognition failures, timeouts, fatal errors, or an enabled debug option.

Developer input testing is separate from task execution and starts disabled every time the engine launches. The Debug page must send an explicit `set_enabled` action before a click or key test. Enable it only with a harmless test application in the foreground. Stopping, pausing, errors, and shutdown release every tracked key and mouse button.
