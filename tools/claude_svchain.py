"""Read the semantic-vector trajectory from Claude Code session transcripts.

    python tools/claude_svchain.py --list
    python tools/claude_svchain.py <session-id-or-path>
    python tools/claude_svchain.py <session-id-or-path> --grep topic

Sibling of tools/codex_svchain.py — same contract, different host: this one reads
~/.claude/projects/<slugged-cwd>/*.jsonl instead of Codex rollouts.

The chain shows topic order and broken links, not the reasoning at a fork.
Read the located transcript and its artifacts to understand a decision.
This program only reads session files.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import re


ZERO = "0" * 32
KEYWORDS = re.compile(r"^Keywords:\s*(.+)$")
DOMINANT = re.compile(r"^Semantic dominant:\s*(.+)$")
OWN = re.compile(r"^md5:\s*([0-9a-fA-F]{32})$")
PREV = re.compile(r"^prev-md5:\s*([0-9a-fA-F]{32})$")
PARENT = re.compile(r"^parent-goal-md5:\s*([0-9a-fA-F]{32})$")


def vector_in(text: str):
    """One vector from one message, or None when any required field is missing.

    A partial block is rejected rather than half-parsed: a vector with no own id
    cannot be linked to, and one with no prev id cannot be checked for a break,
    so admitting it would silently weaken every edge after it.
    """
    found = {}
    for line in text.splitlines():
        for name, pattern in (
            ("keywords", KEYWORDS), ("dominant", DOMINANT),
            ("own", OWN), ("prev", PREV), ("parent", PARENT),
        ):
            match = pattern.match(line.strip())
            if match:
                found.setdefault(name, match.group(1).strip())
    if len(found) < 5:
        return None
    return {
        "keywords": found["keywords"],
        "dominant": found["dominant"],
        "own": found["own"].lower(),
        "prev": found["prev"].lower(),
    }


def vectors(path: Path) -> list[dict]:
    """Every vector in one transcript, in order."""
    chain: list[dict] = []
    with path.open(encoding="utf-8", errors="replace") as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue
            message = row.get("message")
            if not isinstance(message, dict) or message.get("role") != "assistant":
                continue
            content = message.get("content")
            blocks = [content] if isinstance(content, str) else content
            if not isinstance(blocks, list):
                continue
            for block in blocks:
                text = block if isinstance(block, str) else (
                    block.get("text") if isinstance(block, dict) and block.get("type") == "text" else None
                )
                if not isinstance(text, str):
                    continue
                vector = vector_in(text)
                if vector:
                    chain.append(vector)
    return chain


def format_chain(chain: list[dict], needle: str | None = None) -> list[str]:
    """Numbered lines with edge status. A filter narrows what PRINTS, never the
    numbering and never the edge check — an edge is always compared against the
    real predecessor in the full stream, or a filtered view would invent breaks."""
    lines: list[str] = []
    for index, vector in enumerate(chain):
        if index == 0 or vector["prev"] == ZERO:
            edge = "[RESET]" if vector["prev"] == ZERO else "[START]"
        elif vector["prev"] == chain[index - 1]["own"]:
            edge = "[LINK]"
        else:
            edge = f"[BREAK expected={chain[index - 1]['own'][:8]} got={vector['prev'][:8]}]"
        if needle and needle.lower() not in vector["dominant"].lower() \
                and needle.lower() not in vector["keywords"].lower():
            continue
        lines.append(f"{index + 1:>4} {edge}")
        lines.append(f"     {vector['dominant']}")
    return lines


def project_dir() -> Path:
    """The transcript folder for the current working directory."""
    projects = Path.home() / ".claude" / "projects"
    if not projects.is_dir():
        raise ValueError(f"no transcript root at {projects}")
    # Walk up: transcripts are keyed by the PROJECT root, so running from a
    # subdirectory must still find them. The control that caught this ran the
    # tool from tools/ and got "no folder matches".
    here = Path.cwd().resolve()
    for directory in (here, *here.parents):
        slug = str(directory).replace(":", "-").replace("\\", "-").replace("/", "-")
        if (projects / slug).is_dir():
            return projects / slug
        candidates = [d for d in projects.iterdir()
                      if d.is_dir() and d.name.lower().endswith(directory.name.lower())]
        if candidates:
            return max(candidates, key=lambda d: sum(f.stat().st_size for f in d.glob("*.jsonl")))
    raise ValueError(f"no folder under {projects} matches {here} or any parent")


def select_session(files: list[Path], needle: str) -> Path:
    """One transcript, or an error naming the count. Never a silent pick: two
    matches mean the caller meant something the selector cannot know."""
    direct = Path(needle)
    if direct.is_file():
        return direct
    matches = [f for f in files if needle in f.name]
    if not matches:
        raise ValueError(f"no session matched {needle!r}")
    if len(matches) > 1:
        raise ValueError(f"{needle!r} matched {len(matches)} sessions; name one")
    return matches[0]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("session", nargs="?", help="session id, file name, or path; default is the newest")
    parser.add_argument("--list", action="store_true", help="list transcripts with their vector counts")
    parser.add_argument("--grep", metavar="TEXT", help="only vectors whose dominant or keywords contain TEXT")
    args = parser.parse_args()

    folder = project_dir()
    files = sorted(folder.glob("*.jsonl"), key=lambda p: p.stat().st_mtime)
    if not files:
        raise ValueError(f"no transcripts in {folder}")

    if args.list:
        print(f"# {folder}")
        for path in files:
            print(f"{len(vectors(path)):>4} vectors  {path.stat().st_size / 1e6:>6.1f} MB  {path.name}")
        return 0

    target = select_session(files, args.session) if args.session else files[-1]
    chain = vectors(target)
    print(f"# {target.name}   {len(chain)} vectors")
    for line in format_chain(chain, args.grep):
        print(line)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
