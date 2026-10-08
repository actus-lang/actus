# Verbs, roles, and performance contracts

A `verb` is an Actus operation. Its declaration defines the name, parameter
roles, parameter types, return type, failure type, and body behavior. A `role`
is a named callable contract, and a `perform` block implements that contract
for a concrete type.

## Ordinary verbs and call roles

The role is part of every parameter contract:

```actus
verb inspect(abs value: u32) -> Bool {
    return value == 0u32;
}

verb increment(ins value: u32) -> u32 {
    value += 1u32;
    return value;
}

verb consume(dat value: Buffer) {
    drop(value);
}
```

`abs` receives a read-only view, `ins` receives an exclusive temporary loan,
and `dat` transfers ownership to the callee. The caller must provide an
argument whose type and ownership state satisfy that contract. The role can be
made explicit at the call site:

```actus
erg count: u32 = 0u32;
inspect(value: abs count);
increment(value: ins count);
```

Role inference is intentionally narrow. The compiler may infer a compatible
role for simple scalar calls in the supported cases, but aggregate values,
buffers, external calls, generic boundaries, dynamic calls, and ownership
transfers should keep the role explicit. Do not rely on inference to hide a
lifetime or mutation boundary.

## Declaring a role

A role lists the methods a conforming type must provide. Each method has an
explicit receiver parameter; `self` is an ordinary parameter name, not a
special keyword:

```actus
struct File {
    value: Int,
}

role Writer {
    verb write(abs self: File) -> Int;
}
```

The receiver's role is part of the contract. A role method with `abs self` may
inspect the receiver. An `ins self` receiver may temporarily mutate it. A
`dat self` receiver consumes the receiver and ends the caller's ownership after
the call. The method's other parameter roles and its return type are checked in
the same way as an ordinary verb.

A role may be empty when it is intentionally used as a marker contract. Role
names and method names must be unique within their declaration.

## Implementing a role with `perform`

A `perform` block associates a role with one concrete type:

```actus
struct File {
    value: Int,
}

role Writer {
    verb write(abs self: File) -> Int;
}

perform Writer for File {
    verb write(abs self: File) -> Int {
        return self.value;
    }
}

verb main() -> Int {
    erg source = File { value: 41, };
    abs view = ref source;
    return view.write();
}
```

The implementation must match the role method name, receiver type, receiver
role, parameter names/types/roles, return type, and failure contract. Every
required method must be implemented. A missing method, unknown role, duplicate
performance, or receiver mismatch is a semantic error.

The call `view.write()` is resolved from the concrete type and its `Writer`
performance. Static role dispatch is the normal path: the compiler records the
reachable implementation and lowers a direct, deterministic native call.

## Receiver ownership and cleanup

The receiver role determines what happens to the caller's value:

```actus
role Consumer {
    verb consume(dat self: File);
}

perform Consumer for File {
    verb consume(dat self: File) {
        drop(self);
    }
}

verb use_file() {
    erg file = File { value: 1, };
    file.consume();
    # `file` is moved and cannot be used here.
}
```

An `ins self` performance cannot be called while the same owner is frozen or
already loaned. A `dat self` performance removes the owner from the caller's
remaining cleanup/use state. These are the same ownership rules used by free
verbs; `perform` does not create an exception to them.

For deterministic resource cleanup, the built-in `Drop` performance requires
an exclusive receiver:

```actus
perform Drop for File {
    verb drop(ins self: File) {
        print(self.value);
    }
}
```

The compiler schedules this cleanup at scope exit, return, error propagation,
or another ownership boundary. It rejects cleanup while a view or loan remains
live.

## Generic role bounds

A generic declaration can require a role:

```actus
struct Adapter[Source: Writer] {
    erg source: Source,
}

verb flush[Source: Writer](ins adapter: Adapter[Source]) -> Int {
    return adapter.source.write();
}
```

A concrete `Source` is accepted only when it has a matching `perform Writer`
implementation. Multiple role bounds use `+`. Bounds are checked during
specialization and preserve the receiver and argument roles of each method.
See [generics](generics.md) for type and const-generic rules.

## Dynamic role dispatch

Static dispatch is the default. Dynamic dispatch is an explicit boundary when
the program needs to carry a role value whose concrete type is selected at
runtime:

```actus
verb send(dynamic writer: Writer) -> Int {
    return writer.write();
}
```

A dynamic role value requires a matching performance implementation and uses
the compiler's documented dynamic-call ABI. It is not a workaround for a
failed generic bound, a missing `perform`, or a role mismatch. Keep dynamic
boundaries narrow and document their runtime and allocation behavior when they
are part of a public API.

## Visibility and facades

Roles, performances, and their methods are private unless exposed through the
canonical module facade. `open` controls whether a declaration can cross that
facade; it does not change ownership or dispatch:

```actus
open role Reader {
    verb read(ins self: Input, ins buffer: Buffer) -> Result[Int, IoError];
}
```

A public performance must be reachable through the same facade and must satisfy
the public-documentation contract. Do not expose a private child declaration
through a direct path that bypasses its parent facade.

## Common contract errors

| Error | Meaning | Correction |
| --- | --- | --- |
| Missing role method | The performance omits a required method. | Implement every declared method. |
| Role method mismatch | Receiver role, parameter, return, or failure type differs. | Match the role declaration exactly. |
| Unknown role | `perform` names a role that is not declared. | Declare/import the role through the proper facade. |
| Invalid receiver | A role method has no explicit receiver. | Add `self` with the required role and type. |
| Invalid argument role | The call crosses the boundary with the wrong ownership state. | Use the correct view, loan, or transfer. |
| Constraint mismatch | A generic argument has no required performance. | Add the performance or choose another type. |

Do not add a wrapper that copies a value or hides a role mismatch. Correct the
role declaration, `perform` implementation, module facade, or call boundary
where the contract is actually owned.

## Public verb and role documentation

Public verbs, roles, and performance methods should explain their purpose,
inputs, receiver and argument roles, return value, typed failures, ownership
transfer/restoration, cleanup, allocation, I/O, bounds, and invariants. Keep
those details in the declaration's Actus documentation block so the contract
travels with the source.

## Related reference material

- [Ownership roles](../ownership/roles.md) explains `erg`, `abs`, `dat`, and
  `ins` at ordinary call boundaries.
- [Structs](structs.md) explains aggregate receivers and field cleanup.
- [Generics](generics.md) explains role-bounded generic declarations.
- [Roles and performances reference](reference/18-roles-and-performance-implementations.md)
  records the compact compiler contract.
- [Module facades](../modules/canonical-facades.md) explains public exposure.
