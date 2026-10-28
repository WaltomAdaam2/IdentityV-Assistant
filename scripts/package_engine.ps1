$ErrorActionPreference = "Stop"

if (-not (Get-Command uv -ErrorAction SilentlyContinue)) {
    throw "uv is required to package the Python engine."
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Push-Location (Join-Path $repoRoot "engine")
try {
    & uv sync --locked --extra windows --extra ocr --extra build --extra dev
    if ($LASTEXITCODE -ne 0) { throw "Engine dependencies failed to install." }
    & .\.venv\Scripts\python.exe -m PyInstaller --noconfirm engine.spec
    if ($LASTEXITCODE -ne 0) { throw "Engine packaging failed." }
    & .\.venv\Scripts\python.exe (Join-Path $PSScriptRoot "check_packaged_engine.py") "dist/identityv-engine"
    if ($LASTEXITCODE -ne 0) { throw "Packaged engine self-test failed." }
}
finally {
    Pop-Location
}
