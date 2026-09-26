# Phase 15.5: Language Server and Editor Tooling

This phase turns the existing Actus frontend into one authoritative editor
service. The implementation follows
[ADR-0019](../decisions/ADR-0019-language-server-protocol-and-editor-integration.md)
and keeps syntax rendering in the separate `tree-sitter-actus` repository.

## Goals

- [x] Expose `actus lsp` through the main compiler binary.
- [x] Serve LSP over stdio JSON-RPC without contaminating stdout with logs.
- [x] Reuse lexer, parser, semantic analyzer, diagnostics, module resolver,
  and canonical formatter instead of creating parallel implementations.
- [x] Support unsaved documents through a versioned in-memory document store.
- [x] Keep editor visibility identical to compiler visibility, including
  facade exports and closed siblings.

## Invariants

- [x] Every diagnostic is derived from the current document snapshot.
- [x] The LSP server never implements ownership, type, or visibility rules of
  its own.
- [x] JSON-RPC framing is deterministic and stdout contains protocol messages
  only.
- [x] Byte spans are converted to valid UTF-8 and UTF-16 LSP positions.
- [x] A malformed document cannot terminate the server session.
- [x] Formatting delegates to the canonical no-config formatter.

## Gate 1: Lifecycle and Diagnostics

- [x] Add the `actus lsp` CLI subcommand.
- [x] Implement `Content-Length` JSON-RPC framing over stdin/stdout.
- [x] Implement request/response correlation and notification handling.
- [x] Handle `initialize`, `initialized`, `shutdown`, and `exit`.
- [x] Implement `textDocument/didOpen` and `textDocument/didClose`.
- [x] Implement full-document `textDocument/didChange` synchronization.
- [x] Store URI, version, text, and the latest frontend snapshot.
- [x] Run lexer, parser, and semantic analysis after accepted changes.
- [x] Publish lexical, parser, semantic, ownership, and module diagnostics.
- [x] Preserve stable `E####` diagnostic codes and related locations.
- [x] Recover from invalid JSON-RPC messages and malformed source without
  exiting.

## Gate 2: Navigation and Modules

- [x] Implement `textDocument/definition`.
- [x] Reuse Actus manifest/source-root discovery.
- [x] Reuse Directory Module Resolver and facade export filtering.
- [x] Resolve local symbols, sibling symbols, and cross-file imports.
- [x] Reject definition results for private or unlisted declarations.
- [x] Convert definition spans through the shared position adapter.

## Gate 3: Hover and Formatting

- [x] Implement `textDocument/hover` for resolved types and generic
  applications.
- [x] Show `erg`, `abs`, and `dat` role information in hover results.
- [x] Show role bounds and available documentation comments.
- [x] Implement `textDocument/formatting` using the canonical formatter.
- [x] Return deterministic text edits without editor-specific style options.
- [x] Verify hover ranges and formatting edits with multibyte UTF-8 text.

## Position Adapter

- [x] Index line starts for every document snapshot.
- [x] Convert Actus byte offsets to LSP line/character positions.
- [x] Support UTF-8 scalar boundaries and UTF-16 code-unit positions.
- [x] Convert LSP edits back to validated byte ranges.
- [x] Reject ranges that split a UTF-8 code point.

## Deferred Capabilities

- [ ] Semantic tokens.
- [ ] Completion.
- [ ] Code actions and quick fixes.
- [ ] Inlay hints.
- [ ] Incremental parser optimization after full synchronization is stable.

## Quality Gates

Every gate must pass:

- `cargo fmt --all -- --check`;
- `cargo check --all-targets --all-features`;
- `cargo clippy --all-targets --all-features -- -D warnings`;
- `cargo test --all-targets --all-features`;
- `scripts/check_source_limits.sh`; and
- `git diff --check`.

LSP-specific verification must also cover valid JSON-RPC framing, request
correlation, lifecycle shutdown, full document replacement, diagnostic
publication, malformed input recovery, UTF-8/UTF-16 conversion, and stdout
protocol purity.

## Completion Criteria

Phase 15.5 is complete when all three gates are green, a minimal Neovim or VS
Code client can open an Actus file and receive live diagnostics, definitions,
hover information, and canonical formatting edits, and the separate
`tree-sitter-actus` repository owns syntax highlighting and folding.
