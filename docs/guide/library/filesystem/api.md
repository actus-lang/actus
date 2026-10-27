# `std::fs` public API

This page describes the public facade. Raw `unsafe extern "C"` provider
verbs are private implementation details and are not part of the importable
API.

## `File`

```actus
struct File {
    handle: Int,
}
```

`File` owns one opaque hosted handle. The handle field is not an operating
system pointer and must not be interpreted by application code.

## `Metadata`

```actus
struct Metadata {
    size: Int,
    is_file: Int,
    is_dir: Int,
    readonly: Int,
    is_symlink: Int,
}
```

The fields are copied scalar facts. Flags use `1` for true and `0` for false.
The value owns no external resource.

## `OpenOptions`

```actus
struct OpenOptions {
    read: Int,
    write: Int,
    append: Int,
    truncate: Int,
    create: Int,
    create_new: Int,
}
```

Options are built as owned values and consumed by `options_open`.

```actus
struct CopyOptions {
    overwrite: Int,
    preserve_metadata: Int,
}
```

Construct a policy with `copy_options_new`, then consume it with
`copy_options_overwrite` and/or `copy_options_preserve_metadata`. These
builders perform no I/O. The metadata switch is intentionally limited to
readonly/permission bits; it does not imply timestamp, identity, ownership, or
extended-attribute preservation.

## `SeekFrom`

```actus
enum SeekFrom {
    Start(Int),
    Current(Int),
    End(Int),
}
```

The signed payload is a byte offset from the selected origin.

## File lifecycle verbs

```actus
verb file_open(abs path: Path) -> Result[File, IoError];
verb file_create(abs path: Path) -> Result[File, IoError];
verb file_read(ins self: File, ins buffer: Buffer) -> Result[Int, IoError];
verb file_write(ins self: File, abs buffer: Buffer) -> Result[Int, IoError];
verb file_flush(ins self: File) -> Result[Int, IoError];
verb file_close(ins self: File) -> Result[Int, IoError];
verb file_seek_start(ins self: File, abs offset: Int) -> Result[Int, IoError];
verb file_seek_current(ins self: File, abs offset: Int) -> Result[Int, IoError];
verb file_seek_end(ins self: File, abs offset: Int) -> Result[Int, IoError];
verb file_metadata(ins self: File) -> Result[Metadata, IoError];
```

Read returns bytes placed in the caller buffer. Write returns the number of
bytes accepted. Flush and close return the runtime status, normally zero on
success. Seek and metadata return the resulting position or copied facts.

## Metadata and options

```actus
verb metadata(abs path: Path) -> Result[Metadata, IoError];
verb symlink_metadata(abs path: Path) -> Result[Metadata, IoError];
verb options_new() -> OpenOptions;
verb options_read(dat options: OpenOptions) -> OpenOptions;
verb options_write(dat options: OpenOptions) -> OpenOptions;
verb options_append(dat options: OpenOptions) -> OpenOptions;
verb options_truncate(dat options: OpenOptions) -> OpenOptions;
verb options_create(dat options: OpenOptions) -> OpenOptions;
verb options_create_new(dat options: OpenOptions) -> OpenOptions;
verb options_open(dat options: OpenOptions, abs path: Path)
    -> Result[File, IoError];
```

Each option builder consumes and returns the configuration. The flags are
explicit; no host default is silently selected by the builder chain.

## Path operations

```actus
verb remove_file(abs path: Path) -> Result[Int, IoError];
verb canonicalize(abs path: Path) -> Result[Path, IoError];
verb same_file(abs left: Path, abs right: Path) -> Result[Bool, IoError];
verb rename(abs from: Path, abs to: Path) -> Result[Int, IoError];
verb copy_file(abs from: Path, abs to: Path) -> Result[Int, IoError];
verb copy_file_with_options(dat options: CopyOptions, abs from: Path,
    abs to: Path) -> Result[Int, IoError];
verb create_dir(abs path: Path) -> Result[Int, IoError];
verb remove_dir(abs path: Path) -> Result[Int, IoError];
verb read_to_bytes(abs path: Path) -> Result[Buffer, IoError];
verb read_to_string(abs path: Path) -> Result[Buffer, IoError];
verb write_file(abs path: Path, abs contents: Buffer) -> Result[Int, IoError];
verb write_file_atomic(
    abs path: Path,
    dat staging: Path,
    abs contents: Buffer,
) -> Result[Int, IoError];
verb sync_parent_directory(abs path: Path) -> Result[Int, IoError];
verb write_file_atomic_synced(
    abs path: Path,
    dat staging: Path,
    abs contents: Buffer,
) -> Result[Int, IoError];
```

Path operations borrow paths. `read_to_bytes` transfers an owned result buffer
on success. `read_to_string` transfers a buffer only after UTF-8 validation.
`write_file_atomic` consumes the staging path so failure cleanup has one clear
owner. `write_file_atomic_synced` has the same ownership contract and performs
an explicit parent-directory sync after rename. It returns an error if that
sync is unsupported or fails, but the destination has already been published
in that case; callers must treat durability as uncertain. The Unix backend
supports the directory sync, while unsupported targets return
`IoError.Unsupported`.

`canonicalize` performs blocking host filesystem resolution, follows symbolic
links, requires every component to exist, and returns a new owned identity
`Path`. It is distinct from lexical `std::path::normalize`, which performs no
filesystem access. A missing, inaccessible, invalid, or unsupported path is
reported as `IoError.Failed` and the input path is not modified.

`same_file` follows symbolic links and compares host identity metadata. It
returns `Unsupported` when the target cannot provide a stable identity.

`CopyOptions` contains explicit `overwrite` and `preserve_metadata` switches.
Disabled overwrite uses create-new destination semantics and returns
`IoError.AlreadyExists` without changing an existing destination. Enabled
overwrite replaces the regular-file destination. Metadata preservation copies
only readonly/permission bits; timestamps, identity, ownership, and extended
attributes are not copied. The two-argument `copy_file` wrapper explicitly
uses overwrite enabled and metadata preservation disabled.

## Results

| Operation group | Success value | Typical failure |
| --- | --- | --- |
| open/create/options | `File` | `IoError.Failed` |
| read/write | byte count | `Failed`, `EndOfStream` |
| flush/close | runtime status | `Failed` |
| seek | absolute byte position | `Failed` |
| metadata | `Metadata` | `Failed` |
| symlink metadata | `Metadata` | `Failed` |
| read-to-bytes | owned `Buffer` | `Failed`, `EndOfStream` |
| read-to-string | UTF-8 `Buffer` | I/O error, `InvalidData` |
| rename/copy/remove/dir | status or count | `Failed`, `AlreadyExists`, `CrossDevice` |
| atomic write | written byte count | write, flush, rename, or cleanup error |
| same-file comparison | `Bool` | lookup failure or `Unsupported` |

## Temporary resources

```actus
enum TemporaryError {
    Failed,
    InvalidPrefix,
    CollisionExhausted,
}

struct TemporaryFile {
    file: File,
    path: Path,
}

struct TemporaryDirectory {
    path: Path,
}

verb temporary_file_open(abs root: Path, abs prefix: Buffer)
    -> Result[TemporaryFile, TemporaryError];
verb temporary_directory_open(abs root: Path, abs prefix: Buffer)
    -> Result[TemporaryDirectory, TemporaryError];
verb temporary_file_cleanup(ins self: TemporaryFile)
    -> Result[Int, IoError];
verb temporary_directory_cleanup(ins self: TemporaryDirectory)
    -> Result[Int, IoError];
```

The caller supplies the workspace root; the API never selects a global
temporary directory implicitly. The runtime rejects symlink roots, restricts
the prefix to bounded ASCII name bytes, obtains OS entropy, and uses atomic
create-new or mkdir semantics with bounded collision retries. The returned
resource owns the generated path. Explicit cleanup closes/removes the file or
removes the empty directory and returns failures; Drop performs best-effort
cleanup when control leaves scope unexpectedly.
