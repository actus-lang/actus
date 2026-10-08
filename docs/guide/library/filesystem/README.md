# `std::fs`

`std::fs` provides hosted filesystem access through typed Actus values. It
covers owned file handles, metadata, open modes, cursor movement, one-shot
reads and writes, directory operations, and atomic publication.

```actus
import std::fs;
import std::path;
```

Filesystem calls are external side effects. They belong at an explicit
application, persistence, or service boundary. Do not put synchronous file
access in inference, impulse forwarding, parsing, or another latency-critical
hot path.

## Reading order

1. [API](api.md) — complete public declarations and result contracts.
2. [File handles](file-handles.md) — open, read, write, seek, flush, close,
   and deterministic cleanup.
3. [Open options](options.md) — explicit read/write/create/append behavior.
4. [Operations](operations.md) — one-shot I/O, directories, rename, copy, and
   atomic publication.
5. [Ownership and lifecycle](lifecycle.md) — `abs`, `ins`, `dat`, and failure
   paths.
6. [Errors and runtime](errors.md) and [runtime](runtime.md).
7. [Usage](usage.md) and [examples](examples.md).

## Main type groups

- `File` owns one opaque host handle and closes it through the `Drop`
  performance when its owner leaves scope.
- `Metadata` contains copied scalar facts and owns no host resource.
- `OpenOptions` is an owned set of scalar switches consumed by `options_open`.
- `SeekFrom` selects the origin for cursor movement.
- `IoError` is supplied by `std::io` and is used for every fallible filesystem
  result.

## Path boundary

Filesystem APIs receive `Path`, not arbitrary text. Build and validate paths
with `std::path` first. Path parameters are normally `abs` views: the runtime
uses them for one call and does not retain or copy them as part of a `File`.

## Important distinctions

- `read_to_bytes` returns arbitrary bytes; `read_to_string` additionally
  validates UTF-8 and returns `InvalidData` for malformed text.
- `write_file` creates or truncates a destination directly.
- `write_file_atomic` writes a caller-selected staging path, flushes it, then
  renames it to the final path and removes staging on failure.
- `File` operations provide a controlled streaming lifecycle; one-shot verbs
  are simpler for bounded files.

## Source and evidence

- [API](api.md) documents the public facade, not private C ABI bridges.
- The implementation is in
  [`library/std/src/fs/`](../../../../library/std/src/fs/).
- Contract fixtures are in
  [`tests/library/`](../../../../tests/library/), including `fs_contracts.act`,
  `fs_handles.act`, `fs_errors.act`, and `fs_create_new.act`.
