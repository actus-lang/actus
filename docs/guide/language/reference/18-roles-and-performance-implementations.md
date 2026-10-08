# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 18. Roles and performance implementations

A role is a compile-time callable contract:

```act
open role Reader {
    read(ins buffer: Buffer) -> Result[Int, IoError];
}
```

An implementation associates the role with a concrete type:

```act
perform Reader for Cursor {
    open verb read(ins buffer: Buffer) -> Result[Int, IoError] {
        return cursor_read(self: ins self, buffer: ins buffer);
    }
}
```

Role methods must preserve ownership roles and return contracts. Static role
dispatch is the normal embedded-friendly path. Dynamic dispatch is an explicit
runtime contract and must not be introduced just to avoid a generic type error.
