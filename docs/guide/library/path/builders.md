# `std::path` builders and normalization

## Consuming builder

`join(dat self: Path, abs other: Path) -> Result[Path, PathError]` consumes
the left path and returns its storage after the combination. A relative
`other` is appended using the platform separator. An absolute `other` replaces
the receiver payload according to the platform contract.

If the operation fails, the consumed receiver is cleaned up and the error is
returned. The `other` path remains an `abs` view and is not consumed.

## In-place builders

| Verb | Effect |
| --- | --- |
| `push(ins self, abs other)` | Appends another path in the receiver's storage. |
| `reserve_path(ins self, erg capacity)` | Ensures reusable capacity. |
| `set_file_name(ins self, abs name)` | Replaces the final name. |
| `set_extension(ins self, abs extension)` | Replaces the final extension. |

These operations use an exclusive loan and restore the caller's path owner
after returning. They do not silently change platform representation. A path
and replacement value from incompatible platforms returns
`PathError.UnsupportedPlatform`.

## `normalize`

`normalize(dat self)` removes lexical dot components and repeated separators
using the existing owned storage. It does not access the filesystem or resolve
symbolic links. Absolute roots prevent `..` from escaping above the root.

Normalization consumes and returns the path on success. Handle the returned
`Result[Path, PathError]` before using the path again.
