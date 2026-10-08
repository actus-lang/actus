# `std::fs` usage

## Import and path preparation

```actus
import std::fs;
import std::path;
```

Filesystem verbs receive validated `Path` values. Build the path first and
then pass it to the file operation; do not pass arbitrary text where a `Path`
is required.

## File handles

Use `file_open` for an existing file and `file_create` for a new writable
handle. A `File` is an owned resource. Read and write through `ins self` and
close it through `file_close` when the handle is no longer needed.

```actus
erg opened = file_open(path: abs path);
case dat opened {
    Result.Ok(file) => {
        erg bytes = Buffer[0];
        reserve(buffer: ins bytes, capacity: 4096)?;
        file_read(self: ins file, buffer: ins bytes)?;
        file_close(self: ins file)?;
    },
    Result.Err(error) => {
        return report_io(error: dat error);
    },
};
```

The real error handling must preserve the concrete `IoError` variants. A
failed open does not produce a usable handle.

## One-shot operations

`read_to_bytes` and `read_to_string` perform a complete path-based read and
return owned data. `write_file` writes a complete borrowed buffer. These are
convenient for bounded files; use a `File` and caller-owned buffer for a
controlled streaming lifecycle.

## Atomic publication

`write_file_atomic` writes to a staging path and publishes it only after the
write and flush succeed. The caller chooses the staging and final paths. On a
failure, the staging cleanup result remains part of the typed operation.

## Operations and side effects

`rename`, `copy_file`, `remove_file`, `create_dir`, and `remove_dir` change the
host filesystem. They are not pure path transformations and must be kept at
an explicit application or persistence boundary.
