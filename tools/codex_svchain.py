"""Read the semantic-vector trajectory from Codex session transcripts.

    python tools/codex_svchain.py --list
    python tools/codex_svchain.py <session-id-or-path>
    python tools/codex_svchain.py <session-id-or-path> --grep topic

The chain shows topic order and broken links, not the reasoning at a fork.
Read the located transcript and its artifacts to understand a decision.
This program only reads session files.
"""

from __future__ import annotations

import argparse
import json
import logging
import os
from pathlib import Path
import re


ZERO = "0" * 32
LOG = logging.getLogger(__name__)
FIELDS = (
    re.compile(r"^Keywords:\s*(.+)$"),
    re.compile(r"^Semantic dominant:\s*(.+)$"),
    re.compile(r"^md5:\s*([0-9a-fA-F]{32})$"),
    re.compile(r"^prev-md5:\s*([0-9a-fA-F]{32})$"),
    re.compile(r"^parent-goal-md5:\s*([0-9a-fA-F]{32})$"),
)


def session_files(root: Path) -> list[Path]:
    return sorted(root.glob("**/rollout-*.jsonl"), key=lambda path: (path.stat().st_mtime, str(path)))


def assistant_texts(path: Path):
    with path.open(encoding="utf-8", errors="replace") as transcript:
        for line_number, line in enumerate(transcript, 1):
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                LOG.debug("skipping malformed JSONL row at %s:%d", path, line_number)
                continue
            if row.get("type") != "response_item":
                continue
            item = row.get("payload")
            if not isinstance(item, dict) or item.get("type") != "message" or item.get("role") != "assistant":
                continue
            for part in item.get("content", []):
                if isinstance(part, dict) and part.get("type") == "output_text":
                    value = part.get("text")
                    if isinstance(value, str):
                        yield value


def vector_in(text: str) -> dict[str, str] | None:
    lines = text.splitlines()
    matches: list[dict[str, str]] = []
    for start in range(len(lines) - len(FIELDS) + 1):
        values = [pattern.fullmatch(line.strip()) for pattern, line in zip(FIELDS, lines[start:])]
        if all(values):
            keywords, dominant, own_id, previous_id, parent_id = (match.group(1) for match in values)
            matches.append({
                "keywords": keywords.strip(),
                "dominant": dominant.strip(),
                "md5": own_id.lower(),
                "prev": previous_id.lower(),
                "parent": parent_id.lower(),
            })
    return matches[-1] if matches else None


def vectors(path: Path) -> list[dict[str, str]]:
    return [vector for text in assistant_texts(path) if (vector := vector_in(text)) is not None]


def select_session(files: list[Path], selector: str | None) -> Path:
    if selector is None:
        if not files:
            raise ValueError("no Codex session transcripts found")
        return files[-1]
    explicit = Path(selector)
    if explicit.is_file():
        return explicit
    matches = [path for path in files if selector in path.name]
    if len(matches) != 1:
        raise ValueError(f"session selector matched {len(matches)} transcripts: {selector}")
    return matches[0]


def format_chain(chain: list[dict[str, str]], needle: str | None = None) -> list[str]:
    output: list[str] = []
    for index, vector in enumerate(chain):
        if needle and needle.casefold() not in (vector["dominant"] + " " + vector["keywords"]).casefold():
            continue
        if index == 0:
            edge = "START"
        elif vector["prev"] == chain[index - 1]["md5"]:
            edge = "LINK"
        elif vector["prev"] == ZERO:
            edge = "RESET"
        else:
            edge = f"BREAK expected={chain[index - 1]['md5']} got={vector['prev']}"
        output.append(f"{index + 1:>4} [{edge}] {vector['prev']} -> {vector['md5']}  {vector['dominant']}")
        output.append(f"       {vector['keywords']}")
    return output


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("session", nargs="?", help="unique session id, file name, or path; default newest")
    parser.add_argument("--list", action="store_true", help="list sessions with vector counts")
    parser.add_argument("--grep", metavar="TEXT", help="filter dominant and keywords")
    args = parser.parse_args()

    root = Path(os.environ.get("CODEX_HOME", Path.home() / ".codex")) / "sessions"
    files = session_files(root)
    if args.list:
        for path in files:
            print(f"{len(vectors(path)):>4}  {path.name}")
        return 0
    try:
        target = select_session(files, args.session)
    except ValueError as error:
        parser.error(str(error))
    chain = vectors(target)
    print(f"# {target} ({len(chain)} vectors)")
    for line in format_chain(chain, args.grep):
        print(line)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
