@echo off
setlocal enabledelayedexpansion

echo ============================================
echo  Universal Search Service - Uninstall
echo ============================================
echo.

set SERVICE_NAME=universal-search
set SCRIPT_DIR=%~dp0

if "%1"=="--purge" (
    echo Stopping service...
    sc query "%SERVICE_NAME%" 2>&1 | find "STATE" >nul && (
        nssm stop "%SERVICE_NAME%" 2>&1 | find /i "error" >nul
        nssm remove "%SERVICE_NAME%" confirm 2>&1 | find /i "error" >nul
        sc stop "%SERVICE_NAME%" 2>&1 | find /i "error" >nul
    )
    sc delete "%SERVICE_NAME%" 2>&1 | find /i "error" >nul

    echo Removing service files...
    taskkill /F /FI "WINDOWTITLE eq universal-search-service*" 2>&1 | find /i "error" >nul

    echo Purging installation directory...
    for %%f in (logs firecrawl) do (
        if exist "%SCRIPT_DIR%%%f" (
            echo   Removing %%f...
            rmdir /s /q "%SCRIPT_DIR%%%f"
        )
    )
    if exist "%SCRIPT_DIR%universal-search-service.exe" del "%SCRIPT_DIR%universal-search-service.exe"
    if exist "%SCRIPT_DIR%config.jsonc" del "%SCRIPT_DIR%config.jsonc"

    echo Uninstall complete (purged).
) else (
    echo Stopping and unregistering service...
    sc query "%SERVICE_NAME%" 2>&1 | find "STATE" >nul && (
        nssm stop "%SERVICE_NAME%" 2>&1 | find /i "error" >nul
        nssm remove "%SERVICE_NAME%" confirm 2>&1 | find /i "error" >nul
        sc stop "%SERVICE_NAME%" 2>&1 | find /i "error" >nul
    )
    sc delete "%SERVICE_NAME%" 2>&1 | find /i "error" >nul

    echo.
    echo Service unregistered. Data files remain in: %SCRIPT_DIR%
    echo Use --purge to remove all files including Firecrawl repo and logs.
)
