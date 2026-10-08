# `std::path` platforms and representation

`PathPlatform` identifies the storage grammar used by a path. POSIX and
Windows roots are separate because separators, drive roots, and absolute-path
rules differ.

## Platform constructors

- `path_from_posix` accepts validated POSIX byte storage.
- `path_from_windows_utf16` accepts validated Windows UTF-16 units.
- `path_from_ascii` accepts the restricted ASCII representation documented by
  the facade.

The constructors establish platform metadata and ownership. They do not query
the host filesystem.

## Root and separator classification

Use `posix_root`/`posix_root_result` and `windows_root` for root classification.
Use `posix_is_separator` and `windows_is_separator` when interpreting
components. Do not replace separators manually before validation.

## Cross-platform behavior

A path retains its source representation. Joining or normalizing must respect
that platform's separator and root grammar. A POSIX path is not made into a
Windows path by changing `/` to `\\`, and UTF-16 Windows units are not ASCII
bytes.

## Views and errors

`PathComponent` values borrow the original path. Copy a component before the
path owner is moved or mutated. Handle invalid storage, invalid roots,
encoding, separators, and capacity failures before using a returned path or
component.
