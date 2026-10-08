# `std::path` usage

## Construct a path

Path construction validates the source representation and transfers caller
storage when the constructor takes `dat`:

```actus
import std::path;

open verb make_path(dat storage: Buffer) -> Result[Path, PathError] {
    return path_from_ascii(storage: dat storage);
}
```

Use `path_from_posix` for POSIX bytes and
`path_from_windows_utf16` for Windows units. Do not use the ASCII constructor
as a substitute for platform-native Windows data.

## Inspect components

`parent`, `file_name`, `file_stem`, and `extension` return bounded component
views. A component view borrows the source path. Keep the path owner alive and
do not store a view after the path is consumed or replaced.

`components` creates iterator state and `next_component` advances it. The
iterator does not access the filesystem; it only interprets the path's stored
representation.

## Predicates

Use `is_absolute`, `is_relative`, and `has_root` for classification. Use
`starts_with` and `ends_with` for component-aware prefix/suffix checks under
the path contract. These predicates do not authorize external access.

## Build and normalize

- `join` consumes a path and returns a new path with the other path appended.
- `push` mutates an existing path through `ins`.
- `set_file_name` and `set_extension` mutate through `ins`.
- `reserve_path` prepares reusable capacity.
- `normalize` performs lexical normalization and returns a path value.

None of these operations checks whether a path exists on disk. Normalization
cannot replace authorization, sandbox checks, race handling, or filesystem
policy.
