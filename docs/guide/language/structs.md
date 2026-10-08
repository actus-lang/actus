# Structs

A struct groups named fields into one value. Fields have declared types and
participate in ordinary ownership and cleanup rules.

```actus
struct Point {
    erg x: u32;
    erg y: u32;
}
```

Construct and access a struct with named fields:

```actus
erg point: Point = Point {
    x: 10u32,
    y: 20u32,
};
point.x += 1u32;
```

A mutable owner can update its fields. An `abs` view can inspect fields but
cannot mutate them. Moving an owned field affects the cleanup state of the
containing struct.

Structs can have methods through `perform`. The receiver role is part of the
method contract. A method that mutates the receiver needs the appropriate
mutable or instrumental role; a read-only method uses a shared view.

Struct layout and return behavior are checked by the compiler. Do not replace a
struct with an untyped buffer or manually copied bytes unless the API explicitly
requires a binary representation; use `pack` for that representation.
