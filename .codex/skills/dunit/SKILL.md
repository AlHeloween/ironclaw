---
name: dunit
description: Run and maintain Delphi DUnit tests for Delphi projects.
---

# dunit

## Purpose

Provide a reliable workflow for building and running Delphi DUnit tests (console runner) and interpreting failures.

## Prerequisites

- Delphi toolchain on PATH (at minimum `dcc32`).
- Initialize the environment via `rsvars.bat` for your Delphi version, or use the repo helpers:
  - `call tools\init_msvc.cmd` (optional)
  - `call tools\init_delphi.cmd Win32`

## Commands

- Build + run all DUnit tests:
  - `tests\run_tests.cmd`

- Build tests only:
  - `tests\build_tests.cmd`

## Notes (Win64)

- Project test scripts commonly build the test runner via MSBuild for `Win64`; inspect the local test script before assuming platform/configuration.

## Typical project layout

- Test project: `artefacts\examples\project-agnostic\tests\ProjectTests.dpr`
- Test units: `artefacts\examples\project-agnostic\tests\TestCore.pas`, `artefacts\examples\project-agnostic\tests\TestServices.pas`

## How to add a new test

1. Add a new unit such as `tests\TestSomething.pas`.
2. Register it in the local DUnit project file. See `artefacts\examples\project-agnostic\tests\ProjectTests.dpr` for the documentation fixture.
3. Re-run the local DUnit test command.

## Notes

- DUnit assertions typically come from `TestFramework` (e.g. `CheckEquals`, `CheckTrue`, `CheckNotNull`).
- Prefer testing pure units (no VCL dependencies) so tests run headless and deterministic.
