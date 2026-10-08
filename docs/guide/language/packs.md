# Packs and explicit bit layouts

A `pack` is a value whose storage representation is part of its type
contract. It is intended for registers, flags, binary headers, wire metadata,
and other fixed layouts where the position and width of each field matter.

A pack is different from a normal `struct`:

- a `struct` groups named values and uses ordinary aggregate layout;
- a `pack` declares a fixed backing storage, bit offsets, field widths, and
  endianness;
- a `pack` does not grant pointer access or permit unchecked memory writes.

## Declaring backing storage

The backing storage is declared first. It can be a fixed-width integer or a
byte array with a compile-time extent:

```actus
pack ControlRegister {
    erg storage: u32;
    layout little;
    fields {
        erg enabled: u1 at 0;
        abs ready: u1 at 1;
        erg mode: u2 at 2;
        abs _reserved: u28 at 4 = 0;
    }
}
```

For a multi-word representation, use a byte array whose size is fixed in the
type:

```actus
pack CacheLine {
    erg storage: Array[u8, 64];
    layout little;
    fields {
        erg word_0: u128 at 0;
        erg word_1: u128 at 128;
        erg word_2: u128 at 256;
        erg word_3: u128 at 384;
    }
}
```

The array length is the physical storage capacity. It is not runtime capacity
and cannot come from a mutable variable. The element type and extent must
produce a supported fixed byte layout; for example, `Array[u16, 64]` is not a
replacement for `Array[u8, 64]` in a byte-addressed multi-word pack contract.

## Offsets, widths, and coverage

`at` uses a bit offset. `at 0` starts at the least-significant layout position,
and `at 8` starts eight bits later. The field width comes from the declared
field type: `u1` occupies one bit, `u8` occupies eight bits, and an
`Array[u32, 2]` field occupies 64 bits.

The compiler validates the complete layout before native lowering:

- every field must fit within the backing storage;
- fields may not overlap;
- all storage bits must be covered;
- uncovered space must be represented by an explicit reserved field;
- the reserved field's default can document and initialize unused bits;
- runtime values cannot determine storage capacity or field offsets.

For example, this layout covers all eight bits and makes the unused region
explicit:

```actus
pack Flags {
    erg storage: u8;
    layout little;
    fields {
        erg enabled: u1 at 0;
        abs _reserved: u7 at 1 = 0;
    }
}
```

A gap without a reserved field, an overlapping range, or a field extending past
the storage boundary is a semantic error. These checks happen before code
emission, so a malformed layout cannot become a partially valid native type.

## Endianness and field types

`layout little` and `layout big` define how multi-byte values are interpreted
in the backing representation. The choice is part of the pack contract and
must match the external register, file, or protocol specification.

Fields may use supported unsigned or signed fixed-width integer types. A signed
field is extracted with its declared sign behavior; the backing storage itself
still has to use a supported pack storage type. Floating-point backing storage
is not a valid pack storage contract.

Do not infer endianness from the host machine. Declare it in the pack and keep
the declaration next to the field offsets that depend on it.

## Constructing and accessing a pack

Initialize the backing storage explicitly, then access named fields:

```actus
verb read_control() -> Int {
    erg control = ControlRegister { storage: 0u32, };
    control.enabled = 1u8;
    control.mode = 2u8;
    return control.enabled + control.mode + control.ready;
}
```

Field reads and writes are lowered through the declared layout. The compiler
performs the required mask and shift operations and checks the field value
against its declared width. A value that cannot fit the field is rejected by
the type/runtime contract instead of silently corrupting a neighboring field.

An `erg` pack owner can mutate writable `erg` fields. An `abs` pack view can
inspect fields but cannot mutate them:

```actus
verb inspect(abs control: ControlRegister) -> u8 {
    return control.enabled;
}
```

Field roles are part of the access contract. Mark a field `abs` when it is
read-only or reserved for observation. A reserved field is still part of the
layout and must be declared even when application code never changes it.

## Indexed pack fields

A pack field can itself be a fixed array. Its elements retain ordinary bounded
array indexing and ownership rules:

```actus
pack Links {
    erg storage: Array[u8, 8];
    layout little;
    fields {
        erg links: Array[u32, 2] at 0;
    }
}

verb update_link(ins links: Links, abs index: u32) {
    links.links[index] = 41u32;
}
```

Constant indexes outside the declared array capacity are rejected during
semantic analysis. Computed indexes remain bounded native accesses. An `abs`
pack owner or `abs` field cannot be mutated through an index, and an `ins` loan
is scoped to the call that receives it.

Indexed pack fields compose with arrays of packs and nested aggregate places.
The compiler keeps the pack stride, field width, and indexed element bounds in
the layout contract; callers do not calculate byte offsets manually.

## Native representation and ownership

A pack value contains its declared backing storage and field metadata. Named
field access does not allocate, perform filesystem I/O, or require an
operating-system pointer. Native lowering uses the validated fixed layout and
preserves the owner/view role of the containing value.

Do not replace a pack with a raw pointer, cast a buffer into a pack, or duplicate
its shifts and masks in application code. If an external memory region is
needed, keep that boundary in an explicit target or unsafe adapter and expose a
checked pack-facing API.

## Packs and serialization

A pack defines representation; it does not automatically define a file or
transport lifecycle. The declarative `serialize` contract can build fixed
validation, encode, decode, and migration operations around a pack. Those
operations use caller-owned buffers and typed errors. They do not open files,
commit manifests, or perform atomic replacement by themselves.

Read the [serialization reference](reference/15-buffer-array-pack-and-arena.md)
for the accepted fixed-frame contract. Keep filesystem publication and recovery
in the filesystem layer rather than hiding I/O inside pack field access.

## Common layout errors

| Error | Meaning | Correction |
| --- | --- | --- |
| Field out of bounds | The field end is past the backing capacity. | Reduce the field width/offset or enlarge fixed storage. |
| Field overlap | Two bit ranges claim the same storage. | Move one field or redesign the layout. |
| Uncovered bits | A storage region is not represented by a field. | Add an explicit reserved field with a documented default. |
| Invalid storage | The backing type is unsupported or has runtime extent. | Use a supported fixed integer or byte array. |
| Read-only mutation | An `abs` pack or field is written. | Use an owned mutable value or an `ins` loan. |
| Index out of bounds | A pack array field is indexed outside its fixed extent. | Use a checked index within the declared capacity. |

## Related reference material

- [Structs](structs.md) explains ordinary aggregate values.
- [Arrays and buffers](arrays-and-buffers.md) explains fixed indexed storage.
- [Ownership roles](../ownership/roles.md) explains mutable owners and views.
- [Pack and serialization reference](reference/15-buffer-array-pack-and-arena.md)
  contains the compact technical contract.
