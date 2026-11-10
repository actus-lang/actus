#!/usr/bin/env python3
"""Compare moved Rust bodies while ignoring module and visibility changes."""

from collections import Counter
import subprocess
import sys


VISIBILITY_PREFIXES = ("pub(super) ", "pub(crate) ", "pub ")


def body_line(raw: str, skipping_use: bool) -> tuple[str | None, bool]:
    line = raw.strip()
    if skipping_use:
        return None, ";" not in line
    if line.startswith(("use ", "pub use ")):
        return None, ";" not in line
    if not line or line.startswith(("mod ", "pub mod ")):
        return None, False
    for prefix in VISIBILITY_PREFIXES:
        if line.startswith(prefix):
            line = line[len(prefix) :]
            break
    return line, False


def normalized(lines: list[str]) -> Counter[str]:
    result: Counter[str] = Counter()
    skipping_use = False
    for raw in lines:
        line, skipping_use = body_line(raw, skipping_use)
        if line is not None:
            result[line] += 1
    return result


def diff_lines(base: str, paths: list[str]) -> tuple[list[str], list[str], list[str]]:
    diff = subprocess.check_output(["git", "diff", base, "--", *paths], text=True)
    removed: list[str] = []
    added: list[str] = []
    visibility: list[str] = []
    for raw in diff.splitlines():
        if raw.startswith(("+++", "---")):
            continue
        if raw.startswith(("+", "-")):
            line = raw[1:].strip()
            if not line:
                continue
            if line.startswith(VISIBILITY_PREFIXES):
                visibility.append(raw)
            (added if raw[0] == "+" else removed).append(line)
    return removed, added, visibility


def main() -> int:
    if len(sys.argv) < 3:
        raise SystemExit(f"usage: {sys.argv[0]} BASE PATH [PATH ...]")
    removed, added, visibility = diff_lines(sys.argv[1], sys.argv[2:])
    missing = normalized(removed) - normalized(added)
    extra = normalized(added) - normalized(removed)
    print("differences:")
    for line in sorted(missing.elements()):
        print(f"- {line}")
    for line in sorted(extra.elements()):
        print(f"+ {line}")
    print("visibility changes:")
    for line in visibility:
        print(line)
    return 1 if missing or extra else 0


if __name__ == "__main__":
    raise SystemExit(main())
