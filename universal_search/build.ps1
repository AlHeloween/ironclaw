# Build script for universal-search-service on Windows

param(
    [string]$Profile = "release",
    [switch]$Clean = $false
)

$ErrorActionPreference = "Stop"

if ($Clean) {
    Write-Host "Cleaning build artifacts..."
    cargo clean -p universal-search-service
}

Write-Host "Building universal-search-service ($Profile)..."
cargo build --package universal-search-service --profile $Profile

if ($LASTEXITCODE -ne 0) {
    Write-Error "Build failed"
    exit 1
}

$OutputDir = if ($Profile -eq "release") { "target/release" } else { "target/debug" }
$ExePath = Join-Path $OutputDir "universal-search-service.exe"

if (-not (Test-Path $ExePath)) {
    Write-Error "Binary not found at $ExePath"
    exit 1
}

Write-Host "Build successful: $ExePath"

# Create distribution folder
$DistDir = "dist/universal-search"
if (Test-Path $DistDir) { Remove-Item -Recurse -Force $DistDir }
New-Item -ItemType Directory -Force -Path $DistDir | Out-Null

# Copy binary
Copy-Item $ExePath -Destination (Join-Path $DistDir "universal-search-service.exe")

# Copy config template
if (Test-Path "universal_search/config.jsonc.template") {
    Copy-Item "universal_search/config.jsonc.template" -Destination (Join-Path $DistDir "config.jsonc.template")
}

# Create start.bat
$StartBat = @"
@echo off
echo Starting Universal Search Service...
"%~dp0universal-search-service.exe" run
"@
$StartBat | Out-File -FilePath (Join-Path $DistDir "start.bat") -Encoding ascii

Write-Host "Distribution created at $DistDir"
