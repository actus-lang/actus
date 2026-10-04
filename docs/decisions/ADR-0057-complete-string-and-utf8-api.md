# ADR-0057: Complete String and UTF-8 API Boundary

## Status

Proposed

## Context

Actus currently provides a production-grade low-level boundary for borrowed
`String` values and owned `Utf8Buffer` values. It supports checked byte access,
UTF-8 validation, caller-owned storage, typed failures, native execution, and
deterministic cleanup.

Applications also need scalar decoding, comparison, search, slicing, text
construction, encoding, ASCII transformations, and formatting. Adding these
operations without an ownership and ABI decision could introduce hidden
allocation, partial writes, invalid UTF-8 replacement, or accidental exposure
of raw runtime pointers.

## Decision

Extend `std::string` through explicit, typed, caller-buffer-backed contracts.

1. The existing null-terminated `String` ABI remains unchanged.
2. `String` is a borrowed read-only view and is used for inspection and
   provider calls through `abs`.
3. `Utf8Buffer` is the owned, length-delimited representation for validated
   dynamic text.
4. Builders, concatenation, slicing, encoding, and formatting write into
   caller-owned storage and return typed `Result` values.
5. Byte offsets and Unicode scalar positions are distinct concepts and must be
   represented by distinct API names and documentation.
6. Malformed UTF-8, invalid scalar values, invalid boundaries, overflow,
   capacity exhaustion, invalid storage, and unavailable providers are typed
   failures. No malformed input is silently replaced or truncated.
7. Runtime bridges remain private. Public Actus declarations translate all
   provider statuses before returning to application code.
8. Hosted providers may use the host runtime; freestanding targets must either
   provide the declared contract or receive a deterministic unavailable-provider
   diagnostic.
9. The API remains integer-only and must pass the repository zero-float audit.
10. Tokenization, vocabulary, sparse encoding, and neural processing remain
    application-level responsibilities.

## API shape

The API is grouped by responsibility:

- access: byte length, byte access, scalar decoding, and iteration;
- relation: equality, ordering, prefix/suffix/search, and bounded slicing;
- construction: owned buffers, builders, append, and concatenation;
- encoding: scalar-to-UTF-8 conversion and ASCII transformations;
- formatting: typed caller-buffer formatting and existing output bridges.

Each group has separate error and ownership documentation. A failed operation
must leave the caller's storage in the documented state and must not leak a
partially owned value.

## Alternatives considered

### Replace `String` with an owned growable string

Rejected. It would change the existing ABI and hide allocation policy from
embedded and freestanding consumers.

### Expose raw pointers and C string helpers

Rejected. Raw pointers bypass ownership, UTF-8 validation, and typed failure
handling.

### Put tokenizer and neural encoding in `std::string`

Rejected. Those are application-domain policies, not general text-storage
contracts.

### Silently replace malformed UTF-8

Rejected. Deterministic systems require explicit failure and lossless behavior.

## Consequences

Positive consequences:

- The existing ABI remains compatible.
- Embedded users retain explicit allocation and bounded-storage control.
- Unicode and byte-level operations cannot be confused silently.
- Native, LSP, formatter, and runtime behavior can share one public contract.

Costs:

- Callers must provide storage for owned results.
- Full Unicode normalization and locale-aware case conversion require separate
  future provider contracts.
- Every new operation requires semantic, ownership, native, and failure-path
  tests.

## Verification requirements

The implementation is accepted only with direct evidence for:

- semantic acceptance and rejection;
- typed status/error translation;
- ownership and deterministic cleanup;
- hosted and freestanding provider behavior;
- native object and executable output;
- zero-float IR;
- formatter, LSP, documentation, source-limit, and reproducibility checks.
