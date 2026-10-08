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
verb rename(abs from: Path, abs to: Path) -> Result[Int, IoError];
verb copy_file(abs from: Path, abs to: Path) -> Result[Int, IoError];
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
```

Path operations borrow paths. `read_to_bytes` transfers an owned result buffer
on success. `read_to_string` transfers a buffer only after UTF-8 validation.
`write_file_atomic` consumes the staging path so failure cleanup has one clear
owner.

## Results

| Operation group | Success value | Typical failure |
| --- | --- | --- |
| open/create/options | `File` | `IoError.Failed` |
| read/write | byte count | `Failed`, `EndOfStream` |
| flush/close | runtime status | `Failed` |
| seek | absolute byte position | `Failed` |
| metadata | `Metadata` | `Failed` |
| read-to-bytes | owned `Buffer` | `Failed`, `EndOfStream` |
| read-to-string | UTF-8 `Buffer` | I/O error, `InvalidData` |
| rename/copy/remove/dir | status or count | `Failed` |
| atomic write | written byte count | write, flush, rename, or cleanup error |
