# Single canonical build script for universal-search-service.
#
# Produces: dist/universal-search/
#   universal-search-service.exe  — the binary
#   config.jsonc                  — copied from universal_search/config.jsonc
#   run.bat                       — foreground run script
#   install.bat                   — Windows service installer (NSSM)
#
# Config flow:
#   1. universal_search/config.jsonc         ← YOU EDIT (gitignored, has credentials)
#   2. build.ps1 copies it to →             ← dist/universal-search/config.jsonc
#   3. Binary reads it from next to itself   ← no --config flag needed

param(
    [string]$Profile = "release",
    [switch]$Clean = $false
)

$ErrorActionPreference = "Stop"
$RepoRoot = (Get-Item $PSScriptRoot).Parent.FullName
$DistDir = Join-Path $RepoRoot "dist\universal-search"
$ConfigSource = Join-Path $PSScriptRoot "config.jsonc"

Write-Host "===== Universal Search Service Build =====" -ForegroundColor Cyan
Write-Host "Profile : $Profile" -ForegroundColor White
Write-Host "Output  : $DistDir" -ForegroundColor White
Write-Host

# --- 1. Build binary ---
Write-Host "[1/4] Building binary ($Profile)..." -ForegroundColor Yellow
Set-Location $RepoRoot

if ($Clean) {
    Write-Host "  Cleaning..."
    cargo clean -p universal-search-service
}

cargo build --package universal-search-service --profile $Profile
if ($LASTEXITCODE -ne 0) {
    Write-Error "Build failed"
    exit 1
}

$CargoOutput = if ($Profile -eq "release") { "$RepoRoot\target\release" } else { "$RepoRoot\target\debug" }
$ExePath = Join-Path $CargoOutput "universal-search-service.exe"
if (-not (Test-Path $ExePath)) {
    Write-Error "Binary not found at $ExePath"
    exit 1
}
Write-Host "  Binary: $ExePath" -ForegroundColor Green

# --- 2. Prepare output directory ---
Write-Host
Write-Host "[2/4] Preparing $DistDir..." -ForegroundColor Yellow
if (Test-Path $DistDir) {
    Remove-Item -Recurse -Force $DistDir
    Write-Host "  Cleaned $DistDir" -ForegroundColor Green
}
New-Item -ItemType Directory -Force -Path $DistDir | Out-Null
Write-Host "  Created $DistDir" -ForegroundColor Green

# --- 3. Copy files ---
Write-Host
Write-Host "[3/4] Copying files..." -ForegroundColor Yellow

# Binary
Copy-Item $ExePath -Destination (Join-Path $DistDir "universal-search-service.exe") -Force
Write-Host "  Copied binary" -ForegroundColor Green

# Config — copy from universal_search/config.jsonc, never overwrite template
if (Test-Path $ConfigSource) {
    Copy-Item $ConfigSource -Destination (Join-Path $DistDir "config.jsonc") -Force
    Write-Host "  Copied config (from universal_search/config.jsonc)" -ForegroundColor Green
} else {
    Write-Warning "  universal_search/config.jsonc not found — run: cp config.jsonc.template config.jsonc"
}

# AGENT_GUIDE
$GuideSource = Join-Path $PSScriptRoot "AGENT_GUIDE.md"
if (Test-Path $GuideSource) {
    Copy-Item $GuideSource -Destination (Join-Path $DistDir "AGENT_GUIDE.md") -Force
    Write-Host "  Copied AGENT_GUIDE.md" -ForegroundColor Green
}

# --- 4. Create run scripts ---
Write-Host
Write-Host "[4/4] Creating scripts..." -ForegroundColor Yellow

$RunBat = @'
@echo off
echo ============================================
echo   Universal Search Service
echo ============================================
echo.
echo Starting on port 3005...
echo Config: config.jsonc
echo.
set "DIR=%~dp0"
set "DIR=%DIR:~0,-1%"
if not exist "%DIR%\logs" mkdir "%DIR%\logs"
"%~dp0universal-search-service.exe" --log-level info --log-file "%~dp0logs\universal-search.log" run
'@
$RunBat | Out-File -FilePath (Join-Path $DistDir "run.bat") -Encoding ascii -NoNewline
Write-Host "  Created run.bat" -ForegroundColor Green

$InstallBat = @'
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

'@
$InstallBat | Out-File -FilePath (Join-Path $DistDir "install.bat") -Encoding ascii -NoNewline
Write-Host "  Created install.bat" -ForegroundColor Green

# --- done ---
Write-Host
Write-Host "===== Build complete =====" -ForegroundColor Green
Write-Host "Output: $DistDir" -ForegroundColor White
Write-Host
Get-ChildItem $DistDir | Format-Table Name, Length -AutoSize
