@echo off
echo ============================================
echo   Installing Universal Search Service
echo ============================================
echo.

where nssm >nul 2>&1
if errorlevel 1 (
    echo ERROR: NSSM not found. Install with: winget install nssm.nssm
    exit /b 1
)

set SCRIPT_DIR=%~dp0
REM Remove trailing backslash to avoid cmd quote escaping
if "%SCRIPT_DIR:~-1%"=="\" set SCRIPT_DIR=%SCRIPT_DIR:~0,-1%
set SERVICE_BIN=%SCRIPT_DIR%\universal-search-service.exe
set SERVICE_NAME=universal-search
set LOG_FILE=%SCRIPT_DIR%logs\universal-search.log
set LOG_DIR=%SCRIPT_DIR%logs

REM Create logs directory
if not exist "%LOG_DIR%" mkdir "%LOG_DIR%"

REM Remove existing
nssm stop %SERVICE_NAME% 2>nul
nssm remove %SERVICE_NAME% confirm 2>nul

echo Installing service...
nssm install %SERVICE_NAME% "%SERVICE_BIN%" --log-level info --log-file "%LOG_FILE%" run
nssm set %SERVICE_NAME% AppDirectory "%SCRIPT_DIR%"
nssm set %SERVICE_NAME% Start SERVICE_AUTO_START
nssm set %SERVICE_NAME% AppRestartDelay 5000
nssm set %SERVICE_NAME% AppExit Default Restart

echo.
echo Starting service...
nssm start %SERVICE_NAME%

timeout /t 3 /nobreak >nul
echo.
echo Service installed.
echo   Stop:    nssm stop %SERVICE_NAME%
echo   Remove:  nssm remove %SERVICE_NAME% confirm
echo   Status:  nssm status %SERVICE_NAME%
exit /b 0
