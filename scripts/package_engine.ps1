$ErrorActionPreference = "Stop"

if (-not (Get-Command uv -ErrorAction SilentlyContinue)) {
    throw "uv is required to package the Python engine."
}

Push-Location engine
try {
    uv sync --extra windows --extra ocr --extra dev
    uv run pytest
    Write-Host "Python engine verified. Executable packaging is planned for a later phase."
}
finally {
    Pop-Location
}
