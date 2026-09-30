# Architecture

Identity V Assistant is split into three deliberately narrow layers.

## Frontend

React and TypeScript render the desktop interface, collect settings, issue lifecycle commands, and display engine events. The frontend never decides game actions.

## Tauri core

Rust owns the Python child process, NDJSON transport, configuration file access, crash detection, and application lifecycle. It forwards parsed messages through the `engine-event` Tauri event. It does not contain game state or task logic.

## Python engine

Python owns window discovery, capture, vision, OCR interfaces, state detection, task decisions, recovery, metrics, and input. Tasks use an `AutomationContext`; visual detectors return evidence and never send input directly.

```text
React UI -> Tauri commands -> Rust supervisor -> NDJSON stdin -> Python engine
React UI <- engine-event  <- Rust parser     <- NDJSON stdout <- Python engine
```

The login task uses window messages. Blackjack requires the game to be in the foreground and uses SendInput. The Rust supervisor writes complete engine events to dated AppData logs; the frontend sends its own UI events to the same archive.
