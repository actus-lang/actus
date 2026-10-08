# Phase 39 — Actus `std::wire` Standard Library

## Objective

Implement the universal, target-neutral `std::wire` communication library in
Actus. It must carry bounded binary information for many consumers without
depending on a particular application, operating system, board, or transport.

This phase supersedes the former `std::ustari` split. The protocol name is
`wire`; the public library name is `std::wire`.

## Non-goals

- [ ] Implement encryption in the initial phase.
- [ ] Implement physical transports in the core library.
- [ ] Add application-specific payloads or downstream project terminology.
- [ ] Add filesystem access, hidden allocation, raw OS pointers, or an
      unbounded buffer.
- [ ] Claim hardware transport evidence from host codec tests.

## Gate 39.0 — Contract and migration baseline

- [x] Mark the former Ustari split as superseded by ADR-0078.
- [x] Confirm `std::wire` as the only public protocol namespace.
- [x] Confirm the 12-byte header, byte order, payload bound, frame bound, and
      CRC16-CCITT parameters.
- [x] Define version 1 flags and reject unknown bits.
- [x] Define generic channels and message metadata without application schemas.
- [x] Define `erg`, `abs`, `dat`, and `ins` ownership for every public API.
- [x] Define the typed error taxonomy and failure behavior.
- [x] Record the compiler and standard-library profile used as evidence.

### Gate 39.0 evidence

- ADR: [ADR-0078](../decisions/ADR-0078-std-wire-protocol.md) supersedes the
  former `std::ustari` split and defines the target-neutral contract.
- Public namespace evidence: `library/std/src/lib.act` exports only `wire` for
  this protocol; the active standard-library manifest registers only `wire`.
- Compiler profile: hosted and freestanding standard-library resolution was
  validated with the installed Actus executable at
  `/home/magradze/.cargo/bin/actus`.
- Compiler artifact digest: `14181544766f3b8af67bc48b73170a339ff857cadcb2a1ff9c75bc6732d7fdb2`.
- The installed compiler does not expose a version flag; the executable digest
  is recorded so this evidence remains reproducible for this checkout.

## Gate 39.1 — Canonical module and public facade

- [x] Add the canonical `library/std/src/wire/wire.act` facade.
- [x] Add responsibility-specific siblings for the implemented frame, codec,
      checksum, and error surfaces.
- [x] Export only intentional public declarations through the facade.
- [x] Add positive and negative facade-import tests.
- [x] Keep every source file below the Actus hard limit and document all public
      types, constants, enums, and verbs.

Parser, sequence, and fragmentation siblings are intentionally deferred to
Gates 39.4, 39.5, and 39.6. They will be added with their implementations;
this gate does not create empty placeholder modules.

## Gate 39.2 — Frame model and deterministic serialization

- [x] Implement named protocol constants for magic, version, lengths, flags,
      payload bound, and CRC parameters.
- [x] Implement fixed-width little-endian header encoding and decoding.
- [x] Validate payload length before payload access.
- [x] Serialize valid frames into caller-owned bounded buffers.
- [x] Reject insufficient output capacity with a typed error.
- [x] Prove byte-for-byte deterministic serialization across repeated builds.
- [x] Add empty, minimum, maximum, and over-sized payload tests.

Gate 39.2 evidence is provided by `tests/std_wire_native.rs`: the hosted
native fixture encodes the same header and payload twice, verifies identical
frame bytes, accepts payload lengths `0`, `1`, and `1024`, and rejects length
`1025` with `WireError.PayloadTooLarge`.

## Gate 39.3 — CRC and typed failure paths

- [x] Implement CRC16-CCITT with the ADR-0078 parameters.
- [x] Reject bad magic, version, reserved flags, impossible lengths, and CRC.
- [x] Distinguish framing, capacity, integrity, version, and policy failures.
- [x] Prove rejected frames do not mutate caller-owned protocol state.
- [x] Add native execution tests for successful and failed CRC paths.

Gate 39.3 evidence is provided by `tests/std_wire_native.rs`: native execution
checks typed failures for invalid magic, unsupported version, unknown flags,
oversized payload declarations, truncated headers, insufficient capacity, and
CRC mismatch. The CRC failure path initializes a caller-owned payload buffer
with sentinel bytes and verifies that rejection leaves it unchanged.

## Gate 39.4 — Incremental bounded parser

- [x] Accept arbitrary input chunks; never assume one read equals one frame.
- [x] Track parser state using caller-owned bounded storage.
- [x] Report consumed bytes and incomplete-frame status explicitly.
- [x] Resynchronize after malformed magic, version, length, flags, or CRC.
- [x] Bound scanning, buffering, and recovery work.
- [x] Reject truncated input without dispatching a partial message.
- [x] Add arbitrary-byte tests with no panic, out-of-bounds access, or
      unbounded loop.

Gate 39.4 implementation evidence covers the bounded parser state in
`library/std/src/wire/parser.act`, its public facade export, and native tests
that feed one valid frame through two independent chunks. The tests verify the
intermediate `Collecting` state, consumed-byte counts, final `Ready` state,
decoded header, and payload. The same fixture verifies that incomplete input
leaves sentinel output unchanged, arbitrary noise is consumed without leaving
the searching state, a malformed magic prefix is skipped before a valid frame,
and unsupported version, oversized length, unknown flags, and CRC failures
return typed errors without dispatching invalid data. Native lowering also now
traverses `else if` branches when inlining compile-time constants; this is
covered by the compiler regression test
`native_lowering_inlines_facade_constant_in_else_if_branch`.

## Gate 39.5 — Sequence and duplicate protection

- [x] Implement a bounded sliding sequence window.
- [x] Reject duplicate and stale frames deterministically.
- [x] Define forward advancement and unreasonable jump behavior.
- [x] Define sequence reset and context replacement without silent reuse.
- [x] Test first value, advancement, duplicate, stale value, jump, and wrap.
- [x] Document that this is replay suppression, not authentication.

Gate 39.5 evidence is provided by `library/std/src/wire/sequence.act`, which
keeps one 64-bit receipt mask and bounded scalar context state. The hosted
native fixture in `tests/std_wire_native.rs` covers first acceptance, forward
advancement, bounded out-of-order acceptance, duplicate rejection, stale
rejection, unreasonable jumps, context mismatch, explicit replacement, reset,
and the u32 wrap boundary. The public facade and typed-result surface are
covered by `tests/std_wire_semantic.rs` and `tests/stdlib_result_surface.rs`.
ADR-0078 defines the policy as replay suppression only; it does not provide
authentication or authorization.

## Gate 39.6 — Fragmentation and reassembly primitives

- [x] Define versioned fragmentation metadata outside the canonical header.
- [x] Bound fragment count, size, offsets, storage, and lifetime.
- [x] Reject duplicate, missing, overlapping, stale, and over-sized fragments.
- [x] Define cancellation and resource-exhaustion errors.
- [x] Prove reassembly uses caller-owned storage without hidden allocation.

Gate 39.6 implementation evidence is provided by the public facade at
`library/std/src/wire/fragment/fragment.act` and its codec and reassembly
modules.
The version-one metadata prefix is 17 bytes, carries a generation, and is
validated before any storage write. Reassembly accepts at most 64 fragments
into caller-provided storage capped at 4096 bytes, records each accepted byte
range, and rejects duplicates, overlap, stale generations, inconsistent
message extents, and insufficient capacity. Completion and cancellation are
explicit lifecycle states; the implementation performs no allocation or I/O.

## Gate 39.7 — Optional hosted `wire://` endpoint contract

`wire://` is an optional desktop-facing addressing layer. It is not required
for two participants to exchange Wire frames, and the core library must remain
usable on microcontrollers without URI strings. Direct typed APIs and physical
transport adapters may use the core frame types without this endpoint module.

- [x] Define the grammar for `wire://` endpoint addresses.
- [x] Parse scheme, authority, port, and path without OS or transport calls.
- [x] Reject malformed, ambiguous, or over-sized addresses with typed errors.
- [x] Keep endpoint parsing separate from frame parsing and transport adapters;
      the endpoint layer may depend on core Wire types, never the reverse.
- [x] Keep the endpoint layer optional for hosted desktop clients and absent
      from embedded builds that use numeric channels and caller-owned buffers.
- [x] Add formatter and documentation evidence for the endpoint API.

Gate 39.7 evidence is provided by `library/std/src/wire/endpoint.act` and the
hosted native fixture `hosted_wire_endpoint_parser_is_optional_and_bounded`.
The parser uses a caller-owned byte buffer, returns fixed-layout offsets and
lengths, performs no allocation or transport call, and rejects missing
authority and malformed bounded input with typed errors.

## Gate 39.8 — Standard-library and compiler integration

- [x] Register `std::wire` as a target-neutral standard-library module.
- [x] Verify hosted and freestanding resolution without host-only imports.
- [x] Verify unused Wire modules are not compiled or linked unnecessarily.
- [x] Add formatter, semantic-model, hover, completion, and diagnostics
      evidence for public declarations.
- [x] Add native object evidence for the bounded runtime surface.

Gate 39.8 compiler evidence is provided by the native reachability fixture in
`tests/std_wire_native.rs`. An entry that imports `std::wire` but uses only
`WIRE_VERSION` defines no Wire verb bodies. An entry that calls
`wire_crc16_ccitt` emits only the CRC verb and its private update helper; the
parser, sequence, fragmentation, codec, and endpoint verbs are absent from the
native object set. This behavior is implemented in the compiler's native
declaration reachability pass and does not depend on linker garbage collection.

Editor integration evidence is provided by the standard formatter suite, the
Wire-specific formatter idempotency test, the existing semantic-model public
declaration tests, and `lsp_exposes_the_public_wire_surface_to_editor_clients`.
The LSP fixture verifies clean diagnostics plus hover, completion, definition,
and formatting responses for the public CRC surface.

## Gate 39.9 — Acceptance and future security boundary

- [x] Run host round-trip and negative codec tests.
- [x] Run native serialization, parsing, CRC, sequence, and failure tests.
- [x] Run freestanding object checks with no hosted runtime dependency.
- [x] Record transport-neutral evidence separately from physical transport
      evidence.
- [x] Document encryption as a future profile behind an audited external crypto
      facade and explicit key/session contract.
- [x] Run the repository formatting, compilation, Clippy, test, source-limit,
      and diff checks.
- [x] Close the phase only when every implementation gate has tests and
      reproducible evidence.

## Gate 39.10 — Generated serialization native dependency closure

This gate is part of Phase 39 because the compiler integration changes used by
the Wire object-emission checks must preserve the existing generated
serialization contract. The failure is in compiler native lowering and object
dependency discovery, not in the Wire protocol implementation.

- [x] Reproduce the unresolved `frame_encode`, `frame_decode`,
      `frame_migrate`, and `frame_validate` diagnostics in isolated fixtures.
- [x] Trace each generated serialization verb from its source declaration
      through semantic analysis, object planning, native lowering, and link
      dependency collection.
- [x] Make generated serialization verbs and their private helpers available
      to native lowering through the canonical dependency graph, without
      restoring unrelated unreachable Wire verb bodies.
- [x] Preserve external runtime bridge declarations required by generated
      serialization while retaining deterministic reachability for ordinary
      Actus verbs.
- [x] Add object and executable parity tests for generated encode, decode,
      migration, and validation paths.
- [x] Add negative tests for short input, invalid version, invalid checksum,
      and migration rejection after the dependency fix.
- [x] Prove the fix does not reintroduce unused `std::wire` symbols or hosted
      runtime imports in the freestanding Wire object.
- [x] Rerun the complete repository test suite and record the compiler,
      target, command, and result in the Phase 39 evidence document.
- [x] Mark Gate 39.9's repository-wide checks and Phase 39 completion only
      after Gate 39.10 and every earlier gate are green.

### Gate 39.10 evidence

The failing generated serialization fixtures were reproduced with the native
array CLI tests. The compiler was collecting native roots from the raw source
program before generated serialization verbs were inserted by normalization.
As a result, generated encode, decode, migration, and validation helpers were
available to semantic checking but absent from the native dependency closure.

`src/cli/build/emission.rs` now normalizes the root program before calling
`reachable_call_names`. This includes compiler-generated serialization helpers
in native lowering while preserving ordinary reachability pruning. The
generated serialization object and executable tests now pass, including their
negative input, version, checksum, and migration cases.

The Wire implementation was also split into codec and reassembly modules under
the canonical fragment facade. This keeps each implementation unit within the
standard-library source limits without changing the public `std::wire` names or
the bounded ownership contract. The local standard-library lock checksum was
refreshed after this source change.

Evidence is recorded in
`docs/benchmarks/phase-39-wire-2026-10-08.md`. The full repository command was:

```text
CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR=/tmp/actus-wire-gate39-target \
  cargo test --all-targets -- --test-threads=1
```

## Definition of done

Phase 39 is complete when Actus provides a documented, bounded, deterministic
`std::wire` codec that serializes and parses arbitrary application payloads,
rejects malformed input safely, suppresses duplicate and stale frames, supports
bounded fragmentation, and exposes no hidden operating-system or allocation
dependency, while generated serialization contracts retain object and
executable parity. Encryption and physical transports remain separate future
work.
