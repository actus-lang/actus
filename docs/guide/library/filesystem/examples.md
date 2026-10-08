# `std::fs` examples

## Read a bounded file

```actus
import std::fs;
import std::path;

verb load(dat storage: Buffer) -> Result[Buffer, IoError] {
    erg path = path_from_ascii(storage: dat storage)?;
    return read_to_bytes(path: abs path);
}
```

The path constructor consumes the input storage. The returned byte buffer is
owned by the caller of `load`.

## Write bytes

```actus
verb save(abs path: Path, abs contents: Buffer) -> Result[Int, IoError] {
    return write_file(path: abs path, contents: abs contents);
}
```

The operation reads the path and contents through `abs`; it does not consume
either value.
