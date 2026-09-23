# ADR-0014: Roles and Performance as Contract Abstractions

- Status: Accepted
- Date: 2026-09-23

## Context

Actus needs a contract abstraction for reusable behavior, generic bounds, and
future polymorphism. The terms `trait`, `impl`, and `interface` do not express
the language's role-based model and would introduce terminology that is
separate from `verb`, `erg`, `abs`, and `dat`.

The implementation model must also distinguish compile-time specialization
from runtime dispatch. Static dispatch is the default because it preserves
zero-cost abstractions and deterministic ownership analysis. Runtime dispatch,
when eventually supported, must be explicit and must not be implied by a
contract implementation.

## Decision

Actus adopts `role` and `perform` as its contract vocabulary.

### Role contracts

`role` declares a behavioral contract and its required verb signatures:

```act
role Writer {
    verb write(abs self, abs bytes: Buffer): Int;
}
```

Role contracts may specify receiver roles. The receiver role is part of the
contract and follows the same ownership rules as ordinary calls.

### Compile-time performance

`perform Role for Type` supplies the compile-time realization of a role:

```act
perform Writer for File {
    verb write(erg self, abs bytes: Buffer): Int {
        return 0;
    }
}
```

`perform` is exclusively a static-dispatch mechanism. The compiler resolves
the selected performance, monomorphizes reachable generic instances, and
lowers the resulting direct calls. A `perform` declaration does not create a
runtime vtable or a fat pointer.

### Role bounds

Generic constraints use role requirements:

```act
verb emit[T: Writer](dat item: T): Int {
    return item.write(bytes: Buffer[0]);
}
```

The bound is checked during type resolution and monomorphization. It must not
silently change a call into runtime dispatch.

### Explicit dynamic dispatch

Runtime polymorphism uses the complete `dynamic` keyword and is separate from
`perform`:

```act
verb send(abs writer: dynamic Writer, abs bytes: Buffer): Int {
    return writer.write(bytes: bytes);
}
```

`abs dynamic Role` denotes an explicit role object represented by a fat pointer
and vtable. Its ABI, object layout, and lifetime rules will be specified before
the feature is implemented. `dynamic` is not an alias for `perform`, and a
role cannot become dynamic accidentally.

## Static and Dynamic Dispatch Boundary

| Property | `perform Role for Type` | `abs dynamic Role` |
| --- | --- | --- |
| Resolution | Compile time | Runtime |
| Lowering | Direct monomorphized call | Fat pointer and vtable call |
| Runtime metadata | None required | Vtable/object metadata |
| Cost model | Zero-cost abstraction | Explicit indirection and dispatch cost |
| Ownership | Normal `erg`/`abs`/`dat` rules | Explicit borrowed dynamic object rules |
| Compatibility | Checked at compilation | Requires a defined dynamic ABI |

## Consequences

- Actus has one contract vocabulary aligned with its grammatical model.
- Static role performance remains predictable and suitable for low-level code.
- Generic bounds can be checked without introducing hidden runtime machinery.
- Dynamic dispatch remains available as a deliberate future extension.
- The compiler must maintain separate resolution and lowering paths for static
  performance and dynamic role objects.
- Dynamic role objects require additional design for vtable layout, object
  lifetime, ABI stability, and FFI interoperability.

## Rejected Alternatives

- `trait` and `impl`: rejected because they import terminology and semantics
  from another language rather than naming Actus contracts directly.
- `interface`: rejected because it does not distinguish static realization from
  runtime dispatch.
- `dyn Role` as the dynamic marker: rejected in favor of the explicit,
  full-word `dynamic` keyword.

## Constraints

Role contracts must preserve Actus ownership and borrowing rules. `erg`
receivers may mutate owned state, `abs` receivers provide inspection, and
`dat` receivers consume ownership. Neither `role` nor `perform` permits
borrow escape, implicit ownership transfer, or a hidden cleanup policy.
