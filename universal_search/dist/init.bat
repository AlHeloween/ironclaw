@echo off
setlocal enabledelayedexpansion

echo ============================================
echo  Universal Search Service - Initialization
echo ============================================
echo.

set SCRIPT_DIR=%~dp0
set LOG_DIR=%SCRIPT_DIR%logs
set SERVICE_NAME=universal-search

if not exist "%LOG_DIR%" mkdir "%LOG_DIR%"

echo Step 1/4: Checking prerequisites...
set MISSING=
where git 2>&1 | find "could not find" >nul && set MISSING=!MISSING! git
where node 2>&1 | find "could not find" >nul && set MISSING=!MISSING! node
where pnpm 2>&1 | find "could not find" >nul && (
    echo   Installing pnpm...
    npm install -g pnpm
)
if not "!MISSING!"=="" (
    echo   Missing:!MISSING!
    echo   Please install and re-run.
    exit /b 1
)
echo   Prerequisites OK

echo.
echo Step 2/4: Initializing Firecrawl...
if exist "%SCRIPT_DIR%firecrawl\.git" (
    echo   Firecrawl already present
) else (
    echo   Cloning Firecrawl...
    cd /d "%SCRIPT_DIR%"
    git clone --depth=1 https://github.com/firecrawl/firecrawl.git
    if errorlevel 1 (
        echo   Warning: Failed to clone Firecrawl
        goto :config_step
    )
)
echo   Installing Firecrawl dependencies...
cd "%SCRIPT_DIR%firecrawl\apps\api"
pnpm install
cd /d "%SCRIPT_DIR%"

:config_step
echo.
echo Step 3/4: Creating configuration...
if exist "%SCRIPT_DIR%config.jsonc" (
    echo   Config already exists
) else (
    if exist "%SCRIPT_DIR%config.jsonc.template" (
        copy "%SCRIPT_DIR%config.jsonc.template" "%SCRIPT_DIR%config.jsonc"
    )
)

echo.
echo Step 4/4: Installing and starting Windows service...

sc query "%SERVICE_NAME%" 2>&1 | find "RUNNING" >nul
if not errorlevel 1 (
    echo   Service already running
    goto :done
)

sc query "%SERVICE_NAME%" 2>&1 | find "STOPPED" >nul
if not errorlevel 1 (
    echo   Service is stopped, starting...
    sc start "%SERVICE_NAME%"
    if errorlevel 1 (
        echo   Warning: Could not start service (run as Administrator)
    ) else (
        echo   Service started
    )
    goto :done
)

echo   Checking for NSSM...
where nssm 2>&1 | find "could not find" >nul
if errorlevel 1 (
    echo   NSSM not found, installing...
    winget install nssm.nssm --accept-package-agreements --accept-source-agreements --disable-interactivity --force
    if errorlevel 1 (
        echo   Warning: winget install failed. Try: winget install nssm.nssm
        echo   Falling back to sc.exe service registration...
        goto :sc_create
    )
    echo   NSSM installed
)

echo   Installing via NSSM (better service management)...
set BIN_PATH=%SCRIPT_DIR%universal-search-service.exe
nssm install "%SERVICE_NAME%" "%BIN_PATH%" run --config "%SCRIPT_DIR%config.jsonc"
nssm set "%SERVICE_NAME%" Start SERVICE_AUTO_START
nssm set "%SERVICE_NAME%" AppRestartDelay 5000
nssm set "%SERVICE_NAME%" AppExit Default Restart
nssm set "%SERVICE_NAME%" AppStdout "%LOG_DIR%universal-search.log"
nssm set "%SERVICE_NAME%" AppStderr "%LOG_DIR%universal-search.log"
nssm set "%SERVICE_NAME%" Priority SERVICE_NORMAL
echo   Starting service...
sc start "%SERVICE_NAME%"
if errorlevel 1 (
    echo   Warning: Could not start service automatically
) else (
    echo   Service installed and started via NSSM
)
goto :done

:sc_create
echo   Registering via sc.exe...
set BIN_PATH=%SCRIPT_DIR%universal-search-service.exe
set CMD="%BIN_PATH%" run --config "%SCRIPT_DIR%config.jsonc" --log-file "%LOG_DIR%universal-search.log"
sc create "%SERVICE_NAME%" binPath= %CMD% start= auto obj= ".\CurrentUser" DisplayName= "Universal Search Service"
if errorlevel 1 (
    echo   Warning: Service registration failed (run as Administrator)
    echo   Falling back to foreground mode...
    echo.
    "%BIN_PATH%" run --config "%SCRIPT_DIR%config.jsonc"
    goto :eof
)
echo   Starting service...
sc start "%SERVICE_NAME%"
if errorlevel 1 (
    echo   Warning: Could not start service automatically
) else (
    echo   Service installed and started via sc.exe
)

:done
echo.
echo   Verifying service health...
timeout /t 5 /nobreak
powershell -Command "try { $r = Test-NetConnection -ComputerName localhost -Port 3005 -WarningAction SilentlyContinue -TcpTestTimeOut 5; if ($r.TcpTestSucceeded) { Write-Host '   Service is healthy on port 3005' -ForegroundColor Green } else { Write-Host '   Warning: Service not responding on port 3005' -ForegroundColor Yellow } } catch { Write-Host '   Warning: Could not verify service health' -ForegroundColor Yellow }"

echo.
echo ============================================
echo  Initialization complete!
echo   Service: %SERVICE_NAME%
echo   Logs:    %LOG_DIR%
echo ============================================
