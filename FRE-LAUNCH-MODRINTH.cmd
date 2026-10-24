@echo off
setlocal
cd /d "%~dp0"
echo FRE LAUNCH - Modrinth-based Windows build
echo This uses the pinned Modrinth source and DOES NOT consume GitHub Actions minutes.
echo Requires Git, Python 3, Node.js 24, pnpm, Rust, MSVC Build Tools.
echo The old FRE LAUNCH installation and Minecraft worlds will not be overwritten.
echo.
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0FRE-MODRINTH-LOCAL.ps1" -BuildWindows
if errorlevel 1 (
    echo.
    echo Build stopped with an error. Read the message above for the missing dependency.
    pause
    exit /b 1
)
echo.
echo Source is under FRE-Modrinth-Source. Check its target\release\bundle\nsis folder for the installer.
pause
