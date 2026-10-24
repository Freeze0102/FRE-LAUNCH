@echo off
setlocal EnableExtensions
chcp 65001 >nul
pushd "%~dp0"
title FRE LAUNCH - Local Windows Tester
echo.
echo ===========================================================
echo   FRE LAUNCH - LOCAL WINDOWS TEST / BUILD
echo   This uses your local computer, not GitHub Actions.
echo ===========================================================
echo.
if not exist "src-tauri\tauri.conf.json" (
  echo [ERROR] Cannot find src-tauri\tauri.conf.json.
  echo Extract the FULL GitHub ZIP before running this file.
  goto failed
)
where node >nul 2>nul
if errorlevel 1 (
  echo [MISSING] Node.js 22 / npm - https://nodejs.org/
  goto requirements
)
where npm >nul 2>nul
if errorlevel 1 (
  echo [MISSING] npm - reinstall Node.js 22 from https://nodejs.org/
  goto requirements
)
where rustc >nul 2>nul
if errorlevel 1 (
  echo [MISSING] Rust MSVC - https://rustup.rs/
  goto requirements
)
where cargo >nul 2>nul
if errorlevel 1 (
  echo [MISSING] Cargo - install Rust from https://rustup.rs/
  goto requirements
)
echo [OK] Node.js, npm and Rust found.
echo [IMPORTANT] Local dev and release builds can use the same AppData
echo             as your installed FRE LAUNCH.
echo             Back up Minecraft worlds and settings before testing.
echo [NOTE] You also need Microsoft C++ Build Tools with
echo        "Desktop development with C++" and Microsoft Edge WebView2.
echo        Official guide: https://v2.tauri.app/start/prerequisites/
echo.
echo Choose what to do:
echo   [1] Start current FRE LAUNCH for testing (no installer)
echo   [2] Run checks and create a Windows EXE / NSIS installer
echo   [3] Exit
choice /C 123 /N /M "Enter 1, 2 or 3: "
if errorlevel 3 goto done
if errorlevel 2 goto build
if errorlevel 1 goto dev
goto failed

:prepare
echo.
echo [1/2] Installing dependencies...
call npm install --no-audit --no-fund
if errorlevel 1 exit /b 1
echo [2/2] Generating application icons...
call npm run tauri icon src-tauri/icons/app.svg
if errorlevel 1 exit /b 1
exit /b 0

:dev
echo.
echo --- FRE LAUNCH LIVE TEST ---
call :prepare
if errorlevel 1 goto failed
echo Launching latest local source as a development app...
echo Leave this window open during testing.
call npm run tauri dev
if errorlevel 1 goto failed
goto done

:build
echo.
echo --- FRE LAUNCH RELEASE BUILD ---
call :prepare
if errorlevel 1 goto failed
echo Checking TypeScript...
call npm run check
if errorlevel 1 goto failed
echo Running Rust unit tests...
call cargo test --manifest-path src-tauri/Cargo.toml
if errorlevel 1 goto failed
echo Building Windows EXE and current-user NSIS installer...
call npm run tauri build
if errorlevel 1 goto failed
echo.
echo [SUCCESS] Local build completed.
echo Main executable:
echo %CD%\src-tauri\target\release\fre-launch.exe
echo Windows installer folder:
echo %CD%\src-tauri\target\release\bundle\nsis
echo.
echo NOTE: A successful build is NOT a malware clearance.
echo Keep Windows Security enabled; do not whitelist detections.
if exist "src-tauri\target\release\bundle\nsis" explorer "%CD%\src-tauri\target\release\bundle\nsis"
goto done

:requirements
echo.
echo Install the missing tool and restart this terminal.
echo Also install Microsoft C++ Build Tools / WebView2 using:
echo https://v2.tauri.app/start/prerequisites/
goto failed

:failed
echo.
echo [FAILED] No safe install file was produced by this attempt.
echo Copy the first ERROR lines or take a screenshot and send them to ChatGPT.
echo Do NOT disable Windows Defender to continue.
echo.

:done
echo.
pause
popd
endlocal
