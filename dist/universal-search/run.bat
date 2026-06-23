@echo off
echo ============================================
echo   Universal Search Service
echo ============================================
echo.
echo Starting on port 3005...
echo Config: config.jsonc
echo.
set "DIR=%~dp0"
set "DIR=%DIR:~0,-1%"
if not exist "%DIR%\logs" mkdir "%DIR%\logs"
"%~dp0universal-search-service.exe" --log-level info --log-file "%~dp0logs\universal-search.log" run