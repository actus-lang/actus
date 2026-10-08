# Standard-library errors and `Result`

Every fallible standard-library operation returns a typed `Result`. The `Ok`
payload is the operation's value or byte count. The `Err` payload identifies a
specific failure domain such as `IoError`, `PathError`, `TimeError`,
`RegionError`, or `WireError`.

Use `?` when the enclosing verb returns the same compatible error type. Use
`case dat value` when branches need to inspect or translate the error.

```actus
verb load(abs path: Path) -> Result[Buffer, IoError] {
    erg bytes = read_to_bytes(path: abs path)?;
    return Result[Buffer, IoError].Ok(bytes);
}
```

Do not convert an error into a boolean when the caller needs to distinguish
capacity, corruption, invalid input, end-of-stream, or lifecycle failure.
