# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 9. Structs and aggregates

### 9.1 Struct declaration

```act
struct Point {
    x: Int,
    y: Int,
}
```

Fields may be value fields or ownership-role fields:

```act
struct PacketState {
    erg payload: Buffer,
    abs header: Buffer,
}
```

`erg` fields are owned mutable subresources and participate in aggregate
cleanup. `abs` fields are non-owning read-only views. `ins` fields are not
allowed. `dat` is used at operation boundaries, not as persistent field state.

### 9.2 Struct construction and access

```act
erg point = Point { x: 10, y: 20 };
erg x = point.x;
point.x = x + 1;
```

A move-only source in a struct literal field, whether the field is `erg` or a
plain value field, is moved into the new aggregate and cannot be used again.

Field access uses `.`. Field assignment is ownership-checked. Partial field
moves are tracked. Whole-struct assignment transfers ownership rather than
implicitly copying a resource-bearing aggregate.

### 9.3 Struct methods through `perform`

```act
role Writer {
    write(abs bytes: Buffer) -> Result[Int, IoError];
}

perform Writer for DeviceWriter {
    open verb write(abs bytes: Buffer) -> Result[Int, IoError] {
        ...
    }
}
```

The role declaration is the contract. The `perform` block supplies methods for
the target type. The implementation must satisfy parameter roles, types,
return types, visibility, and dispatch rules.
