# Structs and aggregate values

A `struct` combines named fields into one typed value. Use it when the fields
belong together as one record: a configuration, a state object, a protocol
model, or a node in a data structure.

A struct describes the value's fields. It does not by itself allocate a heap
object or define a serialization format. Use [`pack`](packs.md) when byte
positions and endianness are part of the contract.

## Declaring and constructing a struct

Fields use ordinary Actus types. Ownership roles are written on fields that
carry an ownership or view contract; scalar fields usually need no role marker:

```actus
struct Point {
    x: u32,
    y: u32,
}

verb main() -> Int {
    erg point: Point = Point {
        x: 10u32,
        y: 20u32,
    };
    point.x += 1u32;
    return point.x as Int;
}
```

A struct literal names its fields. Construction requires the fields needed by
the declared type, and field expressions must have compatible types. Access
uses `.`, including access through nested structs and indexed aggregate values.

## Field roles

A field that owns a resource can be declared with `erg`. An `abs` field keeps a
read-only view and does not own the value it refers to. `ins` is a temporary
call-scoped loan and is not a persistent struct-field role. `dat` describes a
transfer at an operation boundary rather than a stored field:

```actus
struct PacketState {
    erg payload: Buffer,
    abs header: Buffer,
}
```

The owner of `payload` is responsible for its cleanup through the containing
struct. `header` can be inspected through the struct but cannot be mutated or
freed through that view. See [ownership roles](../ownership/roles.md) for the
full role rules.

## Mutation, views, and field moves

An `erg` owner can mutate its fields. An `abs` view can read fields but cannot
mutate them:

```actus
verb read_point(abs point: Point) -> u32 {
    return point.x + point.y;
}

verb update_point(ins point: Point) {
    point.x += 1u32;
}
```

Passing a struct with `ins` lends the existing owner for the duration of the
call. The caller keeps the owner after the call and observes the mutation.
Passing a field to a `dat` parameter moves that field out of the aggregate:

```actus
struct Outer {
    erg first: Buffer,
    erg second: Buffer,
}

verb consume(dat value: Buffer) {
    drop(value);
}

verb use_one_field() -> Int {
    erg outer: Outer = Outer {
        first: Buffer[1],
        second: Buffer[2],
    };
    consume(value: outer.first);
    consume(value: outer.second);
    return 0;
}
```

The move is tracked at the field place. It is not an implicit byte copy. A
resource-bearing field cannot be used again after it has moved, while unrelated
fields retain their own ownership state.

Whole-struct assignment and transfer follow the same ownership rules. A
resource-bearing aggregate is not silently duplicated just because its fields
are laid out together.

## Cleanup and custom drop behavior

Struct cleanup is planned at scope boundaries and on return or error paths. If
a type has a `Drop` performance implementation, the compiler uses its
`ins` receiver as the deterministic cleanup operation:

```actus
struct Counter {
    value: Int,
}

perform Drop for Counter {
    verb drop(ins self: Counter) {
        print(self.value);
    }
}
```

The receiver must be exclusive. The compiler rejects dropping an owner while
it is frozen by an `abs` view or while it is borrowed by an `ins` call. If the
owner is transferred through `dat`, the original binding is removed from its
cleanup plan because the callee now owns it.

Do not call `drop` to bypass a live view or loan. End the view or loan first,
or change the operation boundary so the lifetime is correct.

## Generic structs

A struct can be parameterized by a type or a bounded compile-time value:

```actus
struct Box[T] {
    erg item: T,
    label: Int,
}

verb main() -> Int {
    erg boxed: Box[Int] = Box[Int] {
        item: 41,
        label: 1,
    };
    return boxed.item + boxed.label;
}
```

`Box[Int]` and `Box[Option[Int]]` are separate checked instantiations. The
compiler validates their field types and layout before native lowering. A
const-generic struct uses the declared domain, such as `N: Usize`, and the
concrete argument must satisfy that domain before it is used in an `Array` or
other fixed layout.

Generic parameters do not erase ownership. If `T` contains an owned resource,
its containing struct still follows normal move and cleanup rules.

## Structs in arenas

Structs with a known native layout can be placed in an `Arena[N]`. Recursive
structures commonly use `Option[abs Node]` fields for child links. The arena
controls the lifetime of the placed values; a view cannot outlive the arena
owner. See [arenas and bounded placement](arenas.md) for placement, capacity,
and reference-lifetime examples.

## Roles and `perform`

A `role` declares a callable contract. A `perform` block supplies that
contract for a concrete struct:

```actus
role Writer {
    verb write(abs self: File) -> Int;
}

struct File {
    value: Int,
}

perform Writer for File {
    verb write(abs self: File) -> Int {
        return self.value;
    }
}
```

The implementation must match the role method's name, receiver role,
parameter types, parameter roles, return type, and failure type. Static
performance dispatch is selected from the concrete type. A role implementation
does not grant permission to mutate through an `abs` receiver.

Public roles and performances must be reachable through the canonical module
facade and must carry the required Actus documentation blocks. Keep module
visibility separate from struct layout; making a type `open` does not change
its ownership semantics.

## What a struct does not provide

A struct does not automatically provide:

- a serialized byte layout;
- a dynamic collection or unbounded capacity;
- a garbage-collected lifetime;
- implicit copying of owned fields;
- permission to use a field after it has moved.

Use the type, ownership role, `pack`, `Array`, or `Arena` contract that matches
the actual requirement.

## Related reference material

- [Struct reference](reference/9-structs-and-aggregates.md) gives the compact
  syntax and aggregate rules.
- [Ownership roles](../ownership/roles.md) explains field and parameter roles.
- [Cleanup and scope](../ownership/cleanup-and-scope.md) explains deterministic
  destruction.
- [Generics](generics.md) explains type and const-generic parameters.
- [Arena placement](arenas.md) explains bounded recursive structures.
