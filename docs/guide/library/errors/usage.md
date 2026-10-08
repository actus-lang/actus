# Result-handling patterns

## Propagate a compatible error

```actus
open verb load(abs path: Path) -> Result[Buffer, IoError] {
    erg bytes = read_to_bytes(path: abs path)?;
    return Result[Buffer, IoError].Ok(bytes);
}
```

Use `?` only when the enclosing result can return the same error domain and the
success value has the required type.

## Translate a domain

```actus
open verb load_text(abs path: Path) -> Result[Buffer, StringError] {
    erg loaded = read_to_string(path: abs path);
    return case dat loaded {
        Result.Ok(bytes) => Ok(bytes),
        Result.Err(error) => case dat error {
            IoError.InvalidData => Err(StringError.InvalidUtf8),
            IoError.Failed => Err(StringError.ProviderUnavailable),
            IoError.EndOfStream => Err(StringError.ProviderUnavailable),
            IoError.InvalidInput => Err(StringError.InvalidStorage),
        },
    };
}
```

Translation is an application decision and must document why information can
be safely mapped between domains.

## Inspect an error

Match every public variant that the operation can return. Keep a catch-all
only when the surrounding contract intentionally groups variants and says so.

## Ownership rule

Use `case dat result` when branches consume the result payload. Return or
transfer `Ok` values immediately when possible. Drop temporary values that are
not returned. Never reuse a binding after its ownership was consumed by a
branch.

## Failure is observable

A failed operation may have performed part of its work. Read the operation's
contract for short writes, partial reads, cleanup, publication, and lifecycle
behavior before deciding whether retry is safe.
