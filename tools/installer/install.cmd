@echo off
rem Installs or updates Cute Display's installer, then runs `cute-display setup`, or the
rem command it is given, from the Command Prompt:
rem
rem   curl -LsSfo "%TEMP%\cute-display.cmd" https://github.com/mjfcolas/cute-display/releases/latest/download/install.cmd && "%TEMP%\cute-display.cmd"
rem
rem `just release` fills in the version.
setlocal

set "WHEEL=https://github.com/mjfcolas/cute-display/releases/download/v@VERSION@/cute_display_installer-@VERSION@-py3-none-any.whl"

where uv >nul 2>nul || (
    echo Installing uv, which runs the installer...
    powershell -NoProfile -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex" || exit /b 1
    if defined UV_INSTALL_DIR (set "PATH=%UV_INSTALL_DIR%;%PATH%") else (set "PATH=%USERPROFILE%\.local\bin;%PATH%")
)
where uv >nul 2>nul || (
    echo uv could not be installed: see https://docs.astral.sh/uv/getting-started/installation/ 1>&2
    exit /b 1
)

uv tool install --force --quiet "%WHEEL%" || exit /b 1
uv tool update-shell >nul 2>nul
for /f "delims=" %%d in ('uv tool dir --bin') do set "INSTALLER=%%d\cute-display.exe"
if not defined INSTALLER exit /b 1

if "%~1"=="" (
    "%INSTALLER%" setup
) else (
    "%INSTALLER%" %*
)
