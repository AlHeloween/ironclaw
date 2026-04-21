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

REM Find the service binary (in repo root target/release/)
set SCRIPT_DIR=%~dp0
set REPO_ROOT=%SCRIPT_DIR%..
set SERVICE_BIN=%REPO_ROOT%\target\release\universal-search-service.exe

if not exist "%SERVICE_BIN%" (
    echo ERROR: universal-search-service.exe not found at %SERVICE_BIN%
    echo Please build first: cargo build --release -p universal-search-service
    pause
    exit /b 1
)

if not exist "%SCRIPT_DIR%config.jsonc" (
    echo ERROR: config.jsonc not found at %SCRIPT_DIR%config.jsonc
    echo Please copy config.jsonc to the universal_search directory.
    pause
    exit /b 1
)

REM Remove existing service
nssm stop universal-search 2>nul
nssm remove universal-search confirm 2>nul

echo Installing Universal Search Service...

nssm install universal-search "%SERVICE_BIN%" run
nssm set universal-search AppDirectory "%SCRIPT_DIR%"
nssm set universal-search Start SERVICE_AUTO_START
nssm set universal-search AppRestartDelay 5000
nssm set universal-search AppExit Default Restart
nssm set universal-search AppStdout -
nssm set universal-search AppStderr -

echo.
echo Starting Universal Search Service...
sc start universal-search

timeout /t 3 /nobreak >nul

powershell -Command "try { $null = Test-NetConnection -ComputerName localhost -Port 3005 -WarningAction SilentlyContinue; if ($?) { Write-Host 'Universal Search Service started successfully on port 3005' -ForegroundColor Green } else { Write-Host 'Service may still be starting...' -ForegroundColor Yellow } } catch {}"

echo.
echo To stop:   nssm stop universal-search
echo To start:  nssm start universal-search
echo To remove: nssm remove universal-search confirm
echo Logs:     sc qc universal-search
pause
