# ADR-0048: LSP Execution, Cancellation, Progress, and Bounded Responses

- Status: Proposed
- Date: 2026-09-29
- Scope: Actus LSP transport, request execution, cancellation, progress, and
  response resource limits

## Context

The Actus LSP currently processes framed JSON-RPC messages through one
synchronous loop. That is sufficient for small requests, but it cannot provide
production-grade cancellation while a compiler-backed query is running. A
client cancellation notification must be received independently of the query,
and the query must observe cancellation at explicit compiler boundaries.

An editor also needs deterministic progress and bounded responses. Unbounded
workspace traversal, completion payloads, diagnostics, or hover documentation
can freeze a client even when the compiler itself remains correct. Transport,
session state, compiler queries, and rendering must therefore have explicit
ownership boundaries.

## Decision

The LSP will use four cooperating layers:

1. **Framed transport reader** — reads and validates `Content-Length` frames,
   preserves message order, and forwards requests to the session executor.
2. **Session state machine** — owns initialization, target selection, open
   document overlays, module context, and shutdown transitions.
3. **Request executor** — runs compiler-backed queries against a stable
   semantic snapshot, attaches request metadata, emits progress, and commits
   only valid current results.
4. **Protocol renderer** — serializes standard LSP results and Actus metadata;
   it never performs parsing, semantic analysis, ownership inference, or target
   selection.

The mutable session state remains serialized. Read-only queries may use a
versioned snapshot, but a result is publishable only when its document and
workspace versions still match the request snapshot. Notifications that mutate
overlays are applied in order before later snapshots are created.

## Request identity and cancellation

- Every request with an ID receives exactly one response or one standard
  request-canceled error.
- Notifications never receive responses.
- `$/cancelRequest` is handled by the transport/session boundary and records a
  cancellation token keyed by the original request ID.
- Compiler-backed queries receive a cooperative cancellation token and must
  check it before parsing, after module aggregation, before semantic analysis,
  before code generation, before serialization, and before publishing output.
- Cancellation is cooperative. The server must never kill a Rust thread or
  interrupt unsafe runtime state.
- A canceled request returns JSON-RPC error `-32800` and must not publish a
  stale success result or progress completion that claims success.
- Unknown cancellation IDs are harmless and deterministic.

## Progress lifecycle

Long-running requests may publish `$/progress` events using a request-scoped
token:

1. `begin` identifies the operation and whether it is cancellable;
2. `report` describes a bounded phase and optional percentage;
3. `end` reports `completed`, `canceled`, or `failed`.

Progress is advisory. A client that does not advertise progress support must
still receive the same compiler result and diagnostics. Progress messages must
not contain source text, private declarations, or unbounded diagnostic data.

## Bounded response policy

Every compiler-backed request has explicit limits for:

- workspace traversal and module count;
- diagnostics, completion items, references, and symbol results;
- hover/documentation bytes;
- serialized response bytes;
- analysis time and cancellation polling intervals.

When a limit is reached, the response is marked `partial` and includes a
machine-readable reason. It must never be presented as a complete semantic
answer. Unsupported, stale, invalid, and unavailable states remain distinct.

## Versioned metadata

Every response and structured error carries an Actus metadata object containing:

- protocol version;
- schema version;
- compiler version;
- selected target profile;
- current document URI and version where applicable.

The standard LSP `result` shape remains unchanged. Metadata is an additive
top-level extension field so existing LSP clients can ignore it safely.

## Lifecycle and shutdown

- `initialize` is accepted once and negotiates compatible protocol/schema and
  optional capabilities.
- `initialized` is valid only after successful initialization.
- `shutdown` transitions the session to `ShuttingDown`, rejects new work,
  cancels pending queries, drains deterministic cleanup, and returns `null`.
- `exit` terminates the transport after shutdown handling; no response is
  emitted for the notification form.
- Reader, executor, and progress resources must be closed deterministically;
  no worker may write after the output stream is closed.

## Error and security contracts

Invalid params, unknown methods, invalid lifecycle transitions, stale results,
unsupported capabilities, cancellation, and bounded-result truncation use
stable machine-readable codes or state labels. Raw panics, temporary paths,
private compiler declarations, and backend-specific implementation details must
not be exposed through the protocol.

The server never executes client-provided commands, loads arbitrary provider
paths, or mutates source/artifacts/devices as a side effect of inspection.

## Verification

- Protocol fixtures cover valid, invalid, incompatible, canceled, stale,
  partial, and shutdown exchanges.
- Integration tests prove request correlation, notification silence,
  cancellation before and during each expensive phase, and no stale publish.
- Stress tests cover bounded completion, references, diagnostics, hover data,
  workspace traversal, and concurrent document changes.
- Cross-platform tests cover framing, UTF-8/UTF-16 positions, URI normalization,
  target metadata, and deterministic shutdown on Linux, macOS, and Windows.
- Tests verify that every response has metadata and that standard LSP result
  payloads remain schema-compatible.

## Non-goals

This ADR does not define compiler semantic rules, ownership states, target
selection policy, DAP behavior, VS Code rendering, or hardware actions. It
does not promise hard real-time scheduling; it defines bounded, cooperative,
observable request execution for the LSP process.
