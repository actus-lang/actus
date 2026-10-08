# `std::path`

`std::path` is the typed path representation used by `std::fs` and other APIs
that need filesystem names. It keeps platform representation, ownership,
length, capacity, termination, and root rules explicit. It does not convert
every path into a `String`.

```actus
import std::path;
```

## What this module provides

- validated owned `Path` values;
- POSIX raw-byte and Windows native UTF-16 representations;
- typed `PathError` failures;
- component views and a borrowed component iterator;
- root and separator classification for both platforms;
- component-aware predicates;
- lexical normalization;
- consuming and in-place path builders.

## Reading order

1. [Representation and construction](representation.md)
2. [Platform rules](platforms.md)
3. [Components and predicates](components.md)
4. [Builders and normalization](builders.md)
5. [Ownership and errors](ownership.md)
6. [Production contracts](contracts.md)
7. [Filesystem integration](integration.md)
8. [Examples](examples.md)
9. [Public API reference](api.md)
10. [Implementation source](../../../../library/std/src/path/)

The implementation is split into `types`, `storage`, `components`,
`predicates`, `normalize`, `builders`, `posix`, and `windows` source units.
The focused pages above explain those responsibilities in ordinary language;
the source docstrings remain the exact declaration-level contract.

## Important boundary

Path operations are lexical. `normalize`, `join`, `push`, root inspection, and
component queries do not open files, resolve symlinks, or prove that a path
exists. Filesystem side effects belong to `std::fs`.

## Detailed pages

- [Complete public API inventory](api.md)
- [Path implementation](../../../../library/std/src/path/)
- [Path tests](../../../../tests/path_builders.rs)

## Path lifecycle

1. Create a `Buffer` containing the source representation.
2. Call the platform-specific constructor with `dat` so the path owns the
   validated storage.
3. Inspect components and predicates through `abs Path`.
4. Use `push` or another builder through `ins`, or use a consuming builder such
   as `join` and `normalize` when the API moves the value.
5. Pass the finished path to `std::fs` without converting it to an unvalidated
   string.

Path normalization is lexical. It does not inspect the filesystem and cannot
prove that a path exists.
