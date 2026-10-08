# Error ownership

An error enum is an Actus value with an ownership state. It is not metadata
that can be inspected indefinitely after a `case dat` branch consumes it.

## Consume and translate

```actus
return case dat error {
    IoError.EndOfStream => Result[Buffer, IoError].Err(IoError.EndOfStream),
    IoError.Failed => Result[Buffer, IoError].Err(IoError.Failed),
    IoError.InvalidInput => Result[Buffer, IoError].Err(IoError.InvalidInput),
    IoError.InvalidData => Result[Buffer, IoError].Err(IoError.InvalidData),
};
```

Exhaustive translation keeps the caller's error domain explicit. A helper that
changes the success type must consume the original error and construct the new
`Result` in the same branch.

## Cleanup before return

A failing operation may own temporary storage. Drop that storage before
returning the typed error. This applies to file read buffers, staging paths,
reassembly storage, and rejected UTF-8 buffers.

## No hidden rollback

Typed error propagation does not roll back external effects. A caller that
needs atomic publication must use the module's explicit staging/commit
operation rather than assuming that returning `Err` restores the previous
state.

## Preserve detail

Do not collapse `CapacityExceeded`, `InvalidData`, `EndOfStream`, `Stale`,
`Overflow`, and `ProviderUnavailable` into one boolean. Those variants tell
the caller whether to retry, resize, discard input, wait, reset context, or
select another provider.

## Public boundary

Raw ABI status values belong only in private bridge wrappers. The public
function's documentation is the authority for which typed variants can be
returned and which owners remain live on each branch.
