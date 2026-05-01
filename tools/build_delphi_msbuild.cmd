@echo off
rem Build a Delphi project via MSBuild, generating a .dproj from .dpr if missing.
rem
rem Usage:
rem   call tools\init_msvc.cmd
rem   call tools\init_delphi.cmd Win32
rem   tools\build_delphi_msbuild.cmd delphi\ADIDInstallerFMX.dpr Win64 Release

setlocal EnableExtensions EnableDelayedExpansion
cd /d "%~dp0.."

set "DPR=%~1"
set "PLATFORM=%~2"
set "CONFIG=%~3"
set "PROFILE=%~4"

if not defined DPR set "DPR=delphi\ADIDInstallerFMX.dpr"
if not defined PLATFORM set "PLATFORM=Win32"
if not defined CONFIG set "CONFIG=Release"
if not defined PROFILE set "PROFILE=%ADID_DELPHI_PROFILE%"

if not exist "%DPR%" (
  echo [ERROR] DPR not found: %DPR%
  exit /b 2
)

where msbuild >nul 2>&1
if errorlevel 1 (
  echo [ERROR] msbuild not found in PATH.
  echo [HINT] cmd.exe: call tools\init_msvc.cmd
  echo [HINT] PowerShell: . .\tools\init_msvc.ps1
  exit /b 2
)

for %%F in ("%DPR%") do set "DPROJ=%%~dpnF.dproj"

if not exist "!DPROJ!" (
  echo [BUILD] Generating dproj: !DPROJ!
  set "GEN_PLATFORMS=Win32;Win64"
  if /I "%PLATFORM%"=="Linux64" set "GEN_PLATFORMS=Win32;Win64;Linux64"
  if /I "%PLATFORM%"=="OSX64" set "GEN_PLATFORMS=Win32;Win64;OSX64"
  if /I "%PLATFORM%"=="OSXARM64" set "GEN_PLATFORMS=Win32;Win64;OSX64;OSXARM64"
  uv run scripts\generate_delphi_dproj.py --dpr "%DPR%" --platforms "%GEN_PLATFORMS%"
  if errorlevel 1 exit /b !errorlevel!
)

set "PROFILE_ARG="
if defined PROFILE set "PROFILE_ARG=/p:Profile=%PROFILE%"
if /I "%PLATFORM%"=="Linux64" if not defined PROFILE (
  echo [WARN] Linux64 build usually needs a configured Delphi Remote Profile.
  echo [WARN] Set ADID_DELPHI_PROFILE or pass it as arg4.
)

echo [BUILD] msbuild "!DPROJ!" /t:Build /p:Config=%CONFIG% /p:Platform=%PLATFORM% %PROFILE_ARG%
msbuild "!DPROJ!" /t:Build /p:Config=%CONFIG% /p:Platform=%PLATFORM% %PROFILE_ARG%
exit /b %errorlevel%
