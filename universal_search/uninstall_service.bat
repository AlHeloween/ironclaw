@echo off
echo ============================================
echo   Removing Universal Search Services
echo ============================================
echo.
echo This will remove:
echo   - Universal Search Service (universal-search)
echo   - Firecrawl Service (Firecrawl)
echo.
set /p CONFIRM="Continue? (y/N): "
if /i not "%CONFIRM%"=="y" (
    echo Cancelled.
    pause
    exit /b 0
)

where nssm >nul 2>&1
if errorlevel 1 (
    echo ERROR: NSSM not found.
    pause
    exit /b 1
)

echo.
echo Stopping services...
nssm stop universal-search 2>nul
nssm stop Firecrawl 2>nul

echo Removing services...
nssm remove universal-search confirm
nssm remove Firecrawl confirm

echo.
echo Services removed successfully.
pause
