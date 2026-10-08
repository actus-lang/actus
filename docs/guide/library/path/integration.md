# `std::path` integration with `std::fs`

`std::path` owns and validates names. `std::fs` performs external operations.
Keep the boundary explicit:

```actus
import std::fs;
import std::path;

verb save(dat raw_path: Buffer, abs payload: Buffer) -> Result[Int, IoError] {
    erg path = path_from_ascii(storage: dat raw_path)?;
    return write_file(path: abs path, contents: abs payload);
}
```

The path is constructed and validated before the filesystem call. The
filesystem verb receives an `abs Path` view and does not need to parse raw
bytes itself.

For repeated access, keep the owned `Path` and use a `File` handle. For a
single bounded operation, use `read_to_bytes`, `read_to_string`, `write_file`,
or `write_file_atomic` from `std::fs`.

Path normalization does not make a filesystem operation safe by itself. The
application remains responsible for authorization, sandbox boundaries, race
handling, and policy around external paths.
