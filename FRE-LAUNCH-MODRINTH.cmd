@echo off
setlocal
cd /d "%~dp0"
rem Both entry points show the Modrinth-based run/build menu.
rem No GitHub Actions, no automatic installer execution.
call "%~dp0FRE-LAUNCH-LOCAL.cmd"
endlocal
