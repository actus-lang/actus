# `std::path` production contracts

## Storage invariant

Every valid path has one owned buffer, a selected platform representation, a
logical payload length, enough capacity for the payload and terminator, and a
valid trailing terminator. `length` never includes the terminator.

POSIX storage uses one-byte raw units and may contain non-UTF-8 bytes. Windows
storage uses native-endian UTF-16 code units. A malformed UTF-16 sequence is
`InvalidEncoding`; it is not converted or repaired.

## Ownership invariant

- Constructors take `dat Buffer` and own it only after validation succeeds.
- Read-only queries take `abs Path` and cannot mutate or retain the path.
- In-place builders take `ins Path` and restore the owner after the call.
- Consuming builders take `dat Path` and return the replacement owner in `Ok`.
- Components and iterators borrow the source path and cannot outlive it.

These roles are part of the API contract. A caller cannot replace an `ins` or
`dat` operation with an implicit copy.

## Lexical boundary

Path operations inspect and transform path data only. They do not:

- open or stat a file;
- resolve symbolic links;
- consult the current working directory;
- guarantee that a path exists;
- convert a path into a text string;
- perform filesystem I/O.

Use `std::fs` for external filesystem effects.

## Error matrix

| Error | Typical cause | Caller action |
| --- | --- | --- |
| `EmbeddedNull` | Null unit before the final terminator. | Reject the input representation. |
| `InvalidEncoding` | Invalid UTF-16 or invalid platform storage. | Repair or replace the source data. |
| `InvalidLength` | Inconsistent metadata or illegal component data. | Rebuild the path with a valid constructor. |
| `MissingTerminator` | Raw storage is not terminated. | Add the required terminator before construction. |
| `CapacityExceeded` | A builder cannot fit the result in existing storage. | Reserve capacity before mutation or use a larger owned buffer. |
| `UnsupportedPlatform` | POSIX and Windows representations were combined. | Convert through an explicit platform boundary. |

## Concurrency and views

`PathComponent` and `PathComponents` are borrowed views. Do not retain them
while mutating or moving their source path. If a component must survive the
source path, copy its units into separately owned storage through an explicit
application operation.
