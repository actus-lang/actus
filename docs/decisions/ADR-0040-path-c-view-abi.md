# ADR-0040: Borrowed Native Path C Views

## Status

Accepted with Phase 16 Gate 3.8 Sub-gate F.

## Context

`std::path` stores POSIX paths as arbitrary non-null bytes and Windows paths as
validated UTF-16 code units. Native filesystem bridges need the original
representation, including its null terminator, without allocating a `String`,
transcoding UTF-16, or copying the path payload.

## Decision

The runtime exposes two caller-slot C ABI functions:

- `actus_path_c_view` writes an `ActusPathCView` for POSIX storage.
- `actus_path_wide_c_view` writes an `ActusPathWideCView` for Windows storage.

Each descriptor contains a pointer to the existing payload and its logical
length. The pointer addresses the source path's storage, and the terminator is
present at `data[length]`. Windows `length` counts UTF-16 code units while the
underlying pointer addresses native-endian `u16` storage.

The caller supplies the output descriptor. A successful bridge returns that
same output pointer; invalid input, a platform mismatch, or a null output slot
returns null. The bridge performs no heap allocation and never transfers or
retains ownership.

## Lifetime and mutation contract

The view is valid only while the source `Path` and its backing `Buffer` remain
alive and unmodified. The source must not be moved, dropped, reallocated, or
mutated while native code uses the pointer. The output descriptor is caller
owned and may be discarded after the native call returns.

The runtime validates the descriptor metadata, payload bounds, required
terminator, embedded-null rule, UTF-16 surrogate structure, and Windows pointer
alignment before returning a view. It does not normalize, resolve symlinks, or
perform filesystem access.

## Consequences

- POSIX C calls receive raw bytes and may handle non-UTF-8 names.
- Windows C calls receive the platform-native wide representation.
- C bridges cannot retain a view beyond the borrowed source lifetime.
- Consumers must handle a null return as an invalid or incompatible path.
- Future `std::fs` runtime verbs can use these views directly at their ABI
  boundary without changing the public `Result[..., IoError]` contract.

## Verification

`tests/path_c_view.rs` verifies pointer identity, logical lengths, terminators,
non-UTF-8 POSIX bytes, UTF-16 code units, null output rejection, platform
mismatch rejection, and malformed termination rejection.
