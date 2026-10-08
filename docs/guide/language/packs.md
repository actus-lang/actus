# Packs and explicit layouts

A `pack` describes fixed storage, field widths, offsets, and byte order. Use it
for wire frames, registers, file headers, and other data whose representation
is part of the contract.

```actus
pack Header {
    erg storage: Array[u8, 4];
    layout little;
    fields {
        erg version: u8 at 0;
        erg flags: u8 at 8;
        erg length: u16 at 16;
    }
}
```

The storage covers the declared fields. Field offsets and widths must fit the
storage and must not overlap. A pack field's access role follows the owner or
view through which the pack is accessed.

Use checked field access and assignment:

```actus
abs version: u8 = header.version;
header.flags = 1u8;
```

Array-backed pack fields support bounded indexing when the representation is
byte-addressable and the element width is a whole number of bytes. The
compiler checks constant indexes and emits checked runtime access for computed
indexes.

Use the declared layout for endian-sensitive values. Do not use pointer casts or
manual unchecked offsets to bypass pack validation.

Declarative serialization contracts can generate fixed encode, decode,
validation, and migration operations. Their public errors and ownership rules
are described by the relevant standard-library or protocol page.
