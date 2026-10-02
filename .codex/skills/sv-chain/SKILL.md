---
name: sv-chain
description: Read your own reasoning trajectory out of past session transcripts — the @SV_FORMAT semantic-vector chain the kernel emits every turn. Use when you need what an earlier session was THINKING rather than what it changed: recovering intent after a fold or a /compact, finding when a topic entered and how it resolved, checking whether the chain lost a turn (a prev-md5 break makes recovery Guess, not Inferred), or auditing attention drift across a long run. Also use before re-deriving a decision you suspect a past session already reasoned through.
---

# SV chain — the reasoning trajectory reader

The kernel writes an `@SV_FORMAT` block after every response (weighted keywords, one-line
semantic dominant, `md5` / `prev-md5` / `parent-goal-md5`). This host has no reader for it, so
until now the trajectory was written every turn and never consulted — a writer with no reader.
`tools/claude_svchain.py` is the reader.

## Why this beats grepping the transcript

Transcript search returns a HIT: it answers "which session mentioned X" and shows text around
the match. To find something that way you must already guess its wording — which is exactly the
knowledge a later cycle lacks.

The SV chain has a FIXED ANCHOR (`Semantic dominant:`), so it is addressable without guessing,
and it returns a SEQUENCE rather than a hit: you see a topic enter, turn, and resolve, in order.

## Use

```bash
python tools/claude_svchain.py                  # newest session in this project
python tools/claude_svchain.py --list           # every transcript, with vector counts
python tools/claude_svchain.py <id-prefix>      # one session, e.g. 5aaef816
python tools/claude_svchain.py <id> --grep oracle   # only vectors matching, numbering preserved
```

Reading only — it never writes into the repo and never touches a live session. Transcripts live
under `~/.claude/projects/<slugged-cwd>/*.jsonl`; the script resolves the folder from the current
directory and walks UP through its parents, so it works from a subdirectory too.

An ambiguous id is an error naming the count, never a silent pick, and a block missing any of its
five fields is rejected rather than half-parsed — a vector with no id cannot be linked to and one
with no prev cannot be checked, so admitting it would weaken every edge after it.

Sibling on the other host: `tools/codex_svchain.py` reads Codex rollouts under the same contract.
Tests for both: `cd tools && python -m pytest tests/ -q` (they do not run from the repo root).

## Reading the output

Each line is `<n> <flag> <dominant>`. A `!` marks a **chain break**: the vector's `prev-md5` does
not name the vector actually before it, which means a turn was lost. Per `@SV_FORMAT`, anything
recovered across such a break is **Guess, not Inferred** — say so when you rely on it. Breaks are
invisible by eye and the reader finds them instantly; that is most of this tool's value.

A run ends with a count of breaks when there are any.

## What it does and does not give you

**Gives:** the trajectory — what each turn was focused on, in order, with attention weights, plus
where continuity actually broke. Good for "when did we decide to X", "what were we chasing before
the fold", "did this topic already get settled".

**Does not give:** the rejected alternatives. A vector records attention, not rationale — it shows
that turn 52 was lost, not what was being decided in it. For **why** an option was rejected you
still need an artifact with a dated reason: a comment in the source, a line in the plan, a
paragraph in a FINDING. The chain shows edges as a trajectory, never as an argument.

So: use this to locate, then read the artifact to understand. If the artifact does not exist, the
reasoning is gone regardless of what the chain shows — which is the standing argument for writing
the dated reason down when you cross it.

## Related

- `@SV_FORMAT` and the `prev-md5` rule: `.claude/reasoning_kernel.md` § 1.2
- The same defect class elsewhere (writer with no reader): memory revisions and summary digests,
  recorded in `plans/to_be_confirmed/2026-09-25_memory-self-injection.md`
