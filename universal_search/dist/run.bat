@echo off
echo ============================================
echo   Universal Search Service
echo ============================================
echo.
echo Starting Universal Search Service...
echo   Config: %~dp0config.jsonc
echo   Port: 3005
echo.
"%~dp0universal-search-service.exe" run
