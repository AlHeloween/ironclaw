@echo off
echo ============================================
echo   Installing Firecrawl Windows Service
echo ============================================
echo.

where nssm >nul 2>&1
if errorlevel 1 (
    echo ERROR: NSSM not found. Install with: winget install nssm.nssm
    pause
    exit /b 1
)

cd /d "%~dp0firecrawl\apps\api"

nssm stop Firecrawl 2>nul
nssm remove Firecrawl confirm 2>nul

echo Installing Firecrawl service...

nssm install Firecrawl "node.exe" "dist/src/index.js"
nssm set Firecrawl AppDirectory "%CD%"
nssm set Firecrawl Start SERVICE_AUTO_START
nssm set Firecrawl AppRestartDelay 5000
nssm set Firecrawl AppExit Default Restart
nssm set Firecrawl AppStdout -
nssm set Firecrawl AppStderr -
nssm set Firecrawl AppPriority BELOW_NORMAL_PRIORITY_CLASS

nssm set Firecrawl AppEnvironmentExtra ^
    "USE_GO_MARKDOWN_PARSER=false" ^
    "SKIP_DOCKER_SERVICES=true" ^
    "NUQ_RABBITMQ_URL=" ^
    "NODE_NO_WARNINGS=1" ^
    "PORT=3002" ^
    "HOST=0.0.0.0" ^
    "NUQ_DATABASE_URL=postgresql://postgres:1412@localhost:5432/nuq" ^
    "NUQ_DATABASE_URL_LISTEN=postgresql://postgres:1412@localhost:5432/nuq" ^
    "REDIS_URL=redis://localhost:6379" ^
    "USE_DB_AUTHENTICATION=false" ^
    "BULL_AUTH_KEY=test"

echo.
echo Starting Firecrawl...
nssm start Firecrawl

timeout /t 3 /nobreak >nul

powershell -Command "try { $null = Test-NetConnection -ComputerName localhost -Port 3002 -WarningAction SilentlyContinue; if ($?) { Write-Host 'Firecrawl service started successfully on port 3002' -ForegroundColor Green } else { Write-Host 'Service may still be starting... check firecrawl-service.log' -ForegroundColor Yellow } } catch {}"

echo.
echo To stop:   nssm stop Firecrawl
echo To start:  nssm start Firecrawl
echo To remove: nssm remove Firecrawl confirm
echo Logs:     sc qc Firecrawl
pause
