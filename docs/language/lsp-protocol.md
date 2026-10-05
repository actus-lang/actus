# Actus LSP Protocol Contract

`actus lsp` is a compiler-backed Language Server Protocol adapter over stdio.
The compiler remains authoritative for syntax, types, ownership, modules,
targets, visibility, formatting, and diagnostics. A client must treat LSP
responses as versioned compiler observations, not as a second semantic model.

## Lifecycle and transport

Messages use JSON-RPC 2.0 framing with a UTF-8 JSON body and a
`Content-Length` header. The supported lifecycle is `initialize`,
`initialized`, document synchronization, `shutdown`, and `exit`. Requests
receive one response; notifications receive none. Logs never use stdout.

## Response metadata

Compiler-backed responses and structured errors carry an `actus` object with:

- `compilerVersion`;
- `protocolVersion` and `schemaVersion`;
- the selected `target`;
- `resultState`; and
- current workspace/document version data when applicable.

`resultState` is one of `available`, `stale`, `partial`, `unsupported`, or
`invalid`. `partial` is not equivalent to success, and an empty result must
not be interpreted as proof that no symbol or diagnostic exists.

## Compatibility and errors

Older clients may omit optional initialization capabilities. The server keeps
the standard LSP result shape and adds metadata, allowing clients to ignore
Actus-specific fields. Unsupported methods return `-32601`; invalid typed
parameters return `-32602`; canceled work returns the documented cancellation
error and never publishes a stale result. Compatibility fixtures are kept in
`tests/fixtures/lsp/protocol/` and response-contract snapshots in
`tests/fixtures/lsp/snapshots/`.

## Source locations and paths

Source spans are converted from compiler byte offsets to LSP UTF-16 line and
character positions. CRLF and LF documents are handled without counting the
carriage return as source content. File URIs are percent-decoded and encoded
without lossy string normalization; localhost, drive-letter, UNC, Unicode,
space-containing, and non-ASCII paths retain their platform shape.

## Visibility and authority

Only declarations exposed through a module facade are visible to external
queries. Private siblings, raw runtime bridges, backend implementation types,
and compiler-internal fields must not appear in completion, hover, definition,
semantic-token, or semantic-model responses. This boundary is verified by
the module visibility and LSP semantic regression suites.

Compiler-generated scalar locals used to normalize explicit `abs` call
expressions are compiler metadata. They may participate in internal semantic
ownership and cleanup analysis, but their generated names are omitted from
semantic-model bindings and source declarations. Hover, completion, formatter,
and definition results continue to describe the user's source expression.

## Verification

Run the same checks locally and in CI:

```sh
cargo test --test lsp --all-features -- --test-threads=1
cargo test --all-targets --all-features -- --test-threads=1
```

The repository CI matrix executes both suites on Linux, macOS, and Windows.
