@echo off
set SERVICE_NAME=universal-search

echo Universal Search Service Status
echo ===============================

sc query "%SERVICE_NAME%" 2>&1 | find "STATE" >nul
if errorlevel 1 (
    echo Service not registered
)

echo.
echo Health check:
powershell -Command "try { $r = Test-NetConnection -ComputerName localhost -Port 3005 -WarningAction SilentlyContinue -TcpTestTimeOut 3; if ($r.TcpTestSucceeded) { Write-Host 'Port 3005: OPEN' -ForegroundColor Green } else { Write-Host 'Port 3005: CLOSED' -ForegroundColor Red } } catch { Write-Host 'Port 3005: UNKNOWN' -ForegroundColor Yellow }"
