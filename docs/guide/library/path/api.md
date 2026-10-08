# `std::path` public API reference

This is the usage reference for the public `std::path` facade. It describes
what application code may call and the contract of each operation. Private
runtime bridges and implementation helpers are intentionally excluded.

```actus
import std::path;
```

## Types

```actus
open struct Path {
    erg storage: Buffer,
    erg platform: PathPlatform,
    erg length: Int,
    erg capacity: Int,
    erg terminated: Int,
}

open enum PathPlatform {
    Posix,
    Windows,
}

open enum PathError {
    EmbeddedNull,
    InvalidEncoding,
    InvalidLength,
    MissingTerminator,
    CapacityExceeded,
    UnsupportedPlatform,
}
```

`Path` owns its storage. `length` excludes the terminator. `platform` selects
the raw representation and therefore the separator and root rules. Callers
should use the constructors rather than directly assembling metadata.

## Construction

```actus
open verb path_from_posix(dat storage: Buffer) -> Result[Path, PathError];
open verb path_from_windows_utf16(dat storage: Buffer) -> Result[Path, PathError];
open verb path_from_ascii(dat storage: Buffer) -> Result[Path, PathError];
```

All constructors consume the supplied buffer. On success the returned `Path`
owns it. On failure the rejected storage is cleaned up and the error identifies
the violated invariant.

`path_from_ascii` accepts stable ASCII input and chooses the host-native path
representation. It is not a general UTF-8, locale, or arbitrary code-page
conversion function.

## Component access

```actus
open struct PathComponent {
    abs source: Path,
    erg offset: Int,
    erg length: Int,
}

open struct PathComponents {
    abs source: Path,
    erg offset: Int,
    erg limit: Int,
}

open verb parent(abs self: Path) -> Option[abs PathComponent];
open verb file_name(abs self: Path) -> Option[abs PathComponent];
open verb file_stem(abs self: Path) -> Option[abs PathComponent];
open verb extension(abs self: Path) -> Option[abs PathComponent];
open verb components(abs self: Path) -> PathComponents;
open verb next_component(ins iter: PathComponents) -> Option[abs PathComponent];
```

Components borrow the source path. They do not own or copy bytes. `None` is
returned when a requested component does not exist. The iterator returns
components in path order and returns `None` at the end.

## Predicates

```actus
open verb is_absolute(abs self: Path) -> Bool;
open verb is_relative(abs self: Path) -> Bool;
open verb has_root(abs self: Path) -> Bool;
open verb starts_with(abs self: Path, abs prefix: Path) -> Bool;
open verb ends_with(abs self: Path, abs suffix: Path) -> Bool;
```

`starts_with` and `ends_with` compare complete components. They do not perform
raw byte prefix matching: `/usr/bin` starts with `/usr`, but not `/us`.

## Platform classification

```actus
open enum PosixRoot {
    Relative,
    Absolute,
}

open verb posix_is_separator(erg byte: Int) -> Int;
open verb posix_root(abs path: Path) -> Result[PosixRoot, PathError];

open enum WindowsRoot {
    Relative,
    DriveRelative,
    DriveAbsolute,
    Unc,
}

open verb windows_is_separator(erg byte: Int) -> Int;
open verb windows_root(abs path: Path) -> Result[WindowsRoot, PathError];
```

POSIX uses `/` as its separator and root marker. Windows accepts `/` and `\`
as separators and distinguishes `C:foo`, `C:\foo`, and UNC roots such as
`\\server\share`.

## Normalization and builders

```actus
open verb normalize(dat self: Path) -> Result[Path, PathError];
open verb join(dat self: Path, abs other: Path) -> Result[Path, PathError];
open verb push(ins self: Path, abs other: Path) -> Result[Int, PathError];
open verb reserve_path(ins self: Path, erg capacity: Int) -> Result[Int, PathError];
open verb set_file_name(ins self: Path, abs name: Path) -> Result[Int, PathError];
open verb set_extension(ins self: Path, abs extension: Path) -> Result[Int, PathError];
```

`normalize` and `join` consume the receiver and return it or a typed error.
`push`, `reserve_path`, and `set_*` borrow the receiver exclusively and return
the owner to the caller. `reserve_path` preserves existing path bytes.

All builders preserve the platform representation. Combining POSIX and Windows
paths returns `UnsupportedPlatform`. Insufficient existing capacity returns
`CapacityExceeded`; builders do not silently replace the path with another
allocation.

## Result handling

```actus
erg result = path_from_ascii(storage: dat raw);
return case dat result {
    Result.Ok(path) => use_path(path: dat path),
    Result.Err(error) => report_path_error(error: dat error),
};
```

Use `?` when the surrounding verb returns the same `PathError` domain. Never
use a `Path` after moving it to `join` or `normalize` unless the returned
`Result.Ok` branch gives back the new owner.
