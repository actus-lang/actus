# `std::fs` examples

## Read bytes into an owned result

```actus
import std::fs;

open verb load(abs path: Path) -> Result[Buffer, IoError] {
    return read_to_bytes(path: abs path);
}
```

The successful `Buffer` is owned by the caller. A failed read does not expose
the temporary buffer.

## Read validated UTF-8 bytes

```actus
open verb load_text(abs path: Path) -> Result[Buffer, IoError] {
    return read_to_string(path: abs path);
}
```

The returned representation is still a length-delimited `Buffer`; the
filesystem operation has established that its contents are valid UTF-8.

## Write a complete payload

```actus
open verb save(abs path: Path, abs contents: Buffer)
    -> Result[Int, IoError] {
    return write_file(path: abs path, contents: abs contents);
}
```

The source remains borrowed. The result is the provider's byte count.

## Publish through staging

```actus
open verb publish(
    abs final_path: Path,
    dat staging_path: Path,
    abs contents: Buffer,
) -> Result[Int, IoError] {
    return write_file_atomic(
        path: abs final_path,
        staging: dat staging_path,
        contents: abs contents,
    );
}
```

The staging path must be selected by the caller and should share a filesystem
with the final path. On write, count, flush, or rename failure, the operation
attempts to remove staging and returns the original typed error.

## Configure exclusive creation

```actus
open verb create_once(abs path: Path) -> Result[File, IoError] {
    erg initial = options_new();
    erg writable = options_write(dat initial);
    erg exclusive = options_create_new(dat writable);
    return options_open(options: dat exclusive, path: abs path);
}
```

An existing target is rejected by the provider and becomes `IoError.Failed`.

## Copy and rename

```actus
open verb relocate(abs source: Path, abs destination: Path)
    -> Result[Int, IoError] {
    erg copied = copy_file(from: abs source, to: abs destination);
    return case dat copied {
        Result.Err(error) => Err(error),
        Result.Ok(_) => rename(from: abs destination, to: abs source),
    };
}
```

Both paths are borrowed for their respective calls. The application must
choose and document whether the source should remain after the operation.

## Directory lifecycle

```actus
open verb make_directory(abs path: Path) -> Result[Int, IoError] {
    return create_dir(path: abs path);
}
```

`remove_dir` is valid only when the provider permits removing the target,
which normally requires an empty directory.

## Read a caller-owned chunk

```actus
open verb read_chunk(ins file: File, ins buffer: Buffer)
    -> Result[Int, IoError] {
    return file_read(self: ins file, buffer: ins buffer);
}
```

This form keeps storage under application control and is preferable when a
complete file should not be collected into one result buffer.
