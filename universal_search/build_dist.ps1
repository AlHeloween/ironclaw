# Build self-contained distribution package for Universal Search Service
# The resulting dist/ folder can be copied to any Windows machine and run directly.

param(
    [string]$Profile = "release",
    [switch]$Clean = $false
)

$ErrorActionPreference = "Stop"
$RootDir = (Get-Item $PSScriptRoot).Parent.FullName
$ServiceDir = Join-Path $RootDir "universal_search"
$DistDir = Join-Path $ServiceDir "dist"

Write-Host "============================================" -ForegroundColor Cyan
Write-Host "  Building Universal Search Distribution" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan
Write-Host

# Step 1: Build binary
Write-Host "[1/4] Building binary ($Profile)..." -ForegroundColor Yellow
Set-Location $RootDir
if ($Clean) {
    cargo clean -p universal-search-service
}
cargo build --package universal-search-service --profile $Profile
if ($LASTEXITCODE -ne 0) {
    Write-Error "Build failed"
    exit 1
}

$OutputDir = if ($Profile -eq "release") { "$RootDir/target/release" } else { "$RootDir/target/debug" }
$ExePath = Join-Path $OutputDir "universal-search-service.exe"
if (-not (Test-Path $ExePath)) {
    Write-Error "Binary not found at $ExePath"
    exit 1
}
Write-Host "  Binary: $ExePath" -ForegroundColor Green

# Step 2: Prepare distribution folder
Write-Host ""
Write-Host "[2/4] Preparing distribution folder..." -ForegroundColor Yellow
if (Test-Path $DistDir) {
    Remove-Item -Recurse -Force $DistDir
}
New-Item -ItemType Directory -Force -Path $DistDir | Out-Null

# Copy binary
Copy-Item $ExePath -Destination (Join-Path $DistDir "universal-search-service.exe")
Write-Host "  Copied binary" -ForegroundColor Green

# Copy config
if (Test-Path (Join-Path $ServiceDir "config.jsonc")) {
    Copy-Item (Join-Path $ServiceDir "config.jsonc") -Destination (Join-Path $DistDir "config.jsonc")
    Write-Host "  Copied config.jsonc" -ForegroundColor Green
}

# Copy AGENT_GUIDE.md
if (Test-Path (Join-Path $ServiceDir "AGENT_GUIDE.md")) {
    Copy-Item (Join-Path $ServiceDir "AGENT_GUIDE.md") -Destination (Join-Path $DistDir "AGENT_GUIDE.md")
    Write-Host "  Copied AGENT_GUIDE.md" -ForegroundColor Green
}

# Step 3: Create run.bat
Write-Host ""
Write-Host "[3/4] Creating run script..." -ForegroundColor Yellow
$RunBat = @"
@echo off
echo ============================================
echo   Universal Search Service
echo ============================================
echo.
echo Starting Universal Search Service...
echo   Config: %~dp0config.jsonc
echo   Port: 3005
echo.
"%~dp0universal-search-service.exe" run
"@
$RunBat | Out-File -FilePath (Join-Path $DistDir "run.bat") -Encoding ascii
Write-Host "  Created run.bat" -ForegroundColor Green

# Step 4: Create install.bat (self-contained service installer)
Write-Host ""
Write-Host "[4/4] Creating service installer..." -ForegroundColor Yellow
$InstallBat = @"
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

powershell -Command "try { `$null = Test-NetConnection -ComputerName localhost -Port 3005 -WarningAction SilentlyContinue; if (`$?) { Write-Host 'Service started on port 3005' -ForegroundColor Green } else { Write-Host 'Service starting...' -ForegroundColor Yellow } } catch {}"

echo.
echo Service installed.
echo   To stop:   nssm stop %SERVICE_NAME%
echo   To remove: nssm remove %SERVICE_NAME% confirm
echo   Status:   nssm status %SERVICE_NAME%
pause
"@
$InstallBat | Out-File -FilePath (Join-Path $DistDir "install.bat") -Encoding ascii
Write-Host "  Created install.bat" -ForegroundColor Green

# Create uninstall.bat
$UninstallBat = @"
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
"@
$UninstallBat | Out-File -FilePath (Join-Path $DistDir "uninstall.bat") -Encoding ascii
Write-Host "  Created uninstall.bat" -ForegroundColor Green

Write-Host ""
Write-Host "============================================" -ForegroundColor Green
Write-Host "  Distribution ready at:" -ForegroundColor Green
Write-Host "  $DistDir" -ForegroundColor White
Write-Host "============================================" -ForegroundColor Green
Write-Host ""
Write-Host "Contents:" -ForegroundColor Cyan
Get-ChildItem -Path $DistDir | Format-Table Name, Length -AutoSize
