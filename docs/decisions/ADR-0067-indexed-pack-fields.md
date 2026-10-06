# ADR-0067: Indexed Fields in Packed Values

- Status: Accepted and implemented
- Date: 2026-10-05
- Decision owners: Actus language and compiler maintainers

## Context

Packed values currently support named fields at explicit bit offsets. A
program that stores a fixed sequence of equal-width values therefore has to
declare and access each element as a separate field. That representation is
correct but makes bounded traversal repetitive and makes the relationship
between the repeated values and their byte layout difficult to read.

Actus already has a bounded `Array[T, N]` type with checked indexing. The
language needs the same bounded operation inside a packed value while
preserving the pack's explicit layout, ownership rules, and native
representation.

## Decision

A packed declaration may use a fixed array as a field type:

```act
pack Example {
    erg storage: Array[u8, 32];
    layout little;
    fields {
        erg links: Array[u32, 8] at 192;
    }
}
```

The field describes eight contiguous `u32` elements beginning at bit offset
192. Its total occupied width is the element width multiplied by the fixed
count. The declaration remains part of the pack coverage and overlap checks;
it does not create a dynamic collection or a second allocation.

An indexed access uses the existing bounded array operation:

```act
abs value: u32 = example.links[index];
example.links[index] = next_value;
example.links[index] += 1u32;
```

The semantic layer records the total field width and the fixed element count.
It rejects invalid element types, malformed counts, overflowed widths,
constant indexes outside the declared range, and writes through immutable
pack views. Runtime indexes use the existing checked array bounds path.

## Native representation

The native backend treats an indexed packed field as a byte-addressable view
of the pack storage. The element address is calculated from the declared base
offset and the element's native byte width, then the ordinary array load or
store is used. This is valid only when the packed field is byte-aligned and
the element width is a whole number of bytes. Unsupported representations
produce a compiler diagnostic instead of falling back to unchecked pointer
arithmetic.

Scalar packed fields retain their existing bit extraction and update path.
Adding an indexed field does not change the pack's storage size, byte order,
alignment, serialization bytes, ownership roles, or ABI. No dynamic
allocation, reflection, raw operating-system pointer, or hidden aggregate
copy is introduced.

## Compatibility

Named scalar fields remain supported. This decision does not generate aliases
for individual array elements and does not change existing source spellings.
Migration from repeated named fields is therefore explicit: the declaration
must be rewritten as one fixed array field and callers must use a checked
index.

## Verification

The implementation includes semantic tests for accepted indexed reads and
writes, immutable mutation rejection, constant bounds rejection, and
non-truncated metadata for a 256-bit field. Native tests cover indexed reads,
writes, and compound updates. The full compiler suite passes with:

```text
cargo test --all --locked
```
