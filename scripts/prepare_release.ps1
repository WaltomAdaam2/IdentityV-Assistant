$ErrorActionPreference = "Stop"
& (Join-Path $PSScriptRoot "package_engine.ps1")
& pnpm --dir (Join-Path $PSScriptRoot "../frontend") build
if ($LASTEXITCODE -ne 0) { throw "Frontend build failed." }
