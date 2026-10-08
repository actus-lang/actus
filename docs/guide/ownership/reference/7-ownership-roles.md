# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 7. Ownership roles

### 7.1 `erg`: active owner

`erg` declares the binding that owns a resource or value and may mutate it.

```act
erg packet: Buffer = Buffer[128];
append(packet, 0x42);
```

An `erg` binding may be read, mutated, borrowed, passed by explicit role,
moved into another owner, or dropped. It is the normal local role for mutable
state.

### 7.2 `abs`: read-only view

`abs` creates or accepts a non-owning read-only view:

```act
verb checksum(abs packet: Buffer) -> Int {
    return length(packet);
}
```

The view does not consume the source, cannot mutate it, and cannot outlive its
origin. An owner with an active `abs` borrow is frozen until the borrow ends.

Explicit borrow construction uses `ref`:

```act
abs view = ref packet;
```

### 7.3 `dat`: terminal transfer

`dat` transfers ownership into the callee:

```act
verb consume(dat packet: Buffer) -> Int {
    erg size = length(packet);
    drop(packet);
    return size;
}

erg packet = Buffer[16];
consume(packet: dat packet);
```

After the transfer the caller binding is `Moved` and cannot be read, mutated,
borrowed, or dropped again. The receiving scope becomes responsible for
cleanup.

### 7.4 `ins`: exclusive call-scope loan

`ins` temporarily gives a callee exclusive mutation access while the caller
retains ownership after the call:

```act
verb append_marker(ins packet: Buffer) -> Result[Int, IoError] {
    append(packet, 0x7f);
    return Result[Int, IoError].Ok(1);
}

erg packet = Buffer[16];
append_marker(packet: ins packet)?;
append(packet, 0x01);
```

During the call the source owner is suspended. It cannot be read, moved,
borrowed, or passed to another exclusive loan. After the call it returns to an
active mutable state. `ins` is a parameter/call role, not persistent struct
storage. An `ins` field is rejected because a call-scoped loan cannot be
stored as aggregate state.

### 7.5 Ownership state transitions

An agent must reason about these states:

```text
Active erg -> Frozen while abs borrow exists -> Active after borrow ends
Active erg -> Suspended during ins call       -> Active after call returns
Active erg -> Moved after dat transfer
Active erg -> Dropped after drop or scope cleanup
```

Illegal transitions include mutation of a frozen owner, use of a suspended
owner, use after move, use after drop, aliased `ins`, double cleanup, and a
borrow escaping its origin.

### 7.6 Cleanup

Owned values are cleaned in deterministic reverse declaration order. Cleanup
also occurs before `return`, `break`, `continue`, and `?` transfer control out
of the affected scope. Moved fields and explicitly dropped fields are removed
from the cleanup plan exactly once.

Do not insert manual cleanup to work around the compiler. First model the
ownership role correctly. Use `drop` only when an early, intentional release
is part of the operation.
