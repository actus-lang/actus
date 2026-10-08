# `std::fs`

```actus
import std::fs;
import std::path;
```
`std::fs` provides hosted filesystem operations through typed file handles,
metadata, open options, seek origins, and path-based operations.

The public facade contains file, metadata, operations, options, and seek
contracts. Filesystem access is hosted and performs external side effects; it
must not be placed in inference, parsing, or other latency-sensitive hot paths
unless the application explicitly owns that boundary.

Use typed result values for open, read, write, seek, metadata, rename, and
remove operations. Document path ownership, handle cleanup, append behavior,
and error variants at the API boundary.

## Public surface

The facade includes:

- `File` and `file_open`, `file_create`, `file_read`, `file_write`,
  `file_flush`, `file_close`, and seek operations;
- `Metadata` and metadata queries;
- `OpenOptions` with read, write, append, truncate, create, and create-new
  configuration;
- filesystem operations for read, write, atomic write, rename, copy, remove,
  and directory creation/removal;
- `SeekFrom` positions.

File handles are owned resources. Mutating operations use an exclusive role and
must preserve handle cleanup on success and failure. Path-based convenience
operations inspect `abs Path`; contents are inspected through `abs Buffer` or
returned in caller-owned buffers according to the declaration.

## Detailed pages

- [Usage guide](usage.md)
- [Source implementation](../../../../library/std/src/fs/)
- [Filesystem tests](../../../../tests/fs.rs)

## Operation groups

- Use `file_open` or `file_create` when the caller needs a persistent `File`
  handle. The handle owns the native resource and must be flushed and closed
  through its typed operations.
- Use `OpenOptions` when read, write, append, truncate, create, or create-new
  behavior must be selected explicitly.
- Use `file_read` and `file_write` with caller-owned buffers. Read destinations
  use `ins`; write sources use `abs`.
- Use `read_to_bytes` or `read_to_string` for one-shot path operations. These
  return owned buffers and therefore must be handled as moved values.
- Use `write_file_atomic` when staging and publication must be separate. The
  operation writes the staging path, flushes it, publishes it with rename, and
  reports typed failure when any step cannot complete.

Filesystem operations are hosted side effects. Keep them behind an explicit
application or persistence boundary rather than inside a computation hot path.
