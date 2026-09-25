# ADR-0019: Language Server Protocol and Editor Integration

- Status: Accepted
- Date: 2026-09-25
- Scope: `actus lsp` and editor-facing language tooling

## Context

Actus already has the compiler components needed for editor intelligence:
the lexer produces source-aware tokens, the parser produces an AST with spans,
the semantic analyzer validates names, types, roles, ownership, and module
visibility, diagnostics represent failures independently from terminal
rendering, and the formatter owns the canonical source style.

Editor integrations must reuse those components. A second parser, a second
ownership checker, or editor-specific semantic rules would create divergent
behavior between the command-line compiler and development tools.

Actus also needs a clear boundary between semantic language services and
editor syntax rendering. The compiler repository will provide the LSP server.
The incremental syntax grammar and highlighting engine will be maintained in
the separate `tree-sitter-actus` repository.

## Decision

### Server identity and transport

The compiler binary exposes the language server as the `actus lsp`
subcommand. It runs in stdio mode and communicates through JSON-RPC 2.0 using
the Language Server Protocol. The server must not print logs or diagnostics to
stdout because stdout is reserved for protocol frames. Operational logging may
use stderr or a future configurable log sink.

`actus lsp` is a protocol adapter around the existing compiler frontend. It
does not become a second compiler pipeline and does not change language
semantics for editor clients.

### Compiler-core reuse

The server uses the following existing boundaries:

- lexer: tokenization and lexical diagnostics;
- parser: AST construction, source spans, and syntax diagnostics;
- semantic analyzer: symbol resolution, types, imports, roles, ownership, and
  visibility diagnostics;
- diagnostic model: stable codes, severities, spans, and related locations;
- formatter: canonical formatting edits.

The LSP layer converts compiler results into protocol objects. Terminal
diagnostic rendering remains a separate concern and is never called by the
server.

### Document lifecycle and synchronization

The server maintains a document store keyed by document URI. Each document
records its text, version, source encoding information, and the latest parsed
or semantic snapshot.

The core lifecycle methods are:

- `initialize` and `initialized`;
- `shutdown` and `exit`;
- `textDocument/didOpen`;
- `textDocument/didChange`; and
- `textDocument/didClose`.

The server accepts both full-document and incremental change events. It may
start with full synchronization while the document store is stabilized, but
the protocol capability must be explicit. Incremental application must reject
invalid ranges deterministically rather than silently corrupting source text.

After every accepted open or change, the server re-runs the available
frontend stages and publishes the current diagnostics. Closing a document
removes its unsaved overlay and clears diagnostics associated with that URI.

### Diagnostics

Compiler errors and warnings are published with `textDocument/publishDiagnostics`.
This includes:

- lexical and parser errors;
- unresolved names and imports;
- type and generic constraint errors;
- `erg`, `abs`, and `dat` role violations;
- use-after-move, invalid borrow, frozen-owner, and cleanup errors; and
- module visibility and duplicate declaration errors.

Diagnostics retain stable Actus `E####` codes where available. Related
locations are used for cross-file declarations, duplicate symbols, and module
facade errors. A partially invalid document must still produce diagnostics
from recoverable compiler stages; one syntax error must not terminate the LSP
session.

### Navigation and information

The accepted capability roadmap includes:

- `textDocument/definition` for local symbols and cross-file module symbols,
  including facade re-exports such as `open ops;`;
- `textDocument/hover` for resolved types, `erg`/`abs`/`dat` roles, generic
  applications, role bounds, and available documentation comments; and
- document and workspace symbol support when the symbol index is available.

Definition lookup must use the same ModuleResolver and facade filtering as a
normal build. An editor must not see a private sibling declaration merely
because the source file is open in the workspace.

### Formatting

`textDocument/formatting` delegates to the canonical Actus formatter. The
server returns text edits and never introduces per-editor formatting options.
The formatter remains no-config and deterministic. Formatting a document must
not modify ownership, semantic state, or the document store outside the text
edit response.

### Advanced capabilities

The following capabilities are explicitly deferred until the core server is
stable:

- semantic tokens for role-aware highlighting;
- completion;
- code actions and quick fixes; and
- inlay hints for inferred types and ownership information.

Syntax highlighting, folding, and editor indentation are not LSP semantic
responsibilities. They belong to `tree-sitter-actus` and its editor adapters.

## Position Adapter

Actus source spans are byte offsets. LSP ranges use line and character
positions, with UTF-16 character units required by the protocol's default
encoding. The server therefore owns one source-position adapter that:

1. indexes line starts from the current document text;
2. converts byte offsets to UTF-8 scalar and UTF-16 character positions;
3. converts LSP positions back to validated byte offsets for edits; and
4. rejects offsets that split a UTF-8 code point.

All diagnostics, definition locations, hover ranges, and formatting edits use
this adapter. Position conversion must be tested with ASCII, multibyte UTF-8,
and characters whose UTF-16 width is two code units.

## Implementation Gates

### Gate 1: Lifecycle, framing, and live diagnostics

Scope:

- add the `actus lsp` CLI hook;
- implement stdio JSON-RPC framing and request dispatch;
- implement the document store and full-document synchronization;
- run lexer, parser, and semantic analysis on open/change; and
- publish deterministic diagnostics.

Invariant: an editor document is analyzed from exactly the text held by the
server, and every accepted change produces a current diagnostic snapshot.

### Gate 2: Definition and module resolution

Scope:

- implement `textDocument/definition`;
- reuse Directory Module Resolver and Arca source roots;
- resolve facade exports and cross-file sibling declarations; and
- return source locations through the position adapter.

Invariant: language-server visibility and definition results are identical to
compiler visibility and never expose closed sibling declarations.

### Gate 3: Hover and canonical formatting

Scope:

- implement `textDocument/hover` for types, roles, bounds, and doc comments;
- implement `textDocument/formatting`; and
- verify edits and hover ranges across UTF-8/UTF-16 boundaries.

Invariant: editor information is derived from the same semantic snapshot as
diagnostics, and formatting is byte-stable with the canonical CLI formatter.

## Consequences

- Actus gets one authoritative semantic source for compiler and editor tools.
- Neovim and VS Code can share the same stdio LSP implementation.
- `tree-sitter-actus` can evolve independently without backend or semantic
  coupling to the compiler repository.
- Incremental parsing and advanced editor features remain possible without
  prematurely changing the compiler phase boundaries.
- The server must carefully manage unsaved overlays, source positions, and
  error recovery as the implementation matures.

## Non-Goals

This decision does not add a language keyword, runtime feature, or alternate
syntax grammar. It does not move `tree-sitter-actus` into the Actus compiler
repository, and it does not make the LSP server responsible for code
generation, linking, or package publication.
