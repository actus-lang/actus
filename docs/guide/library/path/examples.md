# `std::path` examples

## Construct and inspect a POSIX path

```actus
import std::path;

verb file_length(dat raw: Buffer) -> Result[Int, PathError] {
    erg path = path_from_posix(storage: dat raw)?;
    erg name = file_name(self: abs path);
    return case dat name {
        Option.Some(component) => Result[Int, PathError].Ok(component.length),
        Option.None => Result[Int, PathError].Err(PathError.InvalidLength),
    };
}
```

The component is a borrowed view and is used before its source path leaves the
scope.

## Join two owned paths

```actus
verb child_path(dat root_storage: Buffer, dat child_storage: Buffer) -> Result[Path, PathError] {
    erg root = path_from_ascii(storage: dat root_storage)?;
    erg child = path_from_ascii(storage: dat child_storage)?;
    return join(self: dat root, other: abs child);
}
```

The source buffers move into their respective path values. `join` consumes the
left path and returns the combined owned path.

## Mutate a path in place

```actus
verb rename_file(ins path: Path, abs name: Path, abs extension: Path) -> Result[Int, PathError] {
    set_file_name(self: ins path, name: abs name)?;
    return set_extension(self: ins path, extension: abs extension);
}
```

The receiver remains owned by the caller after each `ins` call.

## Platform classification

```actus
verb classify(abs path: Path) -> Result[PosixRoot, PathError] {
    return posix_root(abs path);
}
```

Use `windows_root` for a Windows path. Do not call the POSIX classifier on a
Windows representation.
