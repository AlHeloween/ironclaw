@echo off
echo ============================================
echo   Installing Universal Search Service
echo ============================================
echo.

where nssm >nul 2>&1
if errorlevel 1 (
    echo ERROR: NSSM not found. Install with: winget install nssm.nssm
    pause
    exit /b 1
)

REM Get the absolute path of this script's directory
set SCRIPT_DIR=%~dp0
set SERVICE_BIN=%SCRIPT_DIR%universal-search-service.exe

if not exist "%SERVICE_BIN%" (
    echo ERROR: universal-search-service.exe not found at %SERVICE_BIN%
    echo Please build first:
    echo   pwsh -File build.ps1
    pause
    exit /b 1
)

if not exist "%SCRIPT_DIR%config.jsonc" (
    echo ERROR: config.jsonc not found at %SCRIPT_DIR%config.jsonc
    echo Please ensure config.jsonc is in the same directory as this script.
    pause
    exit /b 1
)

REM Remove existing service
nssm stop universal-search 2>nul
nssm remove universal-search confirm 2>nul

echo Installing Universal Search Service...
echo   Binary: %SERVICE_BIN%
echo   Config: %SCRIPT_DIR%config.jsonc
echo   Port: 3005
echo.

nssm install universal-search "%SERVICE_BIN%" run
nssm set universal-search AppDirectory "%SCRIPT_DIR%"
nssm set universal-search Start SERVICE_AUTO_START
nssm set universal-search AppRestartDelay 5000
nssm set universal-search AppExit Default Restart
nssm set universal-search AppStdout -
nssm set universal-search AppStderr -
nssm set universal-search AppPriority BELOW_NORMAL_PRIORITY_CLASS

echo.
echo Starting Universal Search Service...
sc start universal-search

timeout /t 3 /nobreak >nul

powershell -Command "try { $null = Test-NetConnection -ComputerName localhost -Port 3005 -WarningAction SilentlyContinue; if ($?) { Write-Host 'Universal Search Service started successfully on port 3005' -ForegroundColor Green } else { Write-Host 'Service may still be starting...' -ForegroundColor Yellow } } catch {}"

echo.
echo Service installed and running.
echo.
echo To stop:   nssm stop universal-search
echo To start:  nssm start universal-search
echo To remove: nssm remove universal-search confirm
echo Status:   nssm status universal-search
pause
