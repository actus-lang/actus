#!/usr/bin/env python3
"""Check moved Rust bodies and visibility boundaries against a base tree."""

from __future__ import annotations

from collections import Counter
from dataclasses import dataclass
import pathlib
import re
import subprocess
import sys

VISIBILITY = re.compile(r"^(pub(?:\((?:super|crate|in\s+[^)]+)\))?)\s+(.*)$")
ITEM = re.compile(
    r"^(?:unsafe\s+|async\s+|const\s+|extern\s+\"[^\"]+\"\s+)*"
    r"(fn|struct|enum|union|impl|trait|const|static|type|macro_rules!|mod|use)\b\s*([A-Za-z_][A-Za-z0-9_]*)?"
)

@dataclass(frozen=True)
class Item:
    kind: str
    name: str
    visibility: str
    line: int

def git_lines(*args: str) -> list[str]:
    return subprocess.check_output(["git", *args], text=True).splitlines()

def module_path(path: str) -> tuple[str, ...]:
    relative = path.removeprefix("src/").removesuffix(".rs")
    if relative.endswith("/mod"):
        relative = relative.removesuffix("/mod")
    return tuple(relative.split("/"))

def parent_module(path: str) -> tuple[str, ...]:
    return module_path(path)[:-1]

def parse_items(text: str) -> list[Item]:
    result = []
    for number, raw in enumerate(text.splitlines(), 1):
        stripped = raw.strip()
        match = VISIBILITY.match(stripped)
        visibility = match.group(1) if match else "private"
        rest = match.group(2) if match else stripped
        item = ITEM.match(rest)
        if item and (match or item.group(1) == "mod"):
            name = rest if item.group(1) == "use" else item.group(2) or ""
            result.append(Item(item.group(1), name, visibility, number))
    return result

def base_tree(base: str, paths: list[str]) -> dict[str, str]:
    result = {}
    for path in git_lines("ls-tree", "-r", "--name-only", base, "--", *paths):
        if path.endswith(".rs"):
            result[path] = subprocess.check_output(["git", "show", f"{base}:{path}"], text=True)
    return result

def working_tree(paths: list[str]) -> dict[str, str]:
    names = git_lines("ls-files", "--", *paths) + git_lines("ls-files", "--others", "--exclude-standard", "--", *paths)
    return {path: pathlib.Path(path).read_text(encoding="utf-8") for path in sorted(set(names)) if path.endswith(".rs") and pathlib.Path(path).is_file()}

def rename_pairs(base: str, paths: list[str]) -> dict[str, str]:
    result = {}
    for raw in git_lines("diff", "-M", "--name-status", base, "--", *paths):
        fields = raw.split("\t")
        if len(fields) == 3 and fields[0].startswith("R"):
            result[fields[2]] = fields[1]
    return result

def body_line(raw: str, skipping_use: bool) -> tuple[str | None, bool]:
    line = raw.strip()
    if skipping_use:
        return None, ";" not in line
    if line.startswith(("use ", "pub use ", "pub(crate) use ", "pub(super) use ")):
        return None, ";" not in line
    if not line:
        return None, False
    match = VISIBILITY.match(line)
    body = match.group(2) if match else line
    if body.startswith(("mod ", "use ")):
        return None, ";" not in body
    return body, False

def normalized(texts: list[str]) -> Counter[str]:
    result = Counter()
    skipping_use = False
    declaration = False
    for text in texts:
        for raw in text.splitlines():
            line, skipping_use = body_line(raw, skipping_use)
            if line is None:
                continue
            stripped = line.strip()
            if declaration:
                declaration = "{" not in stripped and ";" not in stripped
                continue
            if re.match(r"(?:pub(?:\([^)]*\))?\s+)?(?:unsafe\s+|async\s+|const\s+|extern\s+\"[^\"]+\"\s+)*(fn|struct|enum|union|impl|trait|const|static|type)\b", stripped):
                declaration = "{" not in stripped and ";" not in stripped
                continue
            if "::" not in stripped:
                result[line] += 1
    return result

def visibility_scope(visibility: str, path: str) -> tuple[str, ...] | str:
    if visibility == "pub":
        return "external"
    if visibility == "pub(crate)":
        return "crate"
    if visibility == "pub(super)":
        return parent_module(path)
    if visibility.startswith("pub(in "):
        return tuple(visibility.removeprefix("pub(in ").removesuffix(")").removeprefix("crate::").split("::"))
    return "private"

def translate_scope(
    scope: tuple[str, ...] | str,
    visibility: str,
    old_path: str,
    current_path: str,
    pairs: dict[str, str],
) -> tuple[str, ...] | str:
    if not isinstance(scope, tuple) or not visibility.startswith("pub(in "):
        return scope
    replacements = []
    for new_path, renamed_old_path in pairs.items():
        old_module = module_path(renamed_old_path)
        if scope[:len(old_module)] == old_module:
            replacements.append((len(old_module), old_module, parent_module(new_path)))
    if not replacements:
        return scope
    _, old_module, new_module = max(replacements)
    return new_module + scope[len(old_module):]

def wider(new: tuple[str, ...] | str, old: tuple[str, ...] | str) -> bool:
    if new == old:
        return False
    if new == "external":
        return old != "external"
    if new == "crate":
        return old not in {"external", "crate"}
    return isinstance(new, tuple) and isinstance(old, tuple) and len(new) < len(old) and old[:len(new)] == new

def compare_items(old_tree: dict[str, str], current_tree: dict[str, str], pairs: dict[str, str]) -> list[str]:
    candidates = {}
    for path, text in old_tree.items():
        for item in parse_items(text):
            candidates.setdefault((item.kind, item.name), []).append((path, item))
    expansions = []
    for current_path, text in current_tree.items():
        mapped_old = pairs.get(current_path, current_path)
        for item in parse_items(text):
            choices = candidates.get((item.kind, item.name), [])
            if not choices:
                continue
            matching = [choice for choice in choices if choice[0] == mapped_old]
            if len(matching) == 1:
                old_path, old_item = matching[0]
            else:
                same_scope = {
                    visibility_scope(choice[1].visibility, choice[0]) for choice in choices
                }
                preferred = [
                    choice for choice in choices
                    if choice[0].endswith("/mod.rs")
                    and pathlib.PurePosixPath(choice[0]).parent
                    == pathlib.PurePosixPath(current_path).parent
                ]
                if preferred:
                    choices = preferred
                elif len(choices) > 1 and len(same_scope) > 1:
                    print(f"ambiguous: {current_path}:{item.line}: {item.kind} {item.name}")
                old_path, old_item = min(choices, key=lambda choice: len(visibility_scope(choice[1].visibility, choice[0])) if isinstance(visibility_scope(choice[1].visibility, choice[0]), tuple) else -1)
            old_scope = translate_scope(
                visibility_scope(old_item.visibility, old_path),
                old_item.visibility,
                old_path,
                current_path,
                pairs,
            )
            if wider(visibility_scope(item.visibility, current_path), old_scope):
                expansions.append(f"{current_path}:{item.line}: {old_item.visibility} -> {item.visibility}: {item.kind} {item.name}")
    return sorted(set(expansions))

def main() -> int:
    if len(sys.argv) < 3:
        raise SystemExit(f"usage: {sys.argv[0]} BASE PATH [PATH ...]")
    old_tree = base_tree(sys.argv[1], sys.argv[2:])
    current_tree = working_tree(sys.argv[2:])
    pairs = rename_pairs(sys.argv[1], sys.argv[2:])
    old_bodies, new_bodies = normalized(list(old_tree.values())), normalized(list(current_tree.values()))
    expansions = compare_items(old_tree, current_tree, pairs)
    print("differences:")
    for line in sorted((old_bodies - new_bodies).elements()):
        print(f"- {line}")
    for line in sorted((new_bodies - old_bodies).elements()):
        print(f"+ {line}")
    print("visibility changes:")
    print("visibility expansions:")
    print("\n".join(expansions))
    return 1 if old_bodies != new_bodies or expansions else 0

if __name__ == "__main__":
    raise SystemExit(main())
