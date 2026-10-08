# `ins`: exclusive call-scoped loan

`ins` gives a callee exclusive mutable access to an existing owner for the
call. The owner is restored when the call returns, including a typed failure
path where the contract permits restoration.

```actus
verb fill(ins output: Buffer) -> Result[Int, BufferError] {
    append(output: ins output, value: 0x2Au8)?;
    return Result[Int, BufferError].Ok(1);
}
```

Only one conflicting `ins` loan may be active for a place. An `ins` loan cannot
be created from an `abs` view, stored in a longer-lived value, returned, or
passed as an owning `erg` or terminal `dat` argument.

Use `ins` for caller-provided buffers, in-place parser state, bounded output,
and other APIs that must mutate existing storage without replacing its owner.
