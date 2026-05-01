# Build a Delphi project via MSBuild, generating a .dproj from .dpr if missing.
#
# Usage:
#   . .\tools\init_msvc.ps1
#   . .\tools\init_delphi.ps1 -Platform Win32
#   .\tools\build_delphi_msbuild.ps1 -Dpr delphi\ADIDInstallerFMX.dpr -Platform Win64 -Config Release

param(
  [string]$Dpr = 'delphi\ADIDInstallerFMX.dpr',
  [ValidateSet('Win32','Win64')]
  [string]$Platform = 'Win32',
  [string]$Config = 'Release',
  [string]$Profile = $env:ADID_DELPHI_PROFILE
)

$ErrorActionPreference = 'Stop'

if (-not (Test-Path -LiteralPath $Dpr)) {
  Write-Host "[ERROR] DPR not found: $Dpr"
  exit 2
}

if (-not (Get-Command msbuild -ErrorAction SilentlyContinue)) {
  Write-Host "[ERROR] msbuild not found in PATH."
  Write-Host "[HINT] Run: . .\\tools\\init_msvc.ps1"
  exit 2
}

$dproj = [System.IO.Path]::ChangeExtension((Resolve-Path -LiteralPath $Dpr).Path, '.dproj')
if (-not (Test-Path -LiteralPath $dproj)) {
  Write-Host "[BUILD] Generating dproj: $dproj"
  $genPlatforms = 'Win32;Win64'
  if ($Platform -eq 'Linux64') { $genPlatforms = 'Win32;Win64;Linux64' }
  if ($Platform -eq 'OSX64') { $genPlatforms = 'Win32;Win64;OSX64' }
  if ($Platform -eq 'OSXARM64') { $genPlatforms = 'Win32;Win64;OSX64;OSXARM64' }
  uv run scripts\\generate_delphi_dproj.py --dpr $Dpr --platforms $genPlatforms
}

$extra = @()
if ($Profile) { $extra += ("/p:Profile=$Profile") }
if ($Platform -eq 'Linux64' -and -not $Profile) {
  Write-Host '[WARN] Linux64 build usually needs a configured Delphi Remote Profile.'
  Write-Host '[WARN] Set ADID_DELPHI_PROFILE or pass -Profile.'
}

Write-Host "[BUILD] msbuild $dproj /t:Build /p:Config=$Config /p:Platform=$Platform $($extra -join ' ')"
msbuild $dproj /t:Build /p:Config=$Config /p:Platform=$Platform @extra
exit $LASTEXITCODE
