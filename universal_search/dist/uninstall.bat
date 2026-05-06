@echo off
echo ============================================
echo   Removing Universal Search Service
echo ============================================
echo.

where nssm >nul 2>&1
if errorlevel 1 (
    echo ERROR: NSSM not found.
    pause
    exit /b 1
)

set /p CONFIRM="Remove the Windows service? (y/N): "
if /i not "%CONFIRM%"=="y" (
    echo Cancelled.
    pause
    exit /b 0
)

nssm stop universal-search 2>nul
nssm remove universal-search confirm

echo.
echo Service removed.
pause
