@echo off
setlocal EnableExtensions
chcp 65001 >nul
pushd "%~dp0"
title FRE LAUNCH - Modrinth-based Windows Launcher
echo.
echo ===============================================================
echo   FRE LAUNCH - ORIGINAL MODRINTH ENGINE / UI
echo   Source: official Modrinth GPL-3.0 fork, pinned and rebranded.
echo   All builds run locally. NO GitHub Actions minutes.
echo ===============================================================
echo.
echo The former React-based FRE LAUNCH can crash with STATUS_STACK_OVERFLOW.
echo This menu launches the independent Modrinth-based implementation.
echo Your existing Minecraft worlds are NOT modified by the source switch.
echo.
echo [1] Run Modrinth-based FRE LAUNCH live (testing)
echo [2] Build Modrinth-based FRE LAUNCH NSIS installer
echo [3] Open OLD React tester (legacy, may still crash)
echo [4] Exit
choice /C 1234 /N /M "Enter 1, 2, 3 or 4: "
if errorlevel 4 goto done
if errorlevel 3 goto legacy
if errorlevel 2 goto build
if errorlevel 1 goto dev
goto failure

:dev
echo.
echo --- REAL MODRINTH SOURCE / DEV ---
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0FRE-MODRINTH-LOCAL.ps1" -RunDev
if errorlevel 1 goto failure
goto done

:build
echo.
echo --- REAL MODRINTH SOURCE / WINDOWS INSTALLER ---
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0FRE-MODRINTH-LOCAL.ps1" -BuildWindows
if errorlevel 1 goto failure
if exist "%~dp0FRE-Modrinth-Source\target\release\bundle\nsis" (
 explorer "%~dp0FRE-Modrinth-Source\target\release\bundle\nsis"
)
goto done

:legacy
call "%~dp0FRE-LAUNCH-LEGACY.cmd"
goto done

:failure
echo.
echo [FAILED] Read the FIRST error above. This is not an installed game crash.
echo Do not disable Windows Security or delete your worlds.
:done
echo.
pause
popd
endlocal
