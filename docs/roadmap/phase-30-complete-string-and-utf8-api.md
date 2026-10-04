# Phase 30: Complete String and UTF-8 API

## Status

Planned. Phase 29 established the production text boundary for borrowed
`String`, owned `Utf8Buffer`, caller-owned `Buffer` storage, typed errors,
native emission, formatter, LSP, and deterministic acceptance. Phase 30
extends that boundary into a complete low-level text API without embedding an
application-specific tokenizer, vocabulary, encoder, or neural engine.

## Objective

Provide the standard library with the operations required for production text
processing while preserving Actus ownership, bounded-storage, deterministic
cleanup, and runtime-profile contracts.

The implementation must preserve the existing null-terminated `String` ABI.
Operations that need retained or mutable text use caller-owned `Buffer` storage
and return owned `Utf8Buffer` values. No API may silently replace malformed
UTF-8, truncate data, allocate behind an `abs` view, or expose a raw pointer.

## Architectural decisions

- `String` remains a borrowed, null-terminated hosted text view.
- `Utf8Buffer` remains an owned, length-delimited validated UTF-8 value.
- Unicode decoding is separate from byte access and returns typed failures.
- Search and comparison APIs are read-only and do not transfer ownership.
- Concatenation, encoding, case conversion, and formatting write into explicit
  caller-provided storage.
- Application-specific tokenization, vocabularies, sparse encodings, and
  neural processing remain outside the Actus standard library.
- Raw runtime providers remain private; public APIs expose `Result` values.

## Gate 30.0: Status, ABI, and error-contract hardening

- [ ] Audit every String/UTF-8 runtime status and map every failure value to a
      typed `StringError`; unknown negative statuses must never become `Ok`.
- [ ] Define stable status contracts for hosted providers and freestanding
      provider implementations.
- [ ] Preserve the existing `String` ABI and verify object compatibility with
      existing `std::io` text output.
- [ ] Define overflow, invalid storage, capacity, malformed UTF-8, and provider
      unavailable behavior for every new API.
- [ ] Add accepted and rejected semantic tests for all public signatures.

### Gate 30.0 evidence

- Status-mapping unit tests for every known and unknown provider result.
- Hosted and freestanding capability diagnostics.
- Existing String literal and text-output regression suite remains green.

## Gate 30.1: Unicode scalar-value decoding and iteration

- [ ] Add a checked UTF-8 scalar decoder separate from byte indexing.
- [ ] Define return representation for scalar values and invalid sequences.
- [ ] Support one-, two-, three-, and four-byte UTF-8 sequences with boundary
      and overlong-sequence validation.
- [ ] Reject surrogate code points, truncated sequences, invalid continuation
      bytes, and values above `U+10FFFF`.
- [ ] Provide a bounded iterator/cursor contract that cannot escape the source
      borrow or allocate implicitly.
- [ ] Add ASCII, multibyte, combining, four-byte, empty, malformed, and
      truncated-input tests.

### Gate 30.1 evidence

- Scalar decoding golden vectors and rejected-input diagnostics.
- Native execution tests for forward iteration and end-of-input behavior.
- No-allocation and ownership-restoration evidence.

## Gate 30.2: Comparison, search, and bounded slicing

- [ ] Add byte-exact equality and ordering for validated UTF-8 values.
- [ ] Add scalar-aware comparison where the contract requires decoded values.
- [ ] Add bounded prefix, suffix, and substring search operations.
- [ ] Define whether search indices are byte offsets or scalar positions and
      expose the distinction in names and documentation.
- [ ] Add a checked slicing operation that returns an owned value only through
      caller-provided storage.
- [ ] Reject invalid boundaries, overflowed ranges, and insufficient output
      capacity deterministically.

### Gate 30.2 evidence

- Deterministic comparison/search vectors for ASCII and multibyte text.
- Accepted and rejected slice-boundary tests.
- Native object/executable tests with exact result codes and no allocation.

## Gate 30.3: Owned text construction and concatenation

- [ ] Add an explicit `Utf8Builder` or equivalent caller-buffer-backed
      construction contract.
- [ ] Support append of validated UTF-8 buffers, borrowed Strings, bytes, and
      decoded scalar values through separate typed operations.
- [ ] Preserve transactional behavior: failed append must not return a
      partially owned value or leak caller storage.
- [ ] Define capacity reservation, length accounting, overflow, and cleanup.
- [ ] Support empty values and embedded NUL bytes without using the String ABI.
- [ ] Add repeated-append, capacity exhaustion, malformed input, and cleanup
      tests.

### Gate 30.3 evidence

- End-to-end caller-buffer construction and concatenation fixtures.
- Failure-path ownership and deterministic cleanup tests.
- Native execution and repeated object reproducibility evidence.

## Gate 30.4: UTF-8 encoding and ASCII transformations

- [ ] Add checked scalar-value-to-UTF-8 encoding into caller-owned storage.
- [ ] Define ASCII-only upper/lower transformation semantics explicitly.
- [ ] Preserve non-ASCII bytes during ASCII transformations.
- [ ] Reject scalar values that cannot be encoded and report capacity failure
      without partial ownership transfer.
- [ ] Keep locale-dependent and Unicode full case-folding behavior out of this
      integer-only low-level API unless a separate provider contract is added.
- [ ] Add golden vectors for encoding, ASCII case conversion, embedded NUL,
      malformed input, and capacity exhaustion.

### Gate 30.4 evidence

- Exact byte fixtures for every supported scalar width.
- Native zero-float and no-allocation verification.
- Hosted/freestanding availability diagnostics.

## Gate 30.5: Text formatting and interoperability

- [ ] Define a typed formatting sink that writes into caller-owned storage.
- [ ] Support integer and byte-oriented formatting without hidden allocation.
- [ ] Define failure behavior for unsupported values, invalid UTF-8, and
      insufficient capacity.
- [ ] Keep `std::io::print` and `println` ABI-compatible while allowing owned
      UTF-8 values to be passed through the documented borrowing boundary.
- [ ] Add formatting, output, and round-trip tests with exact bytes.

### Gate 30.5 evidence

- Formatter and LSP coverage for every public formatting declaration.
- Native stdout and buffer-sink tests.
- Object/executable and zero-float evidence.

## Gate 30.6: Tooling, documentation, and profile parity

- [ ] Add formatter, LSP hover, completion, definition, diagnostics, and
      visibility coverage for all new public APIs.
- [ ] Document every type, verb, error, ownership role, index unit, and ABI
      boundary in the coding guide and standard-library documentation.
- [ ] Reject private provider bridges through semantic visibility tests.
- [ ] Verify hosted and freestanding imports use the same public contract and
      produce deterministic unavailable-provider diagnostics.
- [ ] Keep every Actus source file and function within repository limits.

### Gate 30.6 evidence

- Complete public API documentation test.
- LSP/formatter parity suite.
- Source-limit, architecture, and visibility checks.

## Gate 30.7: Resource, safety, and determinism acceptance

- [ ] Add no-allocation tests for every caller-owned-storage API.
- [ ] Add malformed-input, bounds, overflow, stale-storage, and capacity
      rejection tests.
- [ ] Verify deterministic cleanup on every error, early return, and failed
      builder operation.
- [ ] Verify no unsafe bridge is public or undocumented.
- [ ] Verify zero-float IR for integer-only text operations.
- [ ] Verify repeated object and executable builds meet the documented
      reproducibility contract.

### Gate 30.7 evidence

- Full runtime, semantic, native, ownership, and IR-audit results.
- Reproducibility comparison and failure-path cleanup evidence.

## Gate 30.8: End-to-end production acceptance

- [ ] `actus check --strict` passes the complete String/UTF-8 fixture.
- [ ] `actus test --strict` passes accepted and rejected text API fixtures.
- [ ] Native object and executable builds link without unresolved providers.
- [ ] The executable demonstrates decode, compare, search, slice, build,
      encode, transform, and formatting operations.
- [ ] Hosted/freestanding behavior is validated against the selected runtime
      contracts.
- [ ] Formatter, LSP, documentation, source-limit, ownership, determinism,
      and zero-float checks pass.
- [ ] Every checked box has direct repository evidence; no source-only check
      closes a runtime or ABI requirement.

## Non-goals

- This phase does not implement tokenizers, vocabularies, sparse encoders, or
  neural-network behavior.
- This phase does not add locale databases or full Unicode normalization.
- This phase does not replace the existing null-terminated String ABI.
- This phase does not hide allocation or provider requirements behind a simple
  name.

## Completion criteria

Phase 30 is complete only when the public String/UTF-8 API supports validated
scalar decoding, comparison/search, bounded slicing, owned construction,
encoding, ASCII transformation, and typed formatting with direct semantic,
native, runtime, tooling, ownership, determinism, and profile evidence.
