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

- [ ] Implement a bounded sliding sequence window.
- [ ] Reject duplicate and stale frames deterministically.
- [ ] Define forward advancement and unreasonable jump behavior.
- [ ] Define sequence reset and context replacement without silent reuse.
- [ ] Test first value, advancement, duplicate, stale value, jump, and wrap.
- [ ] Document that this is replay suppression, not authentication.

## Gate 39.6 — Fragmentation and reassembly primitives

- [ ] Define versioned fragmentation metadata outside the canonical header.
- [ ] Bound fragment count, size, offsets, storage, and lifetime.
- [ ] Reject duplicate, missing, overlapping, stale, and over-sized fragments.
- [ ] Define cancellation and resource-exhaustion errors.
- [ ] Prove reassembly uses caller-owned storage without hidden allocation.

## Gate 39.7 — `wire://` endpoint contract

- [ ] Define the grammar for `wire://` endpoint addresses.
- [ ] Parse scheme, authority, port, and path without OS or transport calls.
- [ ] Reject malformed, ambiguous, or over-sized addresses with typed errors.
- [ ] Keep endpoint parsing separate from frame parsing and transport adapters.
- [ ] Add formatter and documentation evidence for the endpoint API.

## Gate 39.8 — Standard-library and compiler integration

- [x] Register `std::wire` as a target-neutral standard-library module.
- [x] Verify hosted and freestanding resolution without host-only imports.
- [ ] Verify unused Wire modules are not compiled or linked unnecessarily.
- [ ] Add formatter, semantic-model, hover, completion, and diagnostics
      evidence for public declarations.
- [ ] Add native object evidence for the bounded runtime surface.

## Gate 39.9 — Acceptance and future security boundary

- [ ] Run host round-trip and negative codec tests.
- [ ] Run native serialization, parsing, CRC, sequence, and failure tests.
- [ ] Run freestanding object checks with no hosted runtime dependency.
- [ ] Record transport-neutral evidence separately from physical transport
      evidence.
- [ ] Document encryption as a future profile behind an audited external crypto
      facade and explicit key/session contract.
- [ ] Run the repository formatting, compilation, Clippy, test, source-limit,
      and diff checks.
- [ ] Close the phase only when every implementation gate has tests and
      reproducible evidence.

## Definition of done

Phase 39 is complete when Actus provides a documented, bounded, deterministic
`std::wire` codec that serializes and parses arbitrary application payloads,
rejects malformed input safely, suppresses duplicate and stale frames, supports
bounded fragmentation, and exposes no hidden operating-system or allocation
dependency. Encryption and physical transports remain separate future work.
