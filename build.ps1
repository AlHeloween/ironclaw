# IronClaw Build Script
# Builds the main ironclaw binary and the universal-search-service.
# Usage: .\build.ps1 [--release] [--clean] [--skip-main]

param(
    [switch]$release = $true,
    [switch]$clean = $false,
    [switch]$skip_main = $false
)

$ErrorActionPreference = "Stop"

# Add Git's Unix tools to PATH for libsql-ffi build (provides cp.exe, make.exe, etc.)
$git_usr_bin = "C:\Program Files\Git\usr\bin"
if (Test-Path $git_usr_bin) {
    $env:PATH = "$git_usr_bin;$env:PATH"
}

# Set C compiler for libsql-ffi C compilation
$clang = "C:\Program Files\LLVM\bin\clang.exe"
if (Test-Path $clang) {
    $env:CC = $clang
}

$ROOT = Split-Path -Parent $MyInvocation.MyCommand.Path
$DIST = Join-Path $ROOT "dist"
$PROFILE = if ($release) { "release" } else { "debug" }

Write-Host "=== IronClaw Full Build ===" -ForegroundColor Cyan
Write-Host "Profile: $PROFILE"
Write-Host "Output: $DIST"
Write-Host ""

# Build main ironclaw binary and universal-search-service (unless skipped)
if (-not $skip_main) {
    Write-Host "Building main ironclaw binary and universal-search-service..." -ForegroundColor Green
    Set-Location $ROOT
    if ($release) {
        cargo build --release 2>&1 | Write-Host
    } else {
        cargo build 2>&1 | Write-Host
    }

    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERROR: Build failed!" -ForegroundColor Red
        exit 1
    }
}

# ── Bundle universal-search-service next to main ironclaw binary ──────────────

Write-Host "Bundling universal-search-service with main binary..." -ForegroundColor Green
Set-Location $ROOT

$IRONCLAW_BIN = Join-Path $ROOT "target\$PROFILE\ironclaw.exe"
$UNIVERSAL_BIN = Join-Path $ROOT "target\$PROFILE\universal-search-service.exe"

if (Test-Path $IRONCLAW_BIN) {
    $IRONCLAW_DIR = Split-Path $IRONCLAW_BIN -Parent

    $BUNDLE_UNIVERSAL = Join-Path $IRONCLAW_DIR "universal-search"
    if (-not (Test-Path $BUNDLE_UNIVERSAL)) { New-Item -ItemType Directory -Path $BUNDLE_UNIVERSAL | Out-Null }

    if (Test-Path $UNIVERSAL_BIN) {
        Copy-Item $UNIVERSAL_BIN (Join-Path $BUNDLE_UNIVERSAL "universal-search-service.exe") -Force
        Write-Host "  universal-search-service bundled next to ironclaw.exe at: $IRONCLAW_DIR" -ForegroundColor Green
    } else {
        Write-Host "  Warning: universal-search-service.exe not found (build may be in progress)" -ForegroundColor Yellow
    }
} else {
    Write-Host "  Warning: ironclaw.exe not found at $IRONCLAW_BIN" -ForegroundColor Yellow
}

# ── Create standalone distribution folder ─────────────────────

Write-Host "Creating distribution folder..." -ForegroundColor Green
Set-Location $ROOT

$DIST_UNIVERSAL = Join-Path $DIST "universal-search"

if (-not (Test-Path $DIST_UNIVERSAL)) { New-Item -ItemType Directory -Path $DIST_UNIVERSAL | Out-Null }

# Copy binary
if (Test-Path $UNIVERSAL_BIN) {
    Copy-Item $UNIVERSAL_BIN (Join-Path $DIST_UNIVERSAL "universal-search-service.exe") -Force
}

# Copy README
$UNIVERSAL_SRC = Join-Path $ROOT "universal_search"
if (Test-Path (Join-Path $UNIVERSAL_SRC "README.md")) {
    Copy-Item (Join-Path $UNIVERSAL_SRC "README.md") (Join-Path $DIST_UNIVERSAL "README.md") -Force
}

# Copy config template
if (Test-Path (Join-Path $UNIVERSAL_SRC "config.jsonc")) {
    Copy-Item (Join-Path $UNIVERSAL_SRC "config.jsonc") (Join-Path $DIST_UNIVERSAL "config.jsonc") -Force
}

# Create example config file
$UNIVERSAL_CONFIG = @'
{
  "service": {
    "port": 3005,
    "bind_address": "127.0.0.1"
  },
  "local_search": {
    "enabled": true,
    "port": 3004,
    "watch_enabled": false,
    "indexes_dir": "./indexes",
    "indexes": [
      {
        "name": "ironclaw",
        "path": "../..",
        "languages": ["all"],
        "symbols_enabled": true,
        "enabled": true
      }
    ]
  },
  "web_search": {
    "firecrawl": {
      "api_url": "http://localhost:3002",
      "source": "local",
      "auto_start": true
    },
    "sourcegraph": {
      "access_token": null
    }
  },
  "bootstrap": {
    "auto_clone_firecrawl": true,
    "auto_install_deps": true
  }
}
'@

$UNIVERSAL_CONFIG | Out-File -FilePath (Join-Path $DIST_UNIVERSAL "config.jsonc") -Encoding utf8

# Create start script
$UNIVERSAL_START = '@echo off
echo Starting Universal Search Service...
universal-search-service.exe --config config.jsonc
pause'

$UNIVERSAL_START | Out-File -FilePath (Join-Path $DIST_UNIVERSAL "start.bat") -Encoding ascii

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
Write-Host "universal-search-service is bundled next to ironclaw.exe automatically." -ForegroundColor Yellow
Write-Host "No manual copying required."
