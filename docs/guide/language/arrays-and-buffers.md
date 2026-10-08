# Arrays and buffers

## Bounded arrays

`Array[T, N]` is fixed-capacity inline storage. `N` is a compile-time extent;
it is not runtime capacity and does not allocate an unbounded collection.

```actus
erg values: Array[u32, 4] = Array[u32, 4]();
values[0] = 10u32;
values[1] = values[0] + 1u32;
```

Constant indexes outside the array are rejected during semantic analysis.
Computed indexes retain runtime bounds checks. The array's ownership role is
preserved through indexed access and instrumental loans.

Arrays can contain supported scalar, struct, pack, and other fixed-size element
types. The element must have a known native layout.

## Buffers

`Buffer` is caller-owned byte storage with a bounded current length and
capacity. Use `abs Buffer` for inspection and `ins Buffer` for exclusive
in-place mutation.

```actus
verb append_marker(ins output: Buffer) -> Result[Int, BufferError] {
    append(output: ins output, value: 0x2Au8)?;
    return Result[Int, BufferError].Ok(1);
}
```

The exact public operation names and errors come from the standard-library
facade being used. Do not assume a buffer operation allocates or grows storage
unless its API says so.

## Indexed access

Indexed arrays, buffer bytes, and array-backed pack fields use checked access.
An `abs` owner cannot be mutated through an index. An `ins` indexed loan is
scoped to the call and restores the original owner afterward.
