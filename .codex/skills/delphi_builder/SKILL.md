---
name: delphi_builder
description: Build Delphi (VCL/FMX) projects from the command line with MSBuild, including environment initialization (MSVC + rsvars) and packaging notes for ADIDInstaller.
---

# delphi_builder

## Goal

Provide a repeatable Windows CLI workflow for building Delphi `.dproj` projects using MSBuild without touching IDE options.

## Commands added to `adm`

- `tools/adm.exe --init-msvc [out.cmd]`
  - Generates `tools/init_msvc.cmd` (default) that first respects an already-initialized `cl` + `msbuild` environment, otherwise calls Visual Studio `VsDevCmd.bat`.
- `tools/adm.exe --init-delphi [out.cmd]`
  - Generates `tools/init_delphi.cmd` (default) that resolves Delphi from `adm.json` (`delphi.bds`) or from `where dcc64` / `where dcc32`, then calls `rsvars.bat` / `rsvars64.bat`.

## Hello World walkthrough

### The .dpr source

```delphi
program hello_console;

{$APPTYPE CONSOLE}
{$R *.res}

uses
  System.SysUtils;

begin
  try
    Writeln('Hello from Delphi!');
    Writeln(Format('Target: %s', [TOSVersion.ToString]));
    Writeln(Format('Time  : %s', [FormatDateTime('yyyy-mm-dd hh:nn:ss', Now)]));
  except
    on E: Exception do
      Writeln(E.ClassName, ': ', E.Message);
  end;
end.
```

### Step-by-step build (PowerShell)

```
# 1. Init MSVC — puts msbuild on PATH
. .\tools\init_msvc.ps1

# 2. Init Delphi — puts dcc64/dcc32 on PATH, sets BDS env vars
. .\tools\init_delphi.ps1 -Platform Win64

# 3. Build via the wrapper (auto-generates .dproj from .dpr when missing)
.\tools\build_delphi_msbuild.ps1 -Dpr hello_console.dpr -Platform Win64 -Config Release
```

### Step-by-step build (cmd.exe)

```
call tools\init_msvc.cmd
call tools\init_delphi.cmd Win64
tools\build_delphi_msbuild.cmd hello_console.dpr Win64 Release
```

### What each script does

**`init_msvc.*`** — Detects an existing MSVC environment (already-loaded `cl` + `msbuild` on PATH); if absent, calls Visual Studio `VsDevCmd.bat` via the VS installer COM API to set up the native x64 toolchain.  This is needed because Delphi MSBuild requires the C++ toolchain for resource compilation (`cgrc.exe` / `brcc32.exe`).

**`init_delphi.*`** — Resolves the Delphi installation root.  Priority order:
1. `adm.json` `delphi.bds` field (explicit path)
2. `where dcc64` / `where dcc32` (PATH discovery)
3. Falls back to common install locations

Then calls `rsvars.bat` (32-bit) or `rsvars64.bat` (64-bit) which sets `BDS`, `Platform`, `Config`, and the Delphi `bin` directory on PATH.  After this, `dcc64.exe`, `dcc32.exe`, and Delphi-aware `msbuild` are available.

**`build_delphi_msbuild.*`** — The build wrapper:
1. If no `.dproj` exists next to the `.dpr`, auto-generates one via `scripts/internal/generate_delphi_dproj.py` with the correct platform and search-path settings.
2. Invokes `msbuild /t:Build /p:Platform=<Win32|Win64> /p:Config=<Release|Debug> <project>.dproj`
3. Output goes to `<project_dir>/bin/<Platform>/<Config>/<project>.exe`

### How the release pipeline invokes it

The full release flow (`_release.cmd` → `scripts/release.py`) chains these for `cmd_runner.exe`:

```
call tools\init_msvc.cmd
call tools\init_delphi.cmd Win64
call tools\build_delphi_msbuild.cmd delphi\cmd_runner.dpr Win64 Release
```

This is the same three-step pattern — just with the project path pointing at `delphi\cmd_runner.dpr`.

## Typical build flow (Windows)

1. Init MSVC:
   - `cmd.exe`: `call tools\\init_msvc.cmd`
   - PowerShell (env persists only when dot-sourced): `. .\\tools\\init_msvc.ps1`
2. Init Delphi:
   - `cmd.exe`: `call tools\\init_delphi.cmd` (or `call tools\\init_delphi.cmd Win64`)
   - PowerShell (dot-source): `. .\\tools\\init_delphi.ps1 -Platform Win64`
3. Build:
   - Prefer wrapper (auto-generates `.dproj` from `.dpr` when missing):
     - `tools\\build_delphi_msbuild.cmd delphi\\cmd_runner.dpr Win32 Release`
     - `tools\\build_delphi_msbuild.cmd delphi\\cmd_runner.dpr Win64 Release`
   - PowerShell wrapper:
     - `.\\tools\\build_delphi_msbuild.ps1 -Dpr delphi\\cmd_runner.dpr -Platform Win64 -Config Release`

Repo shortcut (FMX installer, Win64 Release):
- `delphi\\build_installer_msbuild.cmd Win64 Release`

Notes:
- Delphi 2009+ commonly uses `/p:config=<ConfigName>` (case-insensitive).
- Delphi 2007 commonly uses `/p:Configuration=<ConfigName>`.

## Library paths / SDK paths (without IDE option edits)

Prefer build-time configuration (project/group build props) instead of editing global IDE options. Strategy examples:
- Use per-repo build scripts that call `rsvars.bat` and set additional environment variables before MSBuild.
- Keep build-time path decisions in versioned files (project-local) so CI and team machines stay aligned.

## Linux64 (WSL2) overview (FMX only)

- VCL cannot target Linux; you need an FMX project.
- Linux64 builds typically require a configured Delphi **Remote Profile** + imported SDK (SDK Manager).
- Use `ADID_DELPHI_PROFILE` (or wrapper arg) to pass the Remote Profile name to MSBuild:
  - `set ADID_DELPHI_PROFILE=my_wsl_profile`
  - `tools\\build_delphi_msbuild.cmd path\\to\\YourFMXApp.dpr Linux64 Release`

Details: `docs\\delphi_linux_wsl2.md`

## FMX cross-platform builds (concept)

FMX targets like Android/iOS/macOS require additional platform SDKs and platform services (vendor tooling). The CLI MSBuild entrypoint is similar (`/p:Platform=<target>`), but environment prerequisites differ by target.

Examples (targets vary by project/Delphi version):
- `Platform=Android`
- `Platform=iOSDevice64` / `Platform=iOSSimulator`
- `Platform=OSX64`
- `Platform=Linux64`
