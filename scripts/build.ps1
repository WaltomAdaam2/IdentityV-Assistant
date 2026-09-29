$ErrorActionPreference = "Stop"

pnpm install --frozen-lockfile
pnpm typecheck
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tauri build
