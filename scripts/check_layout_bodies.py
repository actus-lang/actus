#!/usr/bin/env python3
"""Prove that a source-layout change moved Rust items without changing them.

Usage: check_layout_bodies.py BASE PATH [PATH ...]

The base tree is read from git (BASE); the current tree is the working tree,
including untracked files. Items under PATH are compared item by item:

* bodies: every fn/struct/enum/union/const/static/type/trait/impl header and
  every method must exist on both sides with identical tokens. Whitespace,
  rustfmt trailing commas and visibility are ignored; module-path prefixes
  (`crate::`, `self::`, `super::`, and names of modules under src/) are
  ignored only after the path is resolved on both sides to the same item;
* paths: every module-qualified path, and every bare call or capitalised
  name, is resolved in the base and current crates; both sides must reach an
  item with identical tokens (so `support::unsupported` cannot silently
  become another function named `unsupported`);
* visibility: no item, struct field or `mod` declaration may become visible
  to more of the crate than the base allowed. The narrowest allowed scope is
  the common module of every new location that holds code from the base
  scope.

Exit status is 1 on any body difference, path-target change, or visibility
expansion.
"""

from __future__ import annotations

import io
import os
import re
import subprocess
import sys
import tarfile
from collections import defaultdict
from typing import NamedTuple, Optional

PUNCT = [
    "<<=", ">>=", "...", "..=", "::", "->", "=>", "==", "!=", "<=", ">=", "&&",
    "||", "+=", "-=", "*=", "/=", "%=", "^=", "&=", "|=", "<<", ">>", "..",
]
IDENT = re.compile(r"(?:r#)?[A-Za-z_]\w*", re.ASCII)
NUMBER = re.compile(r"\d[\d_]*(?:\.\d[\d_]*)?(?:[eE][+-]?[\d_]+)?\w*", re.ASCII)
RAW_STRING = re.compile(r'(?:br|cr|r)(#*)"')
ITEM_KINDS = {"fn", "struct", "enum", "union", "const", "static", "type", "trait", "impl"}
QUALIFIERS = {"unsafe", "async", "default", "extern"}
ANCHORS = {"crate", "self", "super"}
OPEN = {"(": ")", "[": "]", "{": "}"}
CLOSERS = (")", "]", "}", ">")
NOT_REFERENCE_AFTER = {".", "::", "fn", "struct", "enum", "type", "trait", "const", "static",
                       "mod", "let", "mut"}
MAX_DEPTH = 12


# --------------------------------------------------------------------------
# Lexing
# --------------------------------------------------------------------------

class Lexer:
    """Tokens as (text, line); comments dropped, literals kept whole."""

    def __init__(self, text):
        self.text = text
        self.i = 0
        self.line = 1
        self.tokens = []
        self.scanners = (self.space, self.comment, self.raw_string, self.string,
                         self.character, self.number, self.identifier, self.punctuation)

    def run(self):
        while self.i < len(self.text):
            for scan in self.scanners:
                if scan():
                    break
        return self.tokens

    def emit(self, end, value=None):
        self.tokens.append((self.text[self.i:end] if value is None else value, self.line))
        self.line += self.text.count("\n", self.i, end)
        self.i = end
        return True

    def word_boundary(self):
        return self.i == 0 or not (self.text[self.i - 1].isalnum() or self.text[self.i - 1] == "_")

    def space(self):
        if not self.text[self.i].isspace():
            return False
        self.line += self.text[self.i] == "\n"
        self.i += 1
        return True

    def comment(self):
        if self.text.startswith("//", self.i):
            end = self.text.find("\n", self.i)
            self.i = len(self.text) if end < 0 else end
            return True
        if not self.text.startswith("/*", self.i):
            return False
        depth, end = 1, self.i + 2
        while end < len(self.text) and depth:
            if self.text.startswith("/*", end) or self.text.startswith("*/", end):
                depth += 1 if self.text[end] == "/" else -1
                end += 2
            else:
                end += 1
        self.line += self.text.count("\n", self.i, end)
        self.i = end
        return True

    def raw_string(self):
        found = RAW_STRING.match(self.text, self.i)
        if not found or not self.word_boundary():
            return False
        closing = '"' + found.group(1)
        end = self.text.find(closing, found.end())
        return self.emit(len(self.text) if end < 0 else end + len(closing))

    def string(self):
        c = self.text[self.i]
        prefixed = c in "bc" and self.text.startswith('"', self.i + 1) and self.word_boundary()
        if c != '"' and not prefixed:
            return False
        end = self.i + (2 if prefixed else 1)
        while end < len(self.text) and self.text[end] != '"':
            end += 2 if self.text[end] == "\\" else 1
        return self.emit(end + 1)

    def character(self):
        c = self.text[self.i]
        if not (c == "'" or (c == "b" and self.text.startswith("'", self.i + 1))):
            return False
        start = self.i + (1 if c == "'" else 2)
        if self.text.startswith("\\", start):
            end = self.text.find("'", start + 2)
            return self.emit(len(self.text) if end < 0 else end + 1)
        if self.text.startswith("'", start + 1):
            return self.emit(start + 2)
        lifetime = IDENT.match(self.text, start) if c == "'" else None
        if lifetime:
            return self.emit(lifetime.end(), "'" + lifetime.group(0))
        return False

    def number(self):
        found = NUMBER.match(self.text, self.i)
        if not found:
            return False
        return self.emit(found.end())

    def identifier(self):
        found = IDENT.match(self.text, self.i)
        if not found:
            return False
        return self.emit(found.end())

    def punctuation(self):
        width = next((len(p) for p in PUNCT if self.text.startswith(p, self.i)), 1)
        return self.emit(self.i + width)


def lex(text):
    return Lexer(text).run()


# --------------------------------------------------------------------------
# Token helpers
# --------------------------------------------------------------------------

def token_at(tokens, i, default=""):
    return tokens[i][0] if 0 <= i < len(tokens) else default


def matching(tokens, i):
    """Index after the bracket group that starts at tokens[i]."""
    stack = []
    while i < len(tokens):
        t = tokens[i][0]
        if t in OPEN:
            stack.append(OPEN[t])
        elif stack and t == stack[-1]:
            stack.pop()
            if not stack:
                return i + 1
        i += 1
    return i


def scan_top_level(tokens, i):
    """First `{` or `;` outside ( ) and [ ]: (index, token) or (len, "")."""
    depth = 0
    while i < len(tokens):
        t = tokens[i][0]
        if t in ("(", "["):
            depth += 1
        elif t in (")", "]"):
            depth -= 1
        elif depth == 0 and t in ("{", ";"):
            return i, t
        i += 1
    return i, ""


def item_end(tokens, i):
    """End of an item: `;` at depth 0 or the close of its first `{` group."""
    index, found = scan_top_level(tokens, i)
    if found == "{":
        return matching(tokens, index)
    return index + 1 if found else index


def open_brace(tokens, i):
    index, found = scan_top_level(tokens, i)
    return index if found == "{" else -1


def without_trailing_commas(tokens):
    """rustfmt adds or removes a trailing comma when it re-wraps a list."""
    return [t for k, t in enumerate(tokens)
            if not (t[0] == "," and token_at(tokens, k + 1) in CLOSERS)]


def parse_visibility(tokens, i):
    if token_at(tokens, i) != "pub":
        return "private", i
    if token_at(tokens, i + 1) != "(":
        return "pub", i + 1
    close = matching(tokens, i + 1)
    return "pub(" + " ".join(t[0] for t in tokens[i + 2:close - 1]) + ")", close


def skip_attributes(tokens, i, kept=None):
    while token_at(tokens, i) == "#" and token_at(tokens, i + 1) == "[":
        end = matching(tokens, i + 1)
        if kept is not None:
            kept.extend(tokens[i:end])
        i = end
    return i


def prefix_end(tokens, i, names):
    """Index of the first token after a strippable module prefix at i."""
    if i > 0 and tokens[i - 1][0] == "::":
        return i
    j = i
    while (token_at(tokens, j + 1) == "::"
           and (tokens[j][0] in ANCHORS or tokens[j][0] in names)):
        j += 2
    return j


def normalize(tokens, names):
    """Token texts with resolvable module prefixes removed."""
    out = []
    i = 0
    while i < len(tokens):
        j = prefix_end(tokens, i, names)
        out.append(token_at(tokens, j))
        i = j + 1
    return out


# --------------------------------------------------------------------------
# Items
# --------------------------------------------------------------------------

class Item:
    __slots__ = ("kind", "name", "owner", "visibility", "tokens", "module", "path",
                 "line", "trait_member", "text", "fields")

    def __init__(self, kind, name, owner, visibility, tokens, module, path, line, trait_member):
        self.kind, self.name = kind, name
        self.owner = tuple(without_trailing_commas(list(owner))) if owner else owner
        tokens = without_trailing_commas(tokens)
        self.fields = []
        if kind in ("struct", "union"):
            self.fields, tokens = field_visibilities(tokens)
        self.visibility, self.tokens = visibility, tokens
        self.module, self.path, self.line = module, path, line
        self.trait_member = trait_member
        self.text = []


def field_visibilities(tokens):
    """Remove field visibility tokens; return ([visibility per field], tokens)."""
    keyword = next((k for k, t in enumerate(tokens) if t[0] in ("struct", "union")), 0)
    start = next((k for k, t in enumerate(tokens) if k > keyword and t[0] in ("(", "{")), None)
    if start is None:
        return [], tokens
    close = matching(tokens, start)
    kept = tokens[:start + 1]
    fields = []
    k = start + 1
    while k < close - 1:
        k = skip_attributes(tokens, k, kept)
        visibility, k = parse_visibility(tokens, k)
        fields.append(visibility)
        k = copy_field(tokens, k, close - 1, kept)
    kept.extend(tokens[close - 1:])
    return fields, kept


def copy_field(tokens, k, end, kept):
    """Copy one field's tokens (through its comma) into kept; return next index."""
    depth = 0
    while k < end:
        t = tokens[k][0]
        kept.append(tokens[k])
        k += 1
        if t in OPEN or t == "<":
            depth += 1
        elif t in (")", "]", "}") or (t == ">" and depth > 0):
            depth -= 1
        elif t == "," and depth == 0:
            break
    return k


class Module:
    def __init__(self, path):
        self.path = path
        self.defs = defaultdict(list)   # name -> [Item | ("mod", path)]
        self.uses = {}                  # binding -> segments
        self.globs = []                 # [segments]


class ModDecl(NamedTuple):
    name: str
    module: tuple
    visibility: str
    path: str
    line: int


class Scope(NamedTuple):
    tokens: list
    end: int
    module: tuple
    path: str
    context: str
    owner: Optional[tuple]


class Head(NamedTuple):
    """An item's leading part: attributes, visibility and keyword position."""
    kept: list
    visibility: str
    body_start: int
    keyword: int
    word: str
    line: int


class Crate:
    def __init__(self, files):
        self.modules: dict[tuple, Module] = {}
        self.items: list[Item] = []
        self.mod_decls: list[ModDecl] = []
        self.module_names: set[str] = set()
        self.handlers = {"use": self.parse_use_item, "mod": self.parse_mod,
                         "impl": self.parse_block_item, "trait": self.parse_block_item,
                         "macro_rules": self.parse_macro_rules}
        overrides = file_modules(files)
        for path, text in sorted(files.items()):
            module = overrides.get(path, module_path(path))
            if module is not None:
                self.module_names.update(module)
                tokens = lex(text)
                self.parse_scope(Scope(tokens, len(tokens), module, path, "module", None), 0)
        # Snapshot the keys: self.module() below adds parent modules.
        for module in tuple(self.modules):
            for depth in range(1, len(module) + 1):
                self.module(module[:depth - 1]).defs[module[depth - 1]].append(("mod", module[:depth]))

    def module(self, path):
        if path not in self.modules:
            self.modules[path] = Module(path)
        return self.modules[path]

    def parse_scope(self, scope, i):
        while i < scope.end:
            i = self.parse_item(scope, i)

    def parse_item(self, scope, i):
        tokens = scope.tokens
        if token_at(tokens, i) == "#" and token_at(tokens, i + 1) == "!":
            return matching(tokens, i + 2)
        kept: list[tuple[str, int]] = []
        attributes_end = skip_attributes(tokens, i, kept)
        if attributes_end >= scope.end:
            return scope.end
        visibility, body_start = parse_visibility(tokens, attributes_end)
        keyword = skip_qualifiers(tokens, body_start, scope.end)
        word = token_at(tokens, keyword) if keyword < scope.end else ""
        line = tokens[min(keyword, len(tokens) - 1)][1]
        head = Head(kept, visibility, body_start, keyword, word, line)
        handler = self.handlers.get(word)
        if handler is not None:
            return handler(scope, head)
        if word in ITEM_KINDS and keyword + 1 < scope.end:
            return self.parse_plain_item(scope, head)
        return self.parse_other(scope, head, i)

    def parse_other(self, scope, head, start):
        tokens, j = scope.tokens, head.keyword
        if head.word == "extern" and token_at(tokens, j + 1) == "crate":
            return item_end(tokens, j)
        if head.word == "{" and j > head.body_start:
            close = matching(tokens, j)
            self.parse_scope(scope._replace(end=close - 1, context="extern"), j + 1)
            return close
        if IDENT.fullmatch(head.word) and token_at(tokens, j + 1) == "!":
            stop = statement_end(tokens, matching(tokens, j + 2), scope.end)
            self.add_item(scope, head, "macro_call", head.word, stop, False)
            return stop
        return max(j, start) + 1

    def parse_use_item(self, scope, head):
        tokens, stop = scope.tokens, head.keyword
        while stop < scope.end and tokens[stop][0] != ";":
            stop += 1
        names = self.module(scope.module)
        for binding, segments in parse_use(tokens[head.keyword + 1:stop]):
            if binding == "*":
                names.globs.append(segments)
            elif binding != "_":
                names.uses[binding] = segments
        return stop + 1

    def parse_mod(self, scope, head):
        tokens, j = scope.tokens, head.keyword
        name = token_at(tokens, j + 1)
        self.mod_decls.append(ModDecl(name, scope.module, head.visibility, scope.path, head.line))
        if token_at(tokens, j + 2) != "{":
            return item_end(tokens, j)
        close = matching(tokens, j + 2)
        self.module_names.add(name)
        inner = scope._replace(end=close - 1, module=scope.module + (name,),
                               context="module", owner=None)
        self.parse_scope(inner, j + 3)
        return close

    def parse_block_item(self, scope, head):
        tokens, j = scope.tokens, head.keyword
        brace = open_brace(tokens, j)
        if brace < 0:
            return item_end(tokens, j)
        name = token_at(tokens, j + 1) if head.word == "trait" else "impl"
        self.add_item(scope, head, head.word, name, brace, scope.context != "module")
        close = matching(tokens, brace)
        inner = scope._replace(end=close - 1, context=head.word,
                               owner=tuple(tokens[head.body_start:brace]))
        self.parse_scope(inner, brace + 1)
        return close

    def parse_macro_rules(self, scope, head):
        tokens, j = scope.tokens, head.keyword
        stop = statement_end(tokens, matching(tokens, j + 3), scope.end)
        self.add_item(scope, head, "macro", token_at(tokens, j + 2), stop, False)
        return stop

    def parse_plain_item(self, scope, head):
        tokens, j = scope.tokens, head.keyword
        stop = item_end(tokens, j)
        name_index = j + 2 if head.word == "static" and token_at(tokens, j + 1) == "mut" else j + 1
        self.add_item(scope, head, head.word, token_at(tokens, name_index), stop,
                      scope.context != "module")
        return stop

    def add_item(self, scope, head, kind, name, stop, trait_member):
        body = head.kept + scope.tokens[head.body_start:stop]
        item = Item(kind, name, scope.owner, head.visibility, body, scope.module, scope.path,
                    head.line, trait_member)
        self.items.append(item)
        if item.owner is None and kind not in ("impl", "macro_call"):
            self.module(scope.module).defs[name].append(item)


def skip_qualifiers(tokens, j, end):
    while j < end:
        t = tokens[j][0]
        const_fn = t == "const" and token_at(tokens, j + 1) in ("fn", "unsafe", "async", "extern")
        if not (t in QUALIFIERS or t.startswith('"') or const_fn):
            break
        j += 1
    return j


def statement_end(tokens, stop, end):
    return stop + 1 if stop < end and tokens[stop][0] == ";" else stop


class UseTree:
    """Expand a use tree into [(binding, segments)]."""

    def __init__(self, tokens):
        self.tokens = tokens
        self.result = []

    def parse(self):
        self.tree(0, [])
        return self.result

    def tree(self, i, segments):
        while i < len(self.tokens):
            t = self.tokens[i][0]
            if t == "::":
                i += 1
            elif t == "{":
                return self.group(i, segments)
            elif t in (",", "}"):
                return i
            else:
                done, i, segments = self.segment(i, t, segments)
                if done:
                    return i
        return i

    def group(self, i, segments):
        close = matching(self.tokens, i)
        j = i + 1
        while j < close - 1:
            j = self.tree(j, list(segments))
            if j < close - 1 and token_at(self.tokens, j) == ",":
                j += 1
        return close

    def segment(self, i, t, segments):
        """Handle one path element; returns (finished, next index, segments)."""
        following = token_at(self.tokens, i + 1, ",")
        if t == "*":
            self.result.append(("*", segments))
            return True, i + 1, segments
        if t == "as":
            self.result.append((following, segments))
            return True, i + 2, segments
        if t == "self" and segments:
            if following == "as":
                self.result.append((token_at(self.tokens, i + 2), segments))
                return True, i + 3, segments
            self.result.append((segments[-1], segments))
            return True, i + 1, segments
        segments = segments + [t]
        if following in (",", "}"):
            self.result.append((t, segments))
            return True, i + 1, segments
        return False, i + 1, segments


def parse_use(tokens):
    return UseTree(tokens).parse()


# --------------------------------------------------------------------------
# Module discovery
# --------------------------------------------------------------------------

def file_modules(files):
    """Map each file to its module by following `mod name;` from src/lib.rs.

    Honors `#[path = "..."]` and Rust's mod-rs rules, so files loaded through a
    path attribute (and their children) get their real module path.
    """
    result = {}
    pending: list[tuple[str, tuple, bool]] = []
    if "src/lib.rs" in files:
        pending.append(("src/lib.rs", (), True))
    while pending:
        path, module, mod_rs = pending.pop()
        if path in result or path not in files:
            continue
        result[path] = module
        for name, attribute_path in declared_modules(lex(files[path])):
            child, child_mod_rs = child_location(files, path, mod_rs, name, attribute_path)
            pending.append((child, module + (name,), child_mod_rs))
    return result


def declared_modules(tokens):
    """Yield (name, path attribute or None) for each top-level `mod name;`."""
    depth = 0
    attribute_path = None
    k = 0
    while k < len(tokens):
        t = tokens[k][0]
        if t == "#" and depth == 0 and token_at(tokens, k + 1) == "[":
            end = matching(tokens, k + 1)
            attribute_path = path_attribute(tokens[k + 2:end - 1]) or attribute_path
            k = end
            continue
        if t == "mod" and depth == 0 and token_at(tokens, k + 2) == ";":
            yield token_at(tokens, k + 1), attribute_path
        depth += {"{": 1, "}": -1}.get(t, 0)
        if t in (";", "{", "}"):
            attribute_path = None
        k += 1


def path_attribute(inner):
    words = [t[0] for t in inner]
    if len(words) == 3 and words[0] == "path" and words[1] == "=":
        return words[2][1:-1]
    return None


def child_location(files, path, mod_rs, name, attribute_path):
    """(file, is mod-rs) for child module `name` declared in `path`."""
    directory = path.rsplit("/", 1)[0]
    if attribute_path:
        return os.path.normpath(directory + "/" + attribute_path).replace(os.sep, "/"), True
    own_dir = directory if mod_rs else path[:-3]
    flat = own_dir + "/" + name + ".rs"
    if flat in files:
        return flat, False
    return own_dir + "/" + name + "/mod.rs", True


def module_path(path):
    if not path.startswith("src/") or not path.endswith(".rs"):
        return None
    relative = path[4:-3]
    if relative == "lib":
        return ()
    if relative == "main" or relative.startswith("bin/"):
        return None
    parts = relative.split("/")
    return tuple(parts[:-1] if parts[-1] == "mod" else parts)


# --------------------------------------------------------------------------
# Name resolution
# --------------------------------------------------------------------------

class Target(NamedTuple):
    """Where a name resolves: kind is item, module, external, unresolved or path."""
    kind: str
    value: tuple
    rest: tuple = ()
    head: Optional["Target"] = None


class Site(NamedTuple):
    position: int
    prefix: tuple
    name: str
    line: int


def path_sites(tokens, names):
    """Paths and bare references, keyed by their normalized token index."""
    sites = {}
    out_index = 0
    i = 0
    while i < len(tokens):
        j = prefix_end(tokens, i, names)
        if j < len(tokens):
            prefix = tuple(tokens[k][0] for k in range(i, j, 2))
            if prefix or is_bare_reference(tokens, i, j):
                sites[out_index] = Site(out_index, prefix, tokens[j][0], tokens[j][1])
        out_index += 1
        i = j + 1
    return sites


def is_bare_reference(tokens, i, j):
    name = tokens[j][0]
    if not IDENT.fullmatch(name) or name in ("Self", "self", "super", "crate"):
        return False
    if token_at(tokens, i - 1) in NOT_REFERENCE_AFTER:
        return False
    return token_at(tokens, j + 1) in ("(", "!", "::", "{") or name[:1].isupper()


class Resolver:
    def __init__(self, crate):
        self.crate = crate

    def lookup(self, module, name, depth=0):
        scope = self.crate.modules.get(module)
        if depth > MAX_DEPTH:
            return Target("unresolved", (name,))
        if scope is None:
            return Target("external", (name,))
        if scope.defs.get(name):
            return identity(scope.defs[name])
        if name in scope.uses:
            return self.resolve(module, scope.uses[name], depth + 1)
        return self.lookup_glob(scope, name, depth)

    def lookup_glob(self, scope, name, depth):
        for glob in scope.globs:
            target = self.resolve(scope.path, glob, depth + 1)
            found = self.lookup(target.value, name, depth + 1) if target.kind == "module" else None
            if found is not None and found.kind not in ("external", "unresolved"):
                return found
        return Target("external", (name,))

    def resolve(self, module, segments, depth=0):
        if depth > MAX_DEPTH or not segments:
            return Target("unresolved", tuple(segments))
        start = self.anchor(module, list(segments), depth)
        if start.target is not None:
            return start.target
        current, rest = start.module, start.rest
        if not rest:
            return Target("module", current)
        for k, segment in enumerate(rest[:-1]):
            step = self.lookup(current, segment, depth + 1)
            if step.kind != "module":
                return Target("path", (), tuple(rest[k + 1:]), step)
            current = step.value
        return self.lookup(current, rest[-1], depth + 1)

    def anchor(self, module, segments, depth):
        """Starting module and remaining segments, or the final target."""
        first = segments[0]
        if first == "crate":
            return Anchor((), segments[1:])
        if first == "self":
            return Anchor(module, segments[1:])
        if first == "super":
            supers = next((k for k, s in enumerate(segments) if s != "super"), len(segments))
            return Anchor(module[:len(module) - supers], segments[supers:])
        head = self.lookup(module, first, depth + 1)
        if len(segments) == 1:
            return Anchor((), [], head)
        if head.kind != "module":
            return Anchor((), [], Target("path", (), tuple(segments[1:]), head))
        return Anchor(head.value, segments[1:])


class Anchor(NamedTuple):
    module: tuple
    rest: list
    target: Optional[Target] = None


def identity(defs):
    items = [d for d in defs if not isinstance(d, tuple)]
    if items:
        return Target("item", tuple(sorted(" ".join(d.text) for d in items)))
    return Target("module", next(d for d in defs if isinstance(d, tuple))[1])


def comparable(target):
    """Modules differ in path across trees; compare them by name only."""
    if target.kind == "module":
        return Target("module", target.value[-1:])
    if target.kind == "path" and target.head is not None:
        return Target("path", (), target.rest, comparable(target.head))
    return target


def describe(target):
    if target.kind == "item":
        text = target.value[0]
        return text[:90] + ("…" if len(text) > 90 else "")
    if target.kind == "module":
        return "module " + "::".join(("crate",) + target.value)
    if target.kind == "path" and target.head is not None:
        return describe(target.head) + " :: " + "::".join(target.rest)
    return "%s %s" % (target.kind, "::".join(target.value))


# --------------------------------------------------------------------------
# Trees
# --------------------------------------------------------------------------

def base_files(base):
    data = subprocess.check_output(["git", "archive", "--format=tar", base, "src"])
    files = {}
    with tarfile.open(fileobj=io.BytesIO(data)) as archive:
        for member in archive.getmembers():
            handle = archive.extractfile(member) if member.name.endswith(".rs") else None
            if handle is not None:
                files[member.name] = handle.read().decode("utf-8")
    return files


def working_files():
    files = {}
    for root, _, names in os.walk("src"):
        for name in (n for n in names if n.endswith(".rs")):
            path = os.path.join(root, name).replace(os.sep, "/")
            with open(path, encoding="utf-8") as handle:
                files[path] = handle.read()
    return files


# --------------------------------------------------------------------------
# Comparison
# --------------------------------------------------------------------------

def in_paths(path, paths):
    return any(path == p or path == p + ".rs" or path.startswith(p + "/")
               for p in (p.rstrip("/") for p in paths))


def item_key(item, names):
    owner = " ".join(normalize(list(item.owner), names)) if item.owner else ""
    return (owner, item.kind, item.name)


def grouped(crate, paths, names):
    groups = defaultdict(list)
    for item in crate.items:
        if in_paths(item.path, paths):
            groups[item_key(item, names)].append(item)
    return groups


def match_items(old_items, new_items):
    """Pair identical items; describe everything left over."""
    pairs: list[tuple[Item, Item]] = []
    differences: list[str] = []
    for key in sorted(set(old_items) | set(new_items)):
        olds = list(old_items.get(key, []))
        news = list(new_items.get(key, []))
        for item in list(news):
            match = next((o for o in olds if o.text == item.text), None)
            if match is not None:
                olds.remove(match)
                news.remove(item)
                pairs.append((match, item))
        differences.extend(body_difference(o, news.pop(0) if news else None) for o in olds)
        differences.extend(body_difference(None, n) for n in news)
    return pairs, differences


def check_paths(pairs, old, new):
    """Resolve every path site on both sides; returns (target changes, prefix rewrites)."""
    checker = PathChecker(old, new)
    for o, n in pairs:
        checker.check(o, n)
    return checker.changes, checker.rewrites


class PathChecker:
    def __init__(self, old, new):
        self.old, self.new = old, new
        self.old_resolver, self.new_resolver = Resolver(old), Resolver(new)
        self.changes = []
        self.rewrites: defaultdict[str, int] = defaultdict(int)

    def check(self, o, n):
        old_sites = path_sites(o.tokens, self.old.module_names)
        new_sites = path_sites(n.tokens, self.new.module_names)
        for index in sorted(set(old_sites) | set(new_sites)):
            self.check_site(o, n, old_sites.get(index), new_sites.get(index))

    def check_site(self, o, n, before, after):
        site = before or after
        if site is None:
            return
        old_prefix = before.prefix if before else ()
        new_prefix = after.prefix if after else ()
        old_target = self.old_resolver.resolve(o.module, list(old_prefix) + [site.name])
        new_target = self.new_resolver.resolve(n.module, list(new_prefix) + [site.name])
        if comparable(old_target) != comparable(new_target):
            if not (old_target.kind == new_target.kind == "external"):
                self.changes.append(target_change(n, after, site, old_prefix, new_prefix,
                                                  old_target, new_target))
        elif old_prefix != new_prefix:
            self.rewrites["%s -> %s" % ("::".join(old_prefix) or "(bare)",
                                        "::".join(new_prefix) or "(bare)")] += 1


def target_change(item, after, site, old_prefix, new_prefix, old_target, new_target):
    return "%s:%d: `%s` -> `%s`\n    base: %s\n    now:  %s" % (
        item.path, after.line if after else item.line,
        "::".join(old_prefix + (site.name,)), "::".join(new_prefix + (site.name,)),
        describe(old_target), describe(new_target))


def scope_of(visibility, module):
    if visibility == "pub":
        return "external"
    if visibility == "pub(crate)":
        return "crate"
    if visibility == "pub(super)":
        return module[:-1]
    if visibility.startswith("pub(in "):
        parts = [p for p in visibility[len("pub(in "):-1].split() if p != "::"]
        return tuple(parts[1:] if parts[:1] == ["crate"] else parts)
    return module


def module_images(pairs, old, new):
    """Old module -> new modules that now hold its code."""
    images = defaultdict(set)
    for o, n in pairs:
        images[o.module].add(n.module)
    for module in set(old.modules) & set(new.modules):
        images[module].add(module)
    return images


def check_item_visibility(pairs, images):
    expansions = []
    for o, n in pairs:
        if o.trait_member or n.trait_member or o.kind in ("impl", "macro_call"):
            continue
        expansion = widened(scope_of(o.visibility, o.module), scope_of(n.visibility, n.module),
                            images)
        if expansion:
            expansions.append("%s:%d: %s %s: %s -> %s (narrowest allowed: %s)" % (
                n.path, n.line, n.kind, n.name, o.visibility, n.visibility, expansion))
        expansions.extend(field_expansions(o, n, images))
    return expansions


def field_expansions(o, n, images):
    for index, (old_field, new_field) in enumerate(zip(o.fields, n.fields)):
        expansion = widened(scope_of(old_field, o.module), scope_of(new_field, n.module), images)
        if expansion:
            yield "%s:%d: field %d of %s: %s -> %s (narrowest allowed: %s)" % (
                n.path, n.line, index, n.name, old_field, new_field, expansion)


def check_mod_visibility(old, new, images, paths):
    origins = defaultdict(set)
    for old_module, new_modules in images.items():
        for new_module in new_modules:
            origins[new_module].add(old_module)
    old_decls = {decl.module + (decl.name,): decl for decl in old.mod_decls}
    expansions = []
    for decl in (d for d in new.mod_decls if in_paths(d.path, paths)):
        candidates = mod_candidates(decl, origins, old_decls)
        new_scope = scope_of(decl.visibility, decl.module)
        results = [(widened(scope_of(c.visibility, c.module), new_scope, images), c)
                   for c in candidates]
        if results and all(result for result, _ in results):
            result, base = results[0]
            expansions.append("%s:%d: mod %s: %s (base `mod %s` was %s) (narrowest allowed: %s)" % (
                decl.path, decl.line, decl.name, decl.visibility, base.name, base.visibility,
                result))
    return expansions


def mod_candidates(decl, origins, old_decls):
    """Base `mod` declarations whose code the new module now holds."""
    declared = decl.module + (decl.name,)
    sources = {old for new_module, olds in origins.items()
               if new_module[:len(declared)] == declared for old in olds}
    if declared in old_decls:
        sources.add(declared)
    candidates = []
    for source in sorted(sources):
        prefixes = [source[:depth] for depth in range(len(source), 0, -1)]
        same_level = [old_decls[p] for p in prefixes if len(p) >= len(declared) and p in old_decls]
        nearest = next((old_decls[p] for p in prefixes if p in old_decls), None)
        candidates.extend(same_level or ([nearest] if nearest else []))
    return candidates


def widened(old_scope, new_scope, images):
    """Narrowest allowed scope when new_scope is wider than allowed, else None."""
    if old_scope == "external" or (old_scope == "crate" and new_scope != "external"):
        return None
    if new_scope in ("external", "crate"):
        return format_scope(old_scope if new_scope == "external" else allowed(old_scope, images))
    limit = allowed(old_scope, images)
    if limit is not None and len(new_scope) < len(limit) and limit[:len(new_scope)] == new_scope:
        return format_scope(limit)
    return None


def allowed(old_scope, images):
    if not isinstance(old_scope, tuple):
        return None
    targets = [m for module, mods in images.items()
               if module[:len(old_scope)] == old_scope for m in mods]
    if not targets:
        return None
    common = os.path.commonprefix([list(t) for t in targets])
    return tuple(common)


def format_scope(scope):
    if not isinstance(scope, tuple):
        return str(scope) if scope else "?"
    return "pub(in %s)" % "::".join(("crate",) + scope)


def describe_item(item):
    owner = " ".join(t[0] for t in item.owner) if item.owner else ""
    return "%s:%d %s %s%s" % (item.path, item.line, item.kind, item.name,
                              (" in `%s`" % owner) if owner else "")


def body_difference(old, new):
    if old is None:
        return "+ %s (no base item)" % describe_item(new)
    if new is None:
        return "- %s (missing now)" % describe_item(old)
    a, b = old.text, new.text
    k = len(os.path.commonprefix([a, b]))
    lo = max(0, k - 8)
    return "~ %s\n    base: %s\n    now:  %s" % (
        describe_item(new), " ".join(a[lo:k + 10]), " ".join(b[lo:k + 10]))


def report(pairs, differences, changes, rewrites, expansions):
    print("compared items: %d matched" % len(pairs))
    sections = (("differences:", differences), ("path target changes:", changes),
                ("path-only changes (prefix rewrites, targets verified):",
                 ["  %s  x%d" % item for item in sorted(rewrites.items())]),
                ("visibility expansions:", sorted(set(expansions))))
    for title, lines in sections:
        if lines:
            print(title)
            print("\n".join(lines))
    failed = bool(differences or changes or expansions)
    print("result: %s" % ("FAIL" if failed else "OK"))
    return 1 if failed else 0


def main():
    if len(sys.argv) < 3:
        raise SystemExit("usage: %s BASE PATH [PATH ...]" % sys.argv[0])
    base, paths = sys.argv[1], sys.argv[2:]
    old, new = Crate(base_files(base)), Crate(working_files())
    names = old.module_names | new.module_names
    old.module_names = new.module_names = names
    for item in old.items + new.items:
        item.text = normalize(item.tokens, names)
    pairs, differences = match_items(grouped(old, paths, names), grouped(new, paths, names))
    changes, rewrites = check_paths(pairs, old, new)
    images = module_images(pairs, old, new)
    expansions = check_item_visibility(pairs, images) + check_mod_visibility(old, new, images, paths)
    return report(pairs, differences, changes, rewrites, expansions)


if __name__ == "__main__":
    raise SystemExit(main())
