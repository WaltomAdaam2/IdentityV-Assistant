$ErrorActionPreference = "Stop"

if (-not (Get-Command pnpm -ErrorAction SilentlyContinue)) {
    throw "pnpm is required. Install Node.js LTS and pnpm first."
}

pnpm install
pnpm tauri dev
