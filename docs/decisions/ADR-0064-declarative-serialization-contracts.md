# ADR-0064: Declarative Serialization Contracts

## Status

Accepted for Phase 33, Gate 33.5.

## Context

Actus already provides `pack`, fixed arrays, bounded `Buffer`, checked integer
operations, typed `Result`, and explicit ownership roles. Serialization is still
written as repeated byte indexing and append code. That makes byte order,
padding, checksum coverage, version checks, and failure behavior difficult to
review consistently.

The serialization feature must remain a compile-time layout contract. It must
not create a hidden heap, call the filesystem from generated code, or turn a
buffer view into an implicit ownership transfer.

## Proposed declaration

The proposed syntax is a declaration associated with an existing fixed layout:

```act
serialize Frame from PackFrame {
    layout little;
    version u16 at 0;
    payload bytes at 2 length 16;
    checksum crc32 over 0 .. 18 at 18;
}
```

The generated operations use the following typed error domain:

```act
enum SerializationError {
    BufferTooSmall,
    InvalidVersion,
    InvalidChecksum,
    InvalidLayout,
    UnsupportedVersion,
}
```

The declaration names the source `pack` or fixed aggregate, then states the
wire sections in byte order. Every section has an explicit offset and width.
`version` names a typed integer field. `payload bytes` names a bounded byte
range. `checksum` names an explicit algorithm and coverage interval. Alignment

For a contract named `Frame`, the compiler reserves deterministic generated
API names `frame_encode`, `frame_decode`, `frame_validate`, and `frame_migrate`.
These names are checked for collisions before native lowering. The current
implementation exposes `frame_validate` as an allocation-free wrapper and
`frame_encode` as a generated typed byte writer, and `frame_decode` as a
fixed, version-aware byte reader. The validator accepts a
caller-owned `Buffer` and an expected version, then passes the declared
offsets, lengths, endianness, and checksum range to the runtime validator. The
encoder copies the source pack's complete byte storage into the caller-owned
output buffer and returns the encoded byte count. The decoder first checks the
input length through the compiler-provided `buffer_length` primitive, compares
the encoded version with the caller's expected version, validates the checksum,
and returns typed `InvalidLayout`, `InvalidVersion`, or `InvalidChecksum`
failures before copying bytes into a new pack value. Migration remains a later
implementation gate. Alignment and padding are declarations, never inferred
from the host ABI.

The initial profile supports `little` and `big` endian integer fields, explicit
padding, a fixed payload range, `u16` version fields, and `crc32`. Additional
algorithms or variable-length sections require a separate ADR.

## Generated API

For `Frame`, the compiler generates deterministic typed operations behind the
canonical module facade:

```text
frame_encode(abs value: Frame, ins output: Buffer) -> Result[u32, SerializationError]
frame_decode(abs input: Buffer, abs expected_version: u16) -> Result[Frame, SerializationError]
frame_validate(abs input: Buffer, abs expected_version: u16) -> Result[Bool, SerializationError]
frame_migrate(
    abs input: Buffer,
    abs from_version: u16,
    abs to_version: u16,
    ins output: Buffer
) -> Result[u32, SerializationError]
```

Every operation returns typed success/failure information. Encode uses
caller-owned `ins Buffer` and returns `Result[u32, SerializationError]` after
copying the fixed source storage. Decode accepts caller-owned input and an
expected version, validates the frame, and returns an owned pack. No generated
operation opens a path.
Filesystem persistence remains an explicit hand-written facade operation.

Migration is explicit about both version endpoints. It validates the input
against `from_version`, copies the fixed frame into the caller-owned output,
rewrites the version field to `to_version`, recomputes the declared checksum,
and returns the encoded byte count. The caller must provide an empty output
buffer. Migration does not perform filesystem I/O or publish a persistence
commit.

## Validation and atomic persistence

The compiler rejects overlapping sections, out-of-order offsets, width and
alignment mismatches, checksum ranges that include the checksum field, invalid
version declarations, unsupported algorithms, and total sizes that exceed the
declared layout. It preserves source spans for each section and reports errors
at the declaration that introduced the conflict.

Atomic persistence is a library hook, not an implicit side effect of encode.
The `std::fs::write_file_atomic` hook accepts the final path as `abs`, a
caller-selected same-filesystem staging path as `dat`, and serialized bytes as
`abs`. It writes and flushes the staging file, atomically renames it to the
final path, and removes the staging path when writing, flushing, or renaming
fails. Recovery must ignore uncommitted staged files. The serializer only
produces and checks bytes; it does not decide when a commit is durable. A
separate manifest update and directory durability policy remain above this
hook.

## Rejected alternatives

- Implicit serialization for every `struct` or `pack` is rejected because it
  would make wire compatibility depend on declaration changes.
- Host ABI layout is rejected because alignment and padding vary by target.
- Generated filesystem writes are rejected because they hide I/O latency,
  ownership, failure, and atomicity boundaries.
- Dynamic unbounded payloads are rejected in the initial profile because their
  allocation and failure cost cannot be statically reviewed.
- A checksum field without an explicit coverage range is rejected because it is
  ambiguous during migration and corruption recovery.

## Open implementation gates

- finalize the accepted keyword and field grammar;
- add AST, semantic layout validation, formatter, and LSP support;
- generate allocation-free native encode/decode/validate paths;
- expose `SerializationError` as a compiler-provided enum;
- add reference-byte, corruption, truncation, version, and staged-file tests;
- compare object and executable output against an explicit reference encoder.

The failed-publication recovery case is accepted after closing the compiler
ownership boundary that it exposed. Regular enum payloads are stored inline in
their containing result, while the native representation of an owned enum is
an allocation. When a case extracts such a payload, native lowering now
materializes an owned enum allocation and copies the inline bytes into it.
This keeps subsequent `dat` calls and cleanup on valid allocation boundaries.
The regression verifies that a failed rename removes the staging file,
preserves the existing destination directory, and exits without an invalid
free or signal.

The native migration regression uses an independent reference frame: version
`2`, payload byte `7`, and the little-endian IEEE CRC32 bytes
`DF 98 A1 62`. The executable must produce those exact eight bytes, including
the unchanged trailing byte, before the byte-compatibility gate can close.
