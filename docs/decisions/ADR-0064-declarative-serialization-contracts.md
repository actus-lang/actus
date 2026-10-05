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

The declaration names the source `pack` or fixed aggregate, then states the
wire sections in byte order. Every section has an explicit offset and width.
`version` names a typed integer field. `payload bytes` names a bounded byte
range. `checksum` names an explicit algorithm and coverage interval. Alignment

For a contract named `Frame`, the compiler reserves deterministic generated
API names `frame_encode`, `frame_decode`, `frame_validate`, and `frame_migrate`.
These names are checked for collisions before native lowering. The first
implementation now exposes `frame_validate` as an allocation-free wrapper. It
accepts a caller-owned `Buffer` and an expected version, then passes the
declared offsets, lengths, endianness, and checksum range to the runtime
validator. Encode, decode, and migration remain later implementation gates;
their names and compatibility rules are fixed by this ADR.
and padding are declarations, never inferred from the host ABI.

The initial profile supports `little` and `big` endian integer fields, explicit
padding, a fixed payload range, `u16` version fields, and `crc32`. Additional
algorithms or variable-length sections require a separate ADR.

## Generated API

For `Frame`, the compiler generates deterministic typed operations behind the
canonical module facade:

```text
frame_encode(abs value: Frame, ins output: Buffer)
frame_decode(ins input: Buffer)
frame_validate(abs input: Buffer)
frame_migrate(old_version, ins input: Buffer, ins output: Buffer)
```

The exact Actus result types and error domain are part of the implementation
contract. Every operation returns typed success/failure information. Encode
uses caller-owned `ins Buffer`; decode and validate inspect caller-owned input
according to their declared role. No generated operation allocates or opens a
path. Filesystem persistence remains an explicit hand-written facade operation.

## Validation and atomic persistence

The compiler rejects overlapping sections, out-of-order offsets, width and
alignment mismatches, checksum ranges that include the checksum field, invalid
version declarations, unsupported algorithms, and total sizes that exceed the
declared layout. It preserves source spans for each section and reports errors
at the declaration that introduced the conflict.

Atomic persistence is a library hook, not an implicit side effect of encode.
The filesystem facade may write `name.tmp`, flush and validate the complete
bytes, atomically rename the file, and then publish a manifest update. Recovery
must ignore uncommitted staged files. The serializer only produces and checks
bytes; it does not decide when a commit is durable.

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
- add reference-byte, corruption, truncation, version, and staged-file tests;
- compare object and executable output against an explicit reference encoder.
