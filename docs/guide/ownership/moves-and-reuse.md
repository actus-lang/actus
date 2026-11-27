# Moves and scalar reuse

A move transfers ownership and invalidates the previous owner for the moved
resource. The compiler rejects later use of the moved binding.

A move may happen through a `dat` call, a returned owned value, an assignment,
or a payload operation. Fields and indexed places are tracked as part of their
containing owner.

Every struct, every enum and every pack is moved by these operations, whether it holds a
resource or only plain data (ADR-0124). A binding, an assignment, a struct
field initializer, an enum payload, an `erg` or `dat` argument and a return
take the value away from the source, and the compiler rejects a later use with
`E1005`. A read leaves the source usable: an operand, a comparison, a field
read such as `header.payload_len`, an `abs` argument and a `case` on an `abs`
subject do not move it.

## Scalar reuse

Eligible scalar values can be reused explicitly:

```actus
verb read(abs value: u32) -> Int {
    return value as Int;
}

verb main() -> Int {
    erg value: u32 = 41u32;
    return read(value: abs value);
}
```

Use `copy(value: abs value)` only for a scalar or for a struct, an enum or a pack
declared [`meta copy`](../meta-attributes/copy.md). It does not make buffers,
strings, resources, or cleanup-bearing aggregates copyable. Identity
arithmetic such as `value + 0u32` is not a substitute for explicit reuse.

## Struct, enum and pack reuse

A struct of plain data (an enum or a pack in the same way) is moved like any other owner. To keep the source, the
type declares `meta copy` and the duplication is written:

```actus
meta copy
struct Header {
    kind: u8,
    payload_len: u16,
}

struct Parser {
    header: Header,
}

verb main() {
    erg header = Header { kind: 1u8, payload_len: 12u16, };
    erg parser = Parser { header: copy(value: abs header), };
    print(int: header.payload_len);
}
```

Without the `copy`, the struct field initializer moves `header` and the read
that follows is `E1005`. When the later read is only a field value, read it
before the move instead of copying.

## Moves inside a loop

A loop body runs again, so a value that is declared before the loop and moved or
dropped inside it is moved a second time by the next iteration. The compiler
rejects the move with `E1005` at the move itself, and the message says that the
value was moved in a previous iteration. This holds for `loop`, for `for` over a
range or an array, for nested loops, and for every kind of move: a `dat` or
`erg` argument, a struct field or enum payload, an `erg` binding, a field of an
owner and `drop`:

```actus fails=E1005
verb finish(dat data: Buffer) {
    drop(data);
}

verb main() {
    erg data: Buffer = Buffer[4];
    for erg index in 0..3 {
        finish(data: dat data);
    }
}
```

Two shapes are valid. A move on a path that leaves the loop, with `break` or
`return` after it, never reaches the next iteration. A value that is declared
inside the body is a new owner in every iteration:

```actus
verb finish(dat data: Buffer) {
    drop(data);
}

verb main() {
    erg data: Buffer = Buffer[4];
    erg count: Int = 0;
    loop {
        count += 1;
        if count > 2 {
            finish(data: dat data);
            break;
        }
    }
    for erg index in 0..3 {
        erg fresh: Buffer = Buffer[4];
        finish(data: dat fresh);
    }
}
```

A move before a `continue` reaches the next iteration, so it is rejected like a
move at the end of the body. A moved value cannot be assigned a new value, so
re-initialising it inside the loop is not a way out; create the value in the
loop instead.

A value that is borrowed or moved cannot be reused until the relevant loan or
ownership transition has ended.
