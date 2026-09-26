# ADR-0025: First-Class Block Docstrings

- Status: Proposed
- Date: 2026-09-26
- Decision owners: Actus language and compiler maintainers

## Context

Actus documentation is currently represented by line-oriented `///` comments.
That form is adequate for short notes, but it makes multi-paragraph API
documentation cumbersome and gives the lexer no structured representation of
documentation attached to a declaration. The standard library also needs to
carry Markdown-shaped explanations of ownership roles, return contracts, and
runtime boundaries without repeating a comment marker on every line.

Documentation is source-level information. It should remain visible to the
lexer and parser, be associated with the declaration it documents, and retain
the author's paragraph and code-block layout after normalization.

## Decision

Actus adopts a first-class block docstring delimited by `"""` and `"""`.
The canonical form is a docstring immediately before a `verb`, `struct`, or
`role` declaration, including an `open` declaration. The parser stores that
text in the declaration's `doc` field as `Option<String>`.

```actus
"""
Read bytes into a caller-owned buffer.

The `ins` role provides an exclusive, non-escaping loan.
"""
open verb read(ins buffer: Buffer) -> Result[Int, IoError] {
    return read_line(buffer: ins buffer);
}
```

The same association applies to declarations introduced by `open struct` and
`open role`. A docstring may also appear inside a declaration body as source
documentation for a field, variant, or statement. Those nested strings are
lexically recognized and consumed by the parser; declaration-level AST fields
are the initial public documentation API.

The legacy `///` spelling remains a line comment during this migration. It is
not a declaration documentation representation and new public Actus APIs must
use block docstrings.

## Lexical contract

The lexer emits `TokenKind::DocString(String)` for a complete triple-quoted
literal. Ordinary quoted strings remain `TokenKind::StringLiteral(String)`.
The docstring token span covers both delimiters and uses the original UTF-8
byte offsets. An unterminated block produces a dedicated lexical diagnostic.

Docstrings may contain newlines, Markdown punctuation, and ordinary quote
characters. Only three consecutive double quotes terminate the token. The
lexer normalizes CRLF to LF, removes the leading and trailing delimiter
newlines, and strips the smallest common indentation from non-empty lines.
Internal blank lines and relative indentation are preserved.

## AST and parser contract

The AST adds `doc: Option<String>` to `VerbDecl`, `StructDef`, and `RoleDecl`.
The parser consumes a docstring before the declaration and stores the
normalized text without delimiters. The association is lexical: the next
supported declaration owns the documentation, and the text cannot drift to a
later declaration.

Docstrings do not change ownership, type, role, dispatch, or code generation
semantics. They are metadata for tooling and future documentation extraction.

## Diagnostics and tooling

An unterminated docstring is reported as a lexical error with a stable source
span. The existing diagnostic pipeline renders it independently of terminal
output. LSP and formatter integrations can later use the AST field without
re-parsing comments or reconstructing paragraph boundaries.

## Migration policy

The standard library migrates its `///` documentation to `"""` blocks in the
same change as compiler support. Existing line comments remain valid so that
older source files continue to parse. New documentation examples, public
standard-library declarations, and future manuscript code use the block form.

## Consequences

The lexer and parser gain a small, explicit documentation path, and AST
consumers must preserve the new optional field when cloning declarations.
The representation supports Markdown paragraphs without runtime cost. Nested
documentation is currently parser-consumed metadata rather than a fully
addressable field-level AST API; that can be extended independently if tools
need field or statement documentation later.

## Implementation gates

- [x] Define the block-delimited lexical token and normalization rules.
- [x] Attach normalized text to verb, struct, and role declarations.
- [x] Preserve legacy line comments during migration.
- [x] Migrate `library/std/src/io/` documentation.
- [ ] Expose declaration documentation through LSP hover and generated API docs.
