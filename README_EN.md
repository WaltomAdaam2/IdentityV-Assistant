<p align="center">
  <img src="src-tauri/icons/128x128@2x.png" alt="Identity V Assistant" width="128" height="128">
</p>

<h1 align="center">Identity V Assistant · 第五人格助手</h1>

<p align="center">
  Game launcher · Visual automation · Journal maps<br>
  A Windows desktop assistant built with Tauri 2, React, Rust and Python
</p>

<p align="center">
  <a href="https://github.com/WaltomAdaam2/IdentityV-Assistant/issues">Report an issue</a> · <a href="https://github.com/WaltomAdaam2/IdentityV-Assistant/releases">Changelog</a><br>
  <a href="#install-and-start">Quick start</a> · <a href="docs/README_LOGIC_EN.md">Feature guide</a> · <a href="docs/development.md">Build guide</a>
</p>

<p align="center">
  <a href="https://github.com/WaltomAdaam2/IdentityV-Assistant/releases"><img src="https://img.shields.io/github/v/release/WaltomAdaam2/IdentityV-Assistant?color=70815a" alt="Latest release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-GPL--3.0--only-70815a" alt="License: GPL-3.0-only"></a>
  <a href="https://github.com/WaltomAdaam2/IdentityV-Assistant/stargazers"><img src="https://img.shields.io/github/stars/WaltomAdaam2/IdentityV-Assistant?style=social" alt="GitHub stars"></a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/React-18-61dafb?logo=react" alt="React 18">
  <img src="https://img.shields.io/badge/Tauri-2-24c8db?logo=tauri" alt="Tauri 2">
  <img src="https://img.shields.io/badge/Rust-2021-dea584?logo=rust" alt="Rust edition 2021">
  <img src="https://img.shields.io/badge/Python-3.12-3776ab?logo=python&logoColor=white" alt="Python 3.12">
</p>

<p align="center"><a href="README.md">中文</a> · <strong>English</strong></p>

---

<p align="center">
  <img src="docs/images/overview.png" alt="Identity V Assistant interface" width="850">
</p>

## Features

- **Game launcher:** locate the official `dwrg.exe` client and coordinate direct launch with the independent [idv-login](https://github.com/KKeygen/idv-login) component, including setup, downloads, update confirmation and proxy status.
- **Visual tasks:** lobby login, Blackjack, hunter match, hunter rank and Hide and Seek, with ordered execution, pause, resume, stop and optional match loops.
- **Journal maps:** import multiple ZIP packs, filter by season, difficulty, doors, floor and base/route view, and open map details or an in-window preview.
- **Window and input control:** capture the game window and send foreground-checked Windows input. Explicit engine startup applies window settings; shutdown attempts to restore the original window.
- **Desktop controls:** global engine shortcut, close to tray, capture diagnostics, searchable logs and optional sleep/display controls.
- **Task feedback and recovery:** preflight checks, stage timing and pause guidance, bounded local failure screenshots (on by default), settings backup/restore previews and diagnostic ZIP exports with screenshots separately selected.
- **Updates:** GitHub direct by default with three manual proxies, download speed/time estimates and Markdown notes, with confirmation and signature, version, SHA-256 and size verification.

See [Feature logic](docs/README_LOGIC_EN.md) for behavior, recovery, component responsibilities and map formats; [Development](docs/development.md) for builds and checks; and [Updates](docs/updates_EN.md) for release migration and publishing.

## Requirements

- Windows 11 x64 and Microsoft Edge WebView2 Runtime.
- An installed official Identity V client. Automation input requires the game in the foreground.
- The launcher login proxy uses the independent official idv-login component. Initial login and expired credentials still require the user.

Source builds also need Node.js 24, the pnpm version declared by the repository, Rust MSVC, Python 3.12, uv and Windows C++ build tools.

## Install and start

1. Download `IdentityV-Assistant-<version>-Windows-x64-Setup.exe` from [Releases](https://github.com/WaltomAdaam2/IdentityV-Assistant/releases) and install it.
2. Locate the game on the Game launcher page and configure the login component through its settings button. You can also start the game yourself before using visual tasks.
3. Edit and enable tasks, then select Start enabled tasks. Multiple tasks run in list order.
4. **F3** toggles the automation engine by default. Task-row buttons pause and resume; stopping a task clears the remaining queue.

**Tasks send real keyboard and mouse input; there is no default dry-run mode.** Ordinary game launch does not apply the engine window layout. Login-component readiness does not prove account authentication, and offline tests or screenshot recognition do not prove live gameplay success.

## Maps and local data

ZIP import supports multiple selections and continues after individual failures. Top notices show success counts and failure details, which also remain in logs. Map preferences save silently; the floor resets to 1F when the page is reopened.

![Journal map browser](docs/images/journal.png)

Data lives under `%LOCALAPPDATA%\IdentityV-Assistant\`, including `config.json`, dated `logs\`, manual captures and `journal\packs\`. Upgrades retain this directory. See [Feature logic](docs/README_LOGIC_EN.md) for details and map-pack format.

## Build

```powershell
pnpm install --frozen-lockfile
uv sync --project engine --extra windows --extra ocr --extra dev --extra build
pnpm tauri build
```

Official signed builds require the original updater signing key. Third-party builds can configure their own signing, but cannot replace the official update chain. See [Development](docs/development.md). Installers are written to `src-tauri/target/release/bundle/nsis/`.

## Updates and repository migration

Starting with 0.1.7, this repository hosts source, downloads and future updates. The old `desktop-releases` repository provides a one-time 0.1.7 bridge; whether an older client can use it depends on its updater capabilities. After the old repository is deleted, clients that have not migrated need a manual installation from this repository.

Updates are not downloaded or installed without confirmation. Proxy availability depends on third parties; switching sources retains all integrity checks. See [Updates and releases](docs/updates_EN.md).

## Repository layout

```text
frontend/   React interface, settings editing and status display
src-tauri/  Rust desktop shell, process lifecycle, imports and updates
engine/     Python recognition, task decisions, capture and input
assets/     Recognition assets and reference screenshots
scripts/    Builds, packaging, license collection and publishing
docs/       User guides, developer documentation and decisions
```

## Credits

- [KKeygen/idv-login](https://github.com/KKeygen/idv-login): independent login component, retaining upstream implementation and license.
- [Tauri](https://github.com/tauri-apps/tauri) and [React](https://github.com/facebook/react): desktop shell and interface.
- [OpenCV](https://github.com/opencv/opencv), [RapidOCR](https://github.com/RapidAI/RapidOCR) and [Windows Capture](https://github.com/NiiightmareXD/windows-capture): image processing, text recognition and window capture.
- [Lucide](https://github.com/lucide-icons/lucide) / [Iconify](https://github.com/iconify/icon-sets): interface icons.

Maintained by [WaltomAdaam2](https://github.com/WaltomAdaam2). See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for third-party licenses and bundled source archives, and [artwork sources](frontend/public/launcher/SOURCES.md) for game imagery. Game trademarks and imported maps retain their respective rights.

## License

Current first-party source and versions starting with 0.1.7 use **GNU GPL v3.0 only (GPL-3.0-only)**. Copyright (C) 2026 WaltomAdaam2. See [LICENSE](LICENSE). Historical installers retain their included notices; this change does not relicense third-party materials.
