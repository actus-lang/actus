# Roadmap: Universal `std::wire` and `std::ustari`

This roadmap implements
[ADR-0051](../decisions/ADR-0051-universal-wire-and-ustari-protocol-libraries.md).
The work is intentionally split between a reusable binary codec and the
universal communication protocol built on top of it.

## Priority prerequisite: ADR-0052 language ergonomics and core correctness

Wire implementation is intentionally deferred until the compiler can express
bounded codec algorithms directly and lower them deterministically. The
following prerequisite gates have priority over every wire and Ustari gate:

- [x] **Gate P0.1: Named compile-time constants** — add typed, immutable,
      compile-time constants with cycle, overflow, and runtime-dependency
      validation; lower them without runtime storage or ABI symbols.
      Evidence: parser spans, semantic duplicate/overflow/cycle/runtime checks,
      formatter round-trip, chained native execution, and full repository gates.
- [x] **Gate P0.2: Statement conditionals and diverging branches** — separate
      statement `if` from expression `if`, classify diverging branches, and
      preserve ownership joins.
      Evidence: dedicated statement AST, semicolon-free parser path, native
      loop-break execution, divergent expression typing, formatter round-trip,
      and LSP nested-binding traversal.
- [x] **Gate P0.3: Case and loop control-flow correctness** — support valid
      `return`, `break`, and `continue` paths in case blocks and make nested
      loop CFG lowering deterministic.
      Evidence: `tests/cli.rs` covers native case-return, case-break, and
      case-continue execution; `tests/arrays_cli.rs` covers nested case/loop
      control after short-circuit evaluation; semantic cleanup tests cover
      nested unwind plans; repeated object tests cover deterministic lowering.
- [x] **Gate P0.4: Type-directed integer ergonomics** — reduce unnecessary
      temporary bindings for typed literals and same-type arithmetic without
      introducing implicit numeric or ownership conversions.
      Evidence: `tests/arrays_cli.rs` covers native direct arithmetic,
      compound assignment, comparison, and `u32` indexing; semantic tests
      cover unsuffixed literal range rejection; parser and lexer suites cover
      typed literal spans and postfix syntax.
- [x] **Gate P0.5: Facade-aware type resolution** — distinguish exported,
      private, missing, and malformed declarations with actionable diagnostics.
      Evidence: `tests/modules/imports.rs` covers public facade exports,
      private imported types, missing imports, and malformed facade sources;
      diagnostics assert stable module/parser codes through the boundary.
- [x] **Gate P0.6: Tooling and regression evidence** — update parser,
      semantic, codegen, formatter, LSP, and native tests for each preceding
      gate and pass the repository quality checks.
      Evidence: fresh CLI/native, semantic, module, and LSP tests are green;
      parser, formatter, codegen, and regression suites remain green. The
      complete repository checks pass: `cargo fmt --all -- --check`,
      `cargo check --all-targets --all-features`, `cargo clippy --all-targets
      --all-features -- -D warnings`, `cargo test --all-targets
      --all-features`, `scripts/check_source_limits.sh`, and `git diff --check`.

No `std::wire` or `std::ustari` implementation gate may be closed while a
priority prerequisite is open. The detailed architecture is recorded in
[ADR-0052](../decisions/ADR-0052-core-control-flow-constants-and-type-directed-ergonomics.md).

## Gate 0: Contract and module boundary

- [x] Confirm the versioned frame constants, byte order, flag policy, maximum
      payload, maximum complete-frame length, and CRC parameters.
- [x] Define the public typed error taxonomy for framing, capacity, integrity,
      version, security, authorization, and protocol failures.
- [x] Define caller-owned storage and ownership transitions for parser,
      serializer, and reassembly operations.
- [x] Define the boundary between `std::wire`, `std::ustari`, and transport
      adapters before implementation begins.
- [x] Add the runtime registry entry for `std::wire` as a `target-neutral`
      module.
- [ ] Add the `std::ustari` runtime registry entry after the wire foundation
      is complete.

## Gate 1: `std::wire` frame model and deterministic codec (after ADR-0052)

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

- The complete wire and Ustari implementation is deferred until all ADR-0052
  priority prerequisite gates are closed. This is an intentional dependency,
  not an abandoned feature.

- Physical transport implementations are deferred until the core codec and
  protocol contracts are complete.
- A specific cryptographic dependency is deferred until its maintenance,
  licensing, auditability, code-size, and target-support review is complete.
- Application-specific command registries are deferred to separate protocol
  profiles and must not expand the universal core implicitly.
