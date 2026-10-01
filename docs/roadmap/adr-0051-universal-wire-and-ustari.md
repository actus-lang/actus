# Roadmap: Universal `std::wire` and `std::ustari`

This roadmap implements
[ADR-0051](../decisions/ADR-0051-universal-wire-and-ustari-protocol-libraries.md).
The work is intentionally split between a reusable binary codec and the
universal communication protocol built on top of it.

## Gate 0: Contract and module boundary

- [ ] Confirm the versioned frame constants, byte order, flag policy, maximum
      payload, maximum complete-frame length, and CRC parameters.
- [x] Define the public typed error taxonomy for framing, capacity, integrity,
      version, security, authorization, and protocol failures.
- [x] Define caller-owned storage and ownership transitions for parser,
      serializer, and reassembly operations.
- [x] Define the boundary between `std::wire`, `std::ustari`, and transport
      adapters before implementation begins.
- [ ] Add the runtime registry entries as `target-neutral` modules.

## Gate 1: `std::wire` frame model and deterministic codec

- [ ] Add the canonical `wire` facade and responsibility-specific siblings.
- [ ] Implement fixed-width little-endian field encoding and decoding.
- [ ] Implement frame construction with explicit header and payload lengths.
- [ ] Implement CRC16-CCITT with named protocol constants.
- [ ] Reject reserved flags, unknown versions, impossible lengths, and
      insufficient output capacity before accessing payload storage.
- [ ] Prove deterministic serialization with repeated byte-for-byte builds.

## Gate 2: Incremental parser and bounded failure behavior

- [ ] Implement an incremental state machine that accepts arbitrary input
      chunks and reports consumed bytes.
- [ ] Support zero-copy payload views for complete caller-owned frames.
- [ ] Define resynchronization after invalid magic, version, length, flags, or
      CRC without unbounded scanning.
- [ ] Reject truncation and incomplete frames without treating partial input as
      a valid message.
- [ ] Add malformed-input and arbitrary-byte tests that prove no panic,
      out-of-bounds access, or unbounded loop.

## Gate 3: Fragmentation and reassembly

- [ ] Define versioned fragmentation metadata outside the canonical frame
      header.
- [ ] Bound fragment count, fragment size, offset arithmetic, reassembly
      storage, simultaneous reassemblies, and reassembly lifetime.
- [ ] Reject duplicate, missing, overlapping, stale, oversized, and
      out-of-order fragments according to the selected profile.
- [ ] Add cancellation and resource-exhaustion behavior with typed errors.
- [ ] Prove that reassembly returns caller-owned storage without hidden
      allocation.

## Gate 4: `std::ustari` message and session protocol

- [ ] Add the canonical `ustari` facade and typed message metadata.
- [ ] Define versioned message identifiers, channels, schemas, maximum sizes,
      and response contracts.
- [ ] Define capability negotiation and unknown-message behavior.
- [ ] Define ACK/NACK, duplicate, retry, timeout, and idempotency semantics.
- [ ] Require explicit session state before protected operations are accepted.
- [ ] Keep application-specific message families outside the protocol core.

## Gate 5: Replay, authorization, and security facade

- [ ] Implement bounded directional sequence handling and replay-window state.
- [ ] Reject duplicates, stale values, unreasonable forward jumps, and
      sequence wrap without rekeying or session replacement.
- [ ] Define authentication, authorization context, and security-state errors.
- [ ] Add a narrow crypto facade with an explicit audited implementation
      boundary; do not implement cryptographic primitives in Actus.
- [ ] Prove that protected headers and payload authentication metadata are
      versioned and cannot silently downgrade to unauthenticated control.

## Gate 6: Runtime, LSP, formatter, and standard-library integration

- [ ] Register both modules in the compiler-owned runtime manifest as
      target-neutral surfaces.
- [ ] Verify `runtime = "std"` resolves the modules on hosted and freestanding
      targets without acquiring host-only services.
- [ ] Verify reachability: unused protocol modules are not compiled or linked.
- [ ] Add formatter, semantic-model, hover, completion, and diagnostics
      support for public declarations and protocol types.
- [ ] Keep all module facades and source files within repository size limits.

## Gate 7: Acceptance and transport-neutral evidence

- [ ] Run host round-trip, negative, property-style, and deterministic codec
      tests.
- [ ] Run native execution tests for serialization, parsing, CRC, replay, and
      typed failure paths.
- [ ] Run freestanding target-object tests proving no hosted runtime imports.
- [ ] Verify transport adapters can feed arbitrary chunks without changing
      wire semantics.
- [ ] Document that transport integration requires separate MTU, timeout,
      retry, and resource evidence.
- [ ] Run formatting, compilation, Clippy, full tests, source limits, and
      diff validation before closing the roadmap.

## Explicit deferrals

- Physical transport implementations are deferred until the core codec and
  protocol contracts are complete.
- A specific cryptographic dependency is deferred until its maintenance,
  licensing, auditability, code-size, and target-support review is complete.
- Application-specific command registries are deferred to separate protocol
  profiles and must not expand the universal core implicitly.
