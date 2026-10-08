# `erg`: active owner

`erg` declares the active owner of a value or resource.

```actus
erg counter: u32 = 0u32;
counter += 1u32;
```

An `erg` binding may be changed when its type and current ownership state allow
that operation. It remains responsible for deterministic cleanup at the end of
its scope unless it is moved, returned, or otherwise transferred according to
its contract.

An owner can create a read-only `abs` view for a nested call. It cannot be
mutated while a conflicting view or loan remains active.

Use `erg` for local state, parser state, output storage, and other values whose
lifetime and mutation belong to the current scope.
