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

- [ ] Mark the former Ustari split as superseded by ADR-0078.
- [ ] Confirm `std::wire` as the only public protocol namespace.
- [ ] Confirm the 12-byte header, byte order, payload bound, frame bound, and
      CRC16-CCITT parameters.
- [ ] Define version 1 flags and reject unknown bits.
- [ ] Define generic channels and message metadata without application schemas.
- [ ] Define `erg`, `abs`, `dat`, and `ins` ownership for every public API.
- [ ] Define the typed error taxonomy and failure behavior.
- [ ] Record the compiler and standard-library profile used as evidence.

## Gate 39.1 — Canonical module and public facade

- [ ] Add the canonical `library/std/src/wire/wire.act` facade.
- [ ] Add responsibility-specific siblings for frame, codec, parser, checksum,
      sequence, fragmentation, and errors.
- [ ] Export only intentional public declarations through the facade.
- [ ] Add positive and negative facade-import tests.
- [ ] Keep every source file below the Actus hard limit and document all public
      types, constants, enums, and verbs.

## Gate 39.2 — Frame model and deterministic serialization

- [ ] Implement named protocol constants for magic, version, lengths, flags,
      payload bound, and CRC parameters.
- [ ] Implement fixed-width little-endian header encoding and decoding.
- [ ] Validate payload length before payload access.
- [ ] Serialize valid frames into caller-owned bounded buffers.
- [ ] Reject insufficient output capacity with a typed error.
- [ ] Prove byte-for-byte deterministic serialization across repeated builds.
- [ ] Add empty, minimum, maximum, and over-sized payload tests.

## Gate 39.3 — CRC and typed failure paths

- [ ] Implement CRC16-CCITT with the ADR-0078 parameters.
- [ ] Reject bad magic, version, reserved flags, impossible lengths, and CRC.
- [ ] Distinguish framing, capacity, integrity, version, and policy failures.
- [ ] Prove rejected frames do not mutate caller-owned protocol state.
- [ ] Add native execution tests for successful and failed CRC paths.

## Gate 39.4 — Incremental bounded parser

- [ ] Accept arbitrary input chunks; never assume one read equals one frame.
- [ ] Track parser state using caller-owned bounded storage.
- [ ] Report consumed bytes and incomplete-frame status explicitly.
- [ ] Resynchronize after malformed magic, version, length, flags, or CRC.
- [ ] Bound scanning, buffering, and recovery work.
- [ ] Reject truncated input without dispatching a partial message.
- [ ] Add arbitrary-byte tests with no panic, out-of-bounds access, or
      unbounded loop.

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

- [ ] Register `std::wire` as a target-neutral standard-library module.
- [ ] Verify hosted and freestanding resolution without host-only imports.
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
