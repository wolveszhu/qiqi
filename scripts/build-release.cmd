@echo off
REM One-click release build for qiqi (double-click entry).
REM Keep this file ASCII-only: cmd.exe decodes batches with the OEM code page,
REM so any Chinese text here would garble. All messages live in the .ps1.
setlocal
chcp 65001 >nul
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0build-release.ps1" %*
set "EXITCODE=%ERRORLEVEL%"
echo.
echo Exit code: %EXITCODE%
echo.
REM Pause only on double-click (no args) so the window stays readable.
REM With args (e.g. -DryRun) the caller is a shell and must not hang.
if "%~1"=="" pause
exit /b %EXITCODE%
