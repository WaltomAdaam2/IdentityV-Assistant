# Architecture

Identity V Assistant is split into three deliberately narrow layers.

## Frontend

React and TypeScript render the desktop interface, collect settings, issue lifecycle commands, and display engine events. The frontend never decides game actions.

## Tauri core

Rust owns the Python child process, NDJSON transport, configuration file access, crash detection, and application lifecycle. It also validates map archives, manages assistant updates, and owns the separate login-component process and its download/install lifecycle. It forwards parsed messages through the `engine-event` Tauri event. It does not contain game state or task logic.

## Python engine

Python owns window discovery, capture, vision, OCR interfaces, state detection, task decisions, recovery, metrics, and input. Tasks use an `AutomationContext`; visual detectors return evidence and never send input directly.

Journal recognition uses the same Python entry point in `--recognize-journal` mode without constructing the automation engine. Rust validates imported template paths, owns that one-shot child, forwards progress and terminates only that child on cancellation or timeout. This avoids adopting or stopping a paused automation engine just to capture a map. Task-start/resume acknowledgements and queue transitions remain mutually exclusive with recognition. Python performs WGC-only capture and geometry matching; React receives candidate identities and an in-memory preview, not file access authority or game-action decisions. The foreground-scoped shortcut only observes and forwards the selected key; it never invokes native window activation.

```text
React UI -> Tauri commands -> Rust supervisor -> NDJSON stdin -> Python engine
React UI <- engine-event  <- Rust parser     <- NDJSON stdout <- Python engine
```

Automated login-screen interaction and gameplay require the game to be in the foreground and use SendInput with physical keyboard scan codes. Input failures propagate to task supervision; held inputs remain tracked until their release succeeds. The Rust supervisor writes complete engine events to dated AppData logs; the frontend sends its own UI events to the same archive.

The independent login component handles login traffic, certificates, and URI registration; it is distinct from the screenshot/input engine. Its pinned compatibility contract is documented in [ADR 0001](adr/0001-game-launcher-and-login-component.md). React uses one application-level notice owner for asynchronous results so navigation does not discard feedback. Release tags and update assets live beside their source, with immutable historical installers and manifest metadata appropriate to each update endpoint.
