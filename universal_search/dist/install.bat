@echo off
echo ============================================
echo   Installing Universal Search Service
echo ============================================
echo.

where nssm >nul 2>&1
if errorlevel 1 (
    echo ERROR: NSSM not found. Install with:
    echo   winget install nssm.nssm
    echo.
    pause
    exit /b 1
)

set SCRIPT_DIR=%~dp0
set SERVICE_BIN=%SCRIPT_DIR%universal-search-service.exe
set SERVICE_NAME=universal-search

REM Remove existing service
nssm stop %SERVICE_NAME% 2>nul
nssm remove %SERVICE_NAME% confirm 2>nul

echo Installing service...
nssm install %SERVICE_NAME% "%SERVICE_BIN%" run
nssm set %SERVICE_NAME% AppDirectory "%SCRIPT_DIR%"
nssm set %SERVICE_NAME% Start SERVICE_AUTO_START
nssm set %SERVICE_NAME% AppRestartDelay 5000
nssm set %SERVICE_NAME% AppExit Default Restart
nssm set %SERVICE_NAME% AppPriority BELOW_NORMAL_PRIORITY_CLASS

echo.
echo Starting service...
nssm start %SERVICE_NAME%

timeout /t 3 /nobreak >nul

powershell -Command "try { $null = Test-NetConnection -ComputerName localhost -Port 3005 -WarningAction SilentlyContinue; if ($?) { Write-Host 'Service started on port 3005' -ForegroundColor Green } else { Write-Host 'Service starting...' -ForegroundColor Yellow } } catch {}"

echo.
echo Service installed.
echo   To stop:   nssm stop %SERVICE_NAME%
echo   To remove: nssm remove %SERVICE_NAME% confirm
echo   Status:   nssm status %SERVICE_NAME%
pause
