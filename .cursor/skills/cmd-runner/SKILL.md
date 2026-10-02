---
name: cmd-runner
description: Run interactive commands safely via cmd_runner with per-run logs, inbox bridge, terminal auto-detection, and image capture support.
kernel-gates: [G7, G8]
---

# cmd-runner

**Kernel gates:** G7 (execute bounded implementation steps), G8 (run oracle instruments — tests, builds, probes) — see reasoning-kernel.mdc.

Use this skill when a command may be:
- long/noisy,
- interactive (prompts, TUIs),
- crash-prone or likely to destabilize the agent when run directly,
- render terminal graphics (Kitty/iTerm2/Sixel protocols).

## When to use cmd_runner

**Use cmd_runner for:**
- Long-running builds: `cargo build --release`, `msbuild`, `make`, `gradle build`
- Package installs: `npm install`, `pip install -r requirements.txt`, `cargo fetch`
- Test suites: `pytest`, `cargo test`, `npm test`, `mvn test`
- Interactive TUIs: `htop`, `ncurses` apps, installers with prompts
- Image-rendering commands: `chafa`, `timg`, any Kitty/Sixel protocol output
- Crash-prone or unstable commands
- Commands that produce thousands of lines of output

**Do NOT use cmd_runner for:**
- Quick checks: `list`, `glob`, `git status`, `echo`, `cat` (use product tools, not shell)
- Simple file operations: `cp`, `mv`, `rm`
- Commands that complete in <1 second
- Commands you need to see output from immediately

## Process window behavior

- All subprocesses open with `SW_SHOWMINNOACTIVE` (minimized, taskbar-visible, no focus steal).
- Terminal selection via `--terminal` (see below).
- Default terminal auto-detected by priority.

## What cmd_runner is

- Windows + Linux compatible.
- Default backend uses ConPTY (Windows) or PTY (Linux) for terminal I/O.
- `--direct-terminal` is a Windows Terminal-only graphics backend: the child inherits the actual WT console, so SIXEL capability negotiation and output are not consumed by ConPTY.
- Raw pipe capture mode (`--raw`) for image protocol passthrough — converts Kitty/iTerm2/Sixel to `[IMG:PROTO:b64]` in logs.
- Terminal auto-detection: conhost > wezterm > Windows Terminal > bash (Windows); wezterm > guake > yakuake > xterm > bash (Linux).
- conhost is the Windows default because it reliably supports minimized-no-focus-steal (SW_SHOWMINNOACTIVE).
  Windows Terminal (wt.exe) windows belong to WindowsTerminal.exe, not the launcher PID — use `--terminal wt` explicitly if needed.
- Logs: `logs/cmd_runner/<run_id>/`
- Programmatic input bridge: `logs/cmd_runner/<run_id>/inbox.jsonl`
- Can be launched from any working directory (binary found via PATH or absolute path).

## How to run it

- Repo checkout: `.\cmd_runner.exe <command> ...`
- Release bundle: `cmd_runner.exe <command> ...`
- Via adm: `tools/adm.exe --cmd-runner <args...>`
- From any directory: `cmd_runner <command> ...` (if on PATH)
- Version: `cmd_runner --version` (prints build date)

## Core workflow

### 1) Start a run

```
cmd_runner start [--terminal HOST[=ARGS]] [--cols N --rows N] [--direct-terminal] [--raw|--no-raw] [--cwd PATH] -- <command ...>
```

- Prints `run_id` and `inbox=` path.
- Auto-tails last 5 lines after 500ms (default; `--auto-tail 0` to disable).
- `--shell cmd|pwsh|bash` — explicit shell wrapper.
- `--terminal` — select terminal host (see Terminal section below).
- `--cols` / `--rows` — initial ConPTY grid; with the default `wt` launch they also become `wt --size COLS,ROWS`. Use a taller grid (for example, `80x60`) for portrait graphics rendered by `chafa`.
- `--raw` — force raw pipes (no ConPTY, non-interactive batch commands only).
- `--no-raw` — force ConPTY (interactive mode, default).
- Default: ConPTY (interactive) — supports full send/inbox, TUI apps, interactive shells.
- `--direct-terminal` — require `--terminal wt`; preserve SIXEL/terminal graphics and retain `status`, `stop`, job control, and inbox `send` while the payload runs. The direct WT window stays visible. Its stdout/stderr cannot be captured to runner logs by design.

### Graphics in Windows Terminal

Use the direct-terminal backend when `chafa` or another renderer chooses a symbol fallback under ConPTY:

```
cmd_runner start --terminal wt --direct-terminal --cols 80 --rows 60 --keep-open --cwd D:\zPython\ADID_Python\experiments\Vision -- .\dragon.bat
cmd_runner status <run_id>
cmd_runner send <run_id> --keys ENTER
cmd_runner screenshot <run_id>
cmd_runner screenshot <run_id> --out D:\captures\dragon.png
```

- The host enables `ENABLE_VIRTUAL_TERMINAL_PROCESSING` before it creates the child, then the child inherits the real WT console handles.
- Screenshot identifies the visible WT tab by its per-run title. Do not minimize or close that window before capture.
- Use `tail` for ConPTY/raw runs. Direct-terminal image output is intentionally terminal-only, not a log artifact.

### 2) Check status

```
cmd_runner list [--all] [--limit N] [--json]
cmd_runner status <run_id> [--json]
```

### 3) Tail output

```
cmd_runner tail <run_id> [--follow] [-n N] [--text|--stdout] [--wait-ms N]
```

**Key facts:**

- `cmd_runner tail <run_id>` (without `--follow`) reads the **full captured log** directly from disk (`stdout_text.log`). It works **after the run has finished** — no `--follow` needed.
- `--follow` streams the output in real time. It **exits automatically** when the run reaches a terminal state.
- `--follow --wait-ms 0` follows until the run finishes, then returns the full output. This is the simplest way to stream-to-completion.
- `-n N` shows only the last N lines of the log.

**Reading output after a run finishes (canonical flow):**

```
cmd_runner start -- python my_script.py       # returns run_id
cmd_runner wait <run_id>                      # blocks until done
cmd_runner tail <run_id>                      # reads full captured output
cmd_runner tail <run_id> -n 50                # last 50 lines only
```

**Streaming output until completion:**

```
cmd_runner start -- long_command -- <args>    # returns run_id
cmd_runner tail <run_id> --follow --wait-ms 0 # streams until run exits, then stops
```

**Note:** `--auto-tail N` (default 5, on `start`) and `--send-tail N` (default 3, on `send`) are **snapshots only** — they show the last N lines at that moment, not the full output. Use `cmd_runner tail <run_id>` for the complete log.

### 4) Send input (inbox bridge)

```
cmd_runner send <run_id> --text "uptime" --crlf
cmd_runner send <run_id> --keys "ctrl+c"
cmd_runner send <run_id> --keys "TEXT:root,ENTER"
cmd_runner send <run_id> --stdin-file FILEPATH
cmd_runner send <run_id> --text-file FILEPATH
cmd_runner send <run_id> --text "whoami" --text-as-b64 --crlf
cmd_runner send <run_id> --hex 03
```

- `--keys` tokens: `LEFT,RIGHT,UP,DOWN,HOME,END,INSERT,DELETE,TAB,ESC,ENTER,BACKSPACE,ctrl+a..ctrl+z,TEXT:text,CHAR:char,HEX:hex`
- `--crlf` appends CRLF after text.
- Auto-tails last 3 lines after send (default; `--send-tail 0` to disable).

**Quoting tips (PowerShell):**
```
cmd_runner send <id> --crlf -- "python3 -c 'print(1+2)'"
cmd_runner send <id> --crlf -- 'echo ~~~hello~~~'           # ~ instead of "
cmd_runner send <id> --crlf -- @'                           # PS here-string
echo LINE1
echo LINE2
'@
```

### 5) Stop / Wait

```
cmd_runner stop <run_id> --reason "done"
cmd_runner wait <run_id> [--timeout-s N] [--json]
cmd_runner killall [--force] [--json]
```

## Terminal selection

```
cmd_runner start --terminal wezterm -- <command ...>
cmd_runner start --terminal wt -- <command ...>
cmd_runner start --terminal conhost -- <command ...>
cmd_runner start --terminal alacritty -- <command ...>       # any exe on PATH
```

### Custom terminal arguments

```
cmd_runner start --terminal wezterm="--config-file ~/.config/wezterm/wezterm.lua" -- <command ...>
cmd_runner start --terminal wt="-w new -d C:\project" -- <command ...>
```

### Auto-detection (default)

| Priority | Windows | Linux |
|----------|---------|-------|
| 1 | wezterm | wezterm |
| 2 | wt (Windows Terminal) | guake |
| 3 | conhost | yakuake |
| 4 | bash (git-bash) | xterm |
| 5 | — | bash |

First terminal found on PATH is used. Omit `--terminal` for auto-detection.

## Image capture (--raw)

**Important:** Raw mode is for non-interactive batch commands only. It does NOT support:
- Interactive input (send/inbox)
- TUI applications
- Shell sessions

When running image-rendering batch commands (`chafa`, `timg`, etc.):

```
cmd_runner start --raw -- <image_command ...>
```

- Raw pipes bypass ConPTY filtering — Kitty/Sixel/iTerm2 escape sequences survive.
- Output is captured in logs with `[IMG:...]` markers.
- Text-based tools (`tail`, `assert`, `snapshot`) work on clean text.
- `--no-raw` forces ConPTY mode (no image capture in logs).

## Screenshot

```
cmd_runner screenshot <run_id> [--out PATH]
```

- Captures a visible `--direct-terminal --terminal wt` window to PNG.
- Without `--out`, writes `logs/cmd_runner/<run_id>/screenshot.png`.

## Auto-tail options

| Flag | Default | Description |
|------|---------|-------------|
| `--auto-tail N` | 5 | After start, show last N lines |
| `--send-tail N` | 3 | After send, show last N lines |
| `--wait-ms N` | 500 | Delay before auto-tail (ms) |
| `--tail-timeout N` | 0 | Retry tail up to N seconds |

## Log layout

```
logs/cmd_runner/<run_id>/
  meta.json       — session metadata (argv, cwd, normalized_argv, terminal)
  state.json      — current state (status, exit_code, timestamps)
  stdout.log      — captured output (with [IMG:...] markers in raw mode)
  stdout_text.log — ANSI-stripped text
  stderr.log      — stderr output
  in.log          — inbox messages
  inbox.jsonl     — input bridge (append JSONL to send input)
  payload.cmd     — generated wrapper script (when applicable)
```

## SSH session example

```
cmd_runner start -- ssh root@host                       # start interactive SSH
cmd_runner send <run_id> --text "uptime; df -h" --crlf  # send command
cmd_runner send <run_id> --keys ctrl+c                  # interrupt
cmd_runner send <run_id> --keys ctrl+d                  # end session
```

## Notes

- Bare Windows script shims (npm, npx) auto-wrapped in PowerShell `-File`.
- `.bat`/`.cmd` files auto-wrapped in `cmd.exe /c` to prevent `SearchPathW` AV.
- `add_crlf` defaults to `false` (no implicit Enter).
- All subprocess windows open minimized.
- Direct-terminal Windows Terminal windows are the exception: they remain visible for graphics rendering and screenshot capture.
- `wait` without `--timeout-s` blocks indefinitely.
