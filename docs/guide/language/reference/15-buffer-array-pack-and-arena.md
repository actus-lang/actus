# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 15. `Buffer`, `Array`, `pack`, and `Arena`

### 15.1 `Buffer`

`Buffer` is the primary owned byte container. It has a live length and a
capacity and is used for protocol frames, file data, streams, and raw I/O.

```act
erg buffer: Buffer = Buffer[128];
append(buffer, 0x42u8);
erg length = buffer_length(buffer: abs buffer)?;
```

Buffer indexing is checked against the live range. A zero-length buffer is
valid, but reading or writing an index outside the live range is invalid. Use
the standard-library buffer operations or documented runtime intrinsics for
reserve, append, clear, length, and range operations. Do not assume a null
terminator; binary output uses the live length.

`abs Buffer` reads without consuming or allocating. `ins Buffer` mutates the
caller-owned storage in place. `dat Buffer` transfers cleanup responsibility.

### 15.2 `Array[T, N]`

`Array[T, N]` is contiguous, fixed-capacity storage:

```act
erg values: Array[u32, 4] = Array[u32, 4]();
values[0] = 10u32;
values[1] += 1u32;
```

Indexing accepts `Int`, `Usize`, and unsigned integer indices where the
semantic contract permits. Capacity and runtime bounds are checked before
native load/store. Array slots can participate in exclusive `ins` loans without
copying the entire array.

### 15.3 `pack`

`pack` expresses an explicit storage layout for registers, headers, flags, and
protocol fields. Follow the accepted repository form when adding new code:

```act
pack Register {
    erg storage: u8;
    layout little;
    fields {
        erg value: u8 at 0;
    }
}
```

Pack fields are read and written through field access. The compiler performs
shift/mask lowering, validates field widths and offsets, and preserves the
declared endianness/layout contract. Do not manually duplicate bit shifts when
a pack declaration expresses the actual representation.

### 15.4 Declarative serialization contracts

Fixed-width binary formats may be declared with `serialize` when the byte
layout is part of the type contract:

```act
serialize Frame from FramePack {
    layout little;
    version u16 at 0;
    payload bytes at 2 length 16;
    checksum crc32 over 0 .. 18 at 18;
}
```

The first accepted profile requires exactly one `version u16`, one fixed-size
payload, and one `crc32` section. Offsets, lengths, checksum ranges, and the
source pack's byte capacity are validated at compile time. Sections may not
overlap, and the checksum field may not overlap its input range. The
declaration does not allocate memory, open files, or perform I/O. Generated
read, write, validation, and migration operations must use caller-owned
buffers and explicit ownership roles. The `crc32` intrinsic computes an IEEE
CRC32 over a validated buffer range, while `crc32_matches` compares that value
with an expected integer. The `validate_fixed_frame` intrinsic exposes the
fixed-frame runtime validator, which checks version,
payload bounds, endianness, and stored CRC without allocation. These operations
do not allocate. Generated serialization operations use the compiler-provided
`SerializationError` enum. The compiler currently generates
`<contract>_validate`, which accepts a caller-owned `Buffer` and a read-only
expected version, and `<contract>_encode`, which accepts an `abs` source pack
and an `ins` output `Buffer`, copies the fixed storage bytes, and returns a
typed `Result[u32, SerializationError]`. It also generates
`<contract>_decode`, which accepts an `abs expected_version: u16`, checks the
input length through the compiler-provided `buffer_length` primitive, validates
the encoded version and checksum, and returns an owned pack or a typed
`InvalidLayout`, `InvalidVersion`, or `InvalidChecksum` error. Both operations
derive their fixed storage bounds from the declaration. The compiler also
generates `<contract>_migrate`, which validates `from_version`, copies the fixed
frame into an empty caller-owned output, writes `to_version`, recomputes the
declared checksum, and returns a typed byte count. Dynamic payloads, implicit
allocation, and automatic filesystem commits remain outside this profile until
their contracts are implemented and tested.

When generated migration is available, use the explicit form
`<contract>_migrate(abs input: Buffer, abs from_version: u16,
abs to_version: u16, ins output: Buffer)`. It validates the source version,
rewrites the target version, recomputes the declared checksum, and writes into
the caller-owned output. The output must be empty before migration. Migration
does not perform filesystem I/O; atomic file replacement belongs to the
filesystem persistence layer.

### 15.5 `Arena[N]`

The native backend contains bounded arena support used by current aggregate
and systems examples:

```act
erg arena: Arena[64] = Arena[64]();
erg slot = arena.place(value: Array[Int, 2]());
```

Arena capacity and cleanup are explicit. Treat this as a bounded storage
primitive, not a garbage collector or an unbounded heap. Verify the exact
constructor and method contract against current examples before using it in a
new public API.
