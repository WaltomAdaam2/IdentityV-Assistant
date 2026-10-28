@echo off
setlocal
pushd "%~dp0"
where pnpm >nul 2>nul
if errorlevel 1 goto missing
where cargo >nul 2>nul
if errorlevel 1 goto missing
where uv >nul 2>nul
if errorlevel 1 goto missing
echo Installing project dependencies...
call pnpm install --frozen-lockfile
if errorlevel 1 goto failed
echo Building engine, UI and Windows installer...
call pnpm tauri build
if errorlevel 1 goto failed
echo.
echo Build succeeded. Installer: src-tauri\target\release\bundle\nsis
start "" explorer.exe "%~dp0src-tauri\target\release\bundle\nsis"
popd
pause
exit /b 0
:missing
echo Please install Node.js/pnpm, Rust with MSVC Build Tools, and uv first.
:failed
echo.
echo Build failed. See the error above.
popd
pause
exit /b 1
