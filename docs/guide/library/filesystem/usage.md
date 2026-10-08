# Using `std::fs`

## Import and prepare a path

Filesystem verbs receive `Path`, not arbitrary text. Construct the path with
`std::path` first:

```actus
import std::fs;
import std::path;

open verb load_path(dat raw: Buffer) -> Result[Buffer, IoError] {
    erg path_result = path_from_ascii(storage: dat raw);
    return case dat path_result {
        Result.Err(error) => Err(IoError.InvalidInput),
        Result.Ok(path) => read_to_bytes(path: abs path),
    };
}
```

The exact path constructor depends on the input representation. Keep the
result branch explicit and do not pass unchecked text directly to `std::fs`.

## Read a complete file

Use `read_to_bytes` when the file is bounded and the application wants one
owned result buffer:

```actus
open verb load(abs path: Path) -> Result[Buffer, IoError] {
    return read_to_bytes(path: abs path);
}
```

Use `read_to_string` only when the file must be valid UTF-8. It returns
`InvalidData` for malformed bytes.

## Stream through a file handle

Use a handle when the caller controls storage and cursor lifetime:

```actus
open verb read_chunk(ins file: File, ins bytes: Buffer)
    -> Result[Int, IoError] {
    return file_read(self: ins file, buffer: ins bytes);
}
```

The destination must already have suitable capacity. The operation returns
how many bytes were written into it.

## Write and flush

```actus
open verb save(abs path: Path, abs bytes: Buffer) -> Result[Int, IoError] {
    return write_file(path: abs path, contents: abs bytes);
}
```

For a persistence boundary that must publish a complete staged result, use
`write_file_atomic`. Direct `write_file` creates or truncates the destination
before writing.

## Explicit handle lifecycle

```actus
open verb append_record(abs path: Path, abs record: Buffer)
    -> Result[Int, IoError] {
    erg options = options_new();
    erg writable = options_write(dat options);
    erg appendable = options_append(dat writable);
    erg opened = options_open(options: dat appendable, path: abs path);
    return case dat opened {
        Result.Err(error) => Err(error),
        Result.Ok(file) => {
            erg written = file_write(self: ins file, buffer: abs record);
            erg flushed = file_flush(self: ins file);
            file_close(self: ins file);
            return case dat written {
                Result.Err(error) => Err(error),
                Result.Ok(count) => case dat flushed {
                    Result.Ok(_) => Ok(count),
                    Result.Err(error) => Err(error),
                },
            };
        },
    };
}
```

In production code, preserve and report the close result when the application
needs to distinguish a failed close from a successful write.

## Seek before reading

```actus
open verb seek_to_start(ins file: File) -> Result[Int, IoError] {
    return file_seek_start(self: ins file, offset: 0);
}
```

The returned position is absolute in bytes. A failed seek must be handled
before a subsequent read that depends on the new cursor position.

## Inspect metadata

```actus
open verb file_size(abs path: Path) -> Result[Int, IoError] {
    erg result = metadata(path: abs path);
    return case dat result {
        Result.Ok(info) => Ok(info.size),
        Result.Err(error) => Err(error),
    };
}
```

Metadata is copied scalar information. It does not transfer or retain a file
handle.

## Direct filesystem changes

`rename`, `copy_file`, `remove_file`, `create_dir`, and `remove_dir` all mutate
host state. Keep them behind a named application or persistence operation so
the side effect is visible to the caller.

## Hot-path rule

Do not call filesystem verbs from inference, impulse forwarding, neurogenesis,
coincidence evaluation, or another latency-sensitive path. Use an explicit
save/commit boundary or a separately scheduled background worker.
