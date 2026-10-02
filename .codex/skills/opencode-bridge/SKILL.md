---
name: opencode-bridge
description: Start, read and drive another opencode session — or another robot — through its server HTTP API (POST /session, prompt_async, /session/status, /message). Use when a task needs a second agent in its own worktree, a child robot, or a session the owner watches in his TUI. Never by typing into a TUI through cmd_runner, never by a second process writing the same session.
---

# opencode-bridge — another session through its server (OUR route, inside the runtime)

- sv: { keywords: { server-api-bridge 0.30, one-server-per-worktree 0.24, robots-launch-robots 0.20,
        narrowed-authority 0.14, completion-predicate 0.12 },
        dominant: "An opencode agent starts or reaches another session through its server API, one server per worktree, never through a terminal." }
- status: ✓ verified end-to-end 2026-10-01 from OUTSIDE the runtime (Claude → the owner's TUI server): create
  session, send, a 33-`webfetch` tool turn finished `stop` in ~96 s, reply read back. Not yet run FROM a robot —
  and MEASURED the same day: a robot cannot reach its OWN TUI session, because that host binds no socket at all
  (see Open). The CLIENT half is proven from inside; it is the SERVER half that needs `--port`.

This file is written for an agent INSIDE opencode. Use your own tools (`bash` through the constitution, `read`,
`write`); the shell rules of AGENTS.md § Shell Command Restrictions apply unchanged.

## Why this route and no other

- A TUI is a rendering of state; reading its terminal is reading pictures. The server holds the state.
- `opencode run` WITHOUT `--attach` boots its own server and writes the same session database as any other
  server on that worktree — two writers, against the single-arbiter rule (AGENTS.md § Storage Paradigm).
  Measured 2026-10-01: headless `run` tool turns stalled until killed; the same tool turn over a TUI's server
  completed.
- The owner, 2026-10-01: driving opencode through cmd_runner keystrokes is «как операция на гланды через
  анус»; this bridge is «нормальный навык и очень полезный».

## Rules (owner, 2026-10-01)

1. **Launch from `bin/`** — `D:/zPython/opencode/bin/opencode.exe`, never `dist/bin` («туда мы его собираем и
   там же отлаживаем»). Launching only — nothing is edited, copied or built into `bin/`.
2. **One worktree = one server = one port = one terminal.** Launch from the worktree's own folder (each has its
   own database); ports never mix («порты не микшировать. Под каждую сессию свой терминал»).
3. **Many sessions per server are fine** — `POST /session` on the same port.
4. **A session is held by ONE server.** Before starting a server on a session, make sure no other server has it.
5. **A child never gets more authority than its parent.** Hand the child a bounded task, its oracle and its
   paths; do not hand it a permission you were not given.

## Recipe

1. **Choose the session** (no server needed): `D:/zPython/opencode/bin/opencode.exe session list -n 6`
   — or create one in step 4.
2. **Choose a free port** and check nobody listens on it (`Get-NetTCPConnection -LocalPort <port>`). There is no
   port registry yet: record `worktree → port → run_id` in your progress log entry.
3. **Start the server** through cmd_runner (a long-lived process — never a bare `start`), from the worktree:
   - owner watches → TUI in Windows Terminal:
     `cmd_runner start --terminal wt --cwd <worktree> --no-tail -- D:/zPython/opencode/bin/opencode.exe --session <id> --port <port>`
   - **`serve` is NOT our route (owner, 2026-10-01: «это веб двиг — он нам не нужен пока»).** `serve` is
     `Server.listen` (`src/cli/cmd/serve.ts:15`), which mounts the EMBEDDED WEB UI (`src/server/routes/ui.ts:11,25`
     — `opencode-web-ui.gen.ts`, `index.html`) and warns «OPENCODE_SERVER_PASSWORD is not set; server is
     unsecured» when launched bare. A robot to be driven is a TUI **with `--port`**, above — the port is what
     makes it reachable, the terminal is what the owner watches.
   The server binds `127.0.0.1` unless `--hostname` is given — keep it local.
4. **Talk HTTP** — base `http://127.0.0.1:<port>`, every call with `?directory=<url-encoded worktree>`:
   - `POST /session` `{"title":"…"}` → the new session's `id`
   - `POST /session/<id>/prompt_async`
     `{"model":{"providerID":"deepseek","modelID":"deepseek-flash"},"parts":[{"type":"text","text":"…"}]}` → `204`
   - `GET /session/status` → `{}` when idle; a busy session appears under its id
   - `GET /session/<id>/message?limit=N` → rows `{info:{id,role,finish,time}, parts:[…]}`
   - `GET /event` → server-sent events
   Write the request as a small script file under `experiments/<ISO-date>_<name>/` (UTF-8 JSON body) and run
   it — Russian text and quotes survive a file, not a quoted one-liner.
5. **Done means:** `/session/status` idle AND the newest ASSISTANT row has `finish` ∉ {null, `tool-calls`} and
   `time.completed` set. NOT «the last row has `finish`»: the server may append a Layer-1 summary (role
   `user`) right after the reply — that exact misread happened on 2026-10-01.
6. **Verify the child's result yourself.** Its reply is testimony; run the oracle you handed it.
7. **Stop** the server you started (`cmd_runner stop <run_id>`) when its work is verified — never a server you
   did not start.

## Open

- **A running TUI binds NOTHING — measured 2026-10-01 FROM INSIDE, so a robot cannot bridge to its own session.**
  `tasklist /FI "IMAGENAME eq opencode.exe"` → exactly one process (pid 19348, the TUI running the probe);
  `netstat -ano` filtered by that pid → 5 lines, ALL `ESTABLISHED … :443` outbound to the model providers and
  **zero LISTENING**. The port the outside bridge used (`127.0.0.1:4096`) answered `curl: (7) Failed to connect
  … after 2003 ms` / `HTTP:000`, while one stale client (pid 24568) sat in `SYN_SENT` toward it. Reachability
  from inside was proven in the same minute — `curl 8.21.0` answers and 12 loopback ports listen on this host —
  so the gap is the SERVER side, not the client.
  ⇒ **To be reachable, a session's host must be launched WITH `--port <p>` (recipe step 3)**: the in-process
  server a plain TUI starts binds no socket, which is also why the config `server.port` is ignored. And a child
  robot needs its OWN worktree — a second server on THIS worktree puts two writers on one session database,
  which the section above forbids.
- **`serve --port` is EXCLUDED — the web engine, not our route** (owner, 2026-10-01: «это веб двиг — он нам не
  нужен пока»). It is `Server.listen` (`src/cli/cmd/serve.ts:15`), which mounts the embedded web UI
  (`src/server/routes/ui.ts:11,25`), and it warns «OPENCODE_SERVER_PASSWORD is not set; server is unsecured»
  when launched bare. Recorded rather than deleted, with its lift signal: it returns only if a task needs the
  WEB surface itself — and then with a password set.
- **A child robot is therefore a TUI with its own `--port`, in its OWN worktree** — never a `serve`, and never a
  second server on a worktree that already has one (rule 2). Whether such a TUI can run unattended without a
  terminal, or needs `--terminal wt`, is NOT measured here.
- The TUI ignores `server.port` from config (`src/cli/cmd/tui/thread.ts:227` calls
  `resolveNetworkOptionsNoConfig(args)` without config) — Inferred by reading.
- No port registry: by the storage paradigm it belongs in the LMDB plane (`worktree:<path>:port`), not a file.
- The Claude-side twin of this skill lives in `.claude/skills/opencode-bridge/`; this one wins inside the
  runtime (scanned later, `src/skill/index.ts:155`), and plan `2026-09-30_no-foreign-skill-discovery` F1 removes
  the foreign root altogether.
