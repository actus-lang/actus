# `std::path` integration with `std::fs`

`std::path` owns representation and lexical validation. `std::fs` performs
external operations. Keep the boundary explicit:

```actus
import std::fs;
import std::path;

open verb save(dat raw_path: Buffer, abs payload: Buffer)
    -> Result[Int, IoError] {
    erg path_result = path_from_ascii(storage: dat raw_path);
    return case dat path_result {
        Result.Err(_) => Err(IoError.InvalidInput),
        Result.Ok(path) => write_file(path: abs path, contents: abs payload),
    };
}
```

The path is constructed and validated before the filesystem call. Filesystem
verbs receive an `abs Path` view and do not parse raw bytes themselves.

For repeated access, keep the owned `Path` and use a `File` handle. For one
bounded operation, use `read_to_bytes`, `read_to_string`, `write_file`, or
`write_file_atomic`.

Normalization does not make an external path safe by itself. Authorization,
sandbox boundaries, symlink/race policy, and persistence publication remain
application responsibilities.
