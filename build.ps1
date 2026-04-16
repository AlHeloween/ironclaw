# IronClaw Search Services Build Script
# Builds the main ironclaw binary plus Local Code Search Service and
# Firecrawl Search Service, then bundles them together so users don't
# need to manually copy anything.
# Usage: .\build.ps1 [--release] [--clean] [--skip-main]

param(
    [switch]$release = $true,
    [switch]$clean = $false,
    [switch]$skip_main = $false
)

$ErrorActionPreference = "Stop"
$ROOT = Split-Path -Parent $MyInvocation.MyCommand.Path
$DIST = Join-Path $ROOT "dist"
$PROFILE = if ($release) { "release" } else { "debug" }

Write-Host "=== IronClaw Full Build ===" -ForegroundColor Cyan
Write-Host "Profile: $PROFILE"
Write-Host "Output: $DIST"
Write-Host ""

# Build main ironclaw binary (unless skipped)
if (-not $skip_main) {
    Write-Host "Building main ironclaw binary..." -ForegroundColor Green
    Set-Location $ROOT
    if ($release) {
        cargo build --release 2>&1 | Write-Host
    } else {
        cargo build 2>&1 | Write-Host
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERROR: Main ironclaw build failed!" -ForegroundColor Red
        exit 1
    }
}

# Build Local Code Search Service
Write-Host "Building Local Code Search Service..." -ForegroundColor Green
$LOCAL_SRC = Join-Path $ROOT "src\local_code_search"
$LOCAL_BIN = Join-Path $LOCAL_SRC "target\$PROFILE\local-code-search.exe"

Set-Location $LOCAL_SRC
if ($release) {
    cargo build --release 2>&1 | Write-Host
} else {
    cargo build 2>&1 | Write-Host
}

if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Local Code Search build failed!" -ForegroundColor Red
    exit 1
}

# Build Firecrawl Search Service
Write-Host "Building Firecrawl Search Service..." -ForegroundColor Green
$FIRECRAWL_SRC = Join-Path $ROOT "src\firecrawl-search-service"
$FIRECRAWL_BIN = Join-Path $FIRECRAWL_SRC "target\$PROFILE\firecrawl-search-service.exe"

Set-Location $FIRECRAWL_SRC
if ($release) {
    cargo build --release 2>&1 | Write-Host
} else {
    cargo build 2>&1 | Write-Host
}

if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Firecrawl Search build failed!" -ForegroundColor Red
    exit 1
}

# ── Bundle services next to main ironclaw binary ──────────────

Write-Host "Bundling services with main binary..." -ForegroundColor Green
Set-Location $ROOT

# Find the main ironclaw binary
$IRONCLAW_BIN = Join-Path $ROOT "target\$PROFILE\ironclaw.exe"
if (Test-Path $IRONCLAW_BIN) {
    $IRONCLAW_DIR = Split-Path $IRONCLAW_BIN -Parent

    # Create subdirectories for services next to ironclaw.exe
    $BUNDLE_LOCAL = Join-Path $IRONCLAW_DIR "local-code-search"
    $BUNDLE_FIRECRAWL = Join-Path $IRONCLAW_DIR "firecrawl-search"

    if (-not (Test-Path $BUNDLE_LOCAL)) { New-Item -ItemType Directory -Path $BUNDLE_LOCAL | Out-Null }
    if (-not (Test-Path $BUNDLE_FIRECRAWL)) { New-Item -ItemType Directory -Path $BUNDLE_FIRECRAWL | Out-Null }

    # Copy service binaries
    Copy-Item $LOCAL_BIN (Join-Path $BUNDLE_LOCAL "local-code-search.exe") -Force
    Copy-Item $FIRECRAWL_BIN (Join-Path $BUNDLE_FIRECRAWL "firecrawl-search-service.exe") -Force

    Write-Host "  Services bundled next to ironclaw.exe at: $IRONCLAW_DIR" -ForegroundColor Green
} else {
    Write-Host "  Warning: ironclaw.exe not found at $IRONCLAW_BIN (run 'cargo build' first to bundle)" -ForegroundColor Yellow
}

# ── Create standalone distribution folder ─────────────────────

Write-Host "Creating distribution folder..." -ForegroundColor Green
Set-Location $ROOT

$DIST_LOCAL = Join-Path $DIST "local-code-search"
$DIST_FIRECRAWL = Join-Path $DIST "firecrawl-search"

if (-not (Test-Path $DIST_LOCAL)) { New-Item -ItemType Directory -Path $DIST_LOCAL | Out-Null }
if (-not (Test-Path $DIST_FIRECRAWL)) { New-Item -ItemType Directory -Path $DIST_FIRECRAWL | Out-Null }

# Copy binaries
Copy-Item $LOCAL_BIN (Join-Path $DIST_LOCAL "local-code-search.exe") -Force
Copy-Item $FIRECRAWL_BIN (Join-Path $DIST_FIRECRAWL "firecrawl-search-service.exe") -Force

# Copy READMEs
Copy-Item (Join-Path $LOCAL_SRC "README.md") (Join-Path $DIST_LOCAL "README.md") -Force
Copy-Item (Join-Path $FIRECRAWL_SRC "README.md") (Join-Path $DIST_FIRECRAWL "README.md") -Force

# Copy config templates
Copy-Item (Join-Path $ROOT "local-code-search.jsonc.template") (Join-Path $DIST_LOCAL "config.jsonc.template") -Force
Copy-Item (Join-Path $ROOT "firecrawl-search.jsonc.template") (Join-Path $DIST_FIRECRAWL "config.jsonc.template") -Force

# Create example config files
$LOCAL_CONFIG = @'
{
  "service": {
    "port": 3004,
    "bind_address": "127.0.0.1",
    "watch_enabled": true
  },
  "indexes": [
    {
      "name": "ironclaw",
      "path": "../..",
      "languages": ["all"],
      "symbols_enabled": true,
      "enabled": true
    }
  ]
}
'@

$FIRECRAWL_CONFIG = @'
{
  "service": {
    "port": 3005,
    "bind_address": "127.0.0.1"
  },
  "firecrawl": {
    "api_url": "http://localhost:3002"
  },
  "sourcegraph": {
    "access_token": ""
  },
  "local_search": {
    "url": "http://127.0.0.1:3004",
    "enabled": true
  }
}
'@

$LOCAL_CONFIG | Out-File -FilePath (Join-Path $DIST_LOCAL "config.jsonc") -Encoding utf8
$FIRECRAWL_CONFIG | Out-File -FilePath (Join-Path $DIST_FIRECRAWL "config.jsonc") -Encoding utf8

# Create start scripts
$LOCAL_START = '@echo off
echo Starting Local Code Search Service...
local-code-search.exe --config config.jsonc
pause'

$FIRECRAWL_START = '@echo off
echo Starting Firecrawl Search Service...
firecrawl-search-service.exe
pause'

$LOCAL_START | Out-File -FilePath (Join-Path $DIST_LOCAL "start.bat") -Encoding ascii
$FIRECRAWL_START | Out-File -FilePath (Join-Path $DIST_FIRECRAWL "start.bat") -Encoding ascii

# Summary
Write-Host ""
Write-Host "=== Build Complete ===" -ForegroundColor Green
Write-Host "Distribution folder: $DIST"
Write-Host ""
Write-Host "Contents:" -ForegroundColor Cyan
Get-ChildItem -Path $DIST -Recurse -File | ForEach-Object {
    $rel = $_.FullName.Replace($DIST + '\', '')
    $size = if ($_.Length -gt 1MB) { "{0:N1} MB" -f ($_.Length / 1MB) } else { "{0:N0} KB" -f ($_.Length / 1KB) }
    Write-Host "  $rel ($size)"
}
Write-Host ""
Write-Host "Services are bundled next to ironclaw.exe automatically." -ForegroundColor Yellow
Write-Host "No manual copying required."
