# `std::path` usage

## Construct a path

Path construction validates the source representation and transfers the
caller-provided storage when the constructor takes `dat`:

```actus
import std::path;

verb make_path(dat storage: Buffer) -> Result[Path, PathError] {
    return path_from_ascii(storage: dat storage);
}
```

Use `path_from_posix` for POSIX bytes and `path_from_windows_utf16` for the
Windows representation. Do not use the ASCII constructor as a replacement for
UTF-16 Windows data.

## Inspect components

`parent`, `file_name`, `file_stem`, and `extension` return bounded component
views. The views borrow the path and must not outlive the path owner. The
`components` iterator advances through the path with `next_component`.

## Build and normalize

- `join` consumes a path and returns a new path with the other path appended.
- `push` mutates an existing path through `ins`.
- `normalize` performs lexical normalization and returns a path value.
- `set_file_name` and `set_extension` mutate through `ins`.
- `reserve_path` prepares reusable path capacity.

None of these operations checks whether a path exists on disk.
