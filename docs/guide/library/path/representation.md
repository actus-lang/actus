# `std::path` representation and construction

## `Path`

```actus
open struct Path {
    erg storage: Buffer,
    erg platform: PathPlatform,
    erg length: Int,
    erg capacity: Int,
    erg terminated: Int,
}
```

`storage` owns the raw platform units. `length` excludes the trailing null
terminator. `capacity` describes the available storage and `terminated` records
the required terminator invariant. Callers should create paths through the
factory verbs rather than manufacturing this struct manually.

## Platform values

```actus
open enum PathPlatform {
    Posix,
    Windows,
}
```

`Posix` stores one byte per raw path unit. The bytes are not required to be
UTF-8; non-null bytes remain opaque path data. `Windows` stores native-endian
UTF-16 code units, including the terminator.

## Constructors

| Verb | Input | Result |
| --- | --- | --- |
| `path_from_posix(dat storage: Buffer)` | terminated POSIX raw bytes | owned POSIX `Path` |
| `path_from_windows_utf16(dat storage: Buffer)` | terminated UTF-16 units | owned Windows `Path` |
| `path_from_ascii(dat storage: Buffer)` | terminated ASCII bytes | host-native `Path` |

All constructors consume `storage`. On success the returned path owns it. On
failure the implementation drops rejected storage and returns `PathError`.
`path_from_ascii` is for stable ASCII names; it is not a general encoding
conversion API.

## Validation

Validation checks the terminator, logical length, capacity, embedded nulls, and
the selected unit encoding. An empty terminated path is valid. A POSIX path
may contain non-UTF-8 bytes; malformed UTF-16 code units are rejected by the
Windows constructor.
