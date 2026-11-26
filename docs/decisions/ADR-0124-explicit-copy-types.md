# ADR-0124: Explicit Copy Types

## Status

Accepted.

## Context

ADR-0059 forbids the compiler from inventing a copy: a value is duplicated
only by an explicit operation, and only when its type is declared safe to
duplicate. Its first gate added `copy(value: abs x)` for `Int`, `Bool` and
fixed-width integers, and `erg b = a` on an `Int` moves `a`.

The rule was not applied to structs. A struct that holds no resource was
duplicated silently by an assignment, a plain field initializer, an `erg`
argument and an enum payload, while the same operations on a resource moved it
(bugs #79, #89, #90, #96). Tests relied on it: one passed a plain struct to
an `erg` parameter and then read it, another stored a record into an array
element and kept using it.

A hidden copy is a defect class of its own: code changes the copy while the
reader believes it changes the original, or the reverse. Standards for
safety-critical software, such as DO-178C for airborne software and MISRA C,
ask for code whose data flow can be traced from the source alone. Actus aims
to meet that standard: nothing that duplicates a value may be invisible.

## Decision

### No implicit copy of a struct or an enum

No struct or enum value is ever duplicated implicitly. A binding, an
assignment, a struct field initializer, an enum payload, an `erg` or `dat`
argument and a return move their source, whether the type holds a resource or
only plain data. Reading a value does not move it: an operand, a comparison, a
field read (`header.payload_len`), an `abs` argument and a `case` on an `abs`
subject leave the source usable.

### Declared copy types

A struct or an enum can declare that it may be duplicated:

```actus
meta copy
struct Header {
    kind: u8,
    payload_len: u16,
}
```

`meta copy` is accepted only when every field and every payload is itself a
copy type: `Int`, `Usize`, `Bool`, fixed-width integers, floats, another
`meta copy` type, or an `Array` of copy types. A type that holds a `Buffer`, an
`Arena`, a type with a drop contract, an `abs` or `ins` view, or a non-copy
struct or enum cannot be `meta copy`; the declaration is rejected with a new
code, `E1146`, that names the field which prevents it.

### The copy is written

A copy type is still moved by every operation listed above. It is duplicated
only by the existing intrinsic, which now accepts every copy type:

```actus
record.header = copy(value: abs header);
print(header.payload_len);
```

`copy` on a type that is not a copy type keeps its existing rejection.
`meta copy` does not make any copy implicit; it only allows the written one.

### Packs

A `pack` (ADR-0026) is not covered by this decision. Packs are bit-level
layouts of plain fields and keep their current behaviour until a separate
decision; the standard library's wire header is one.

### Scalars

This decision does not change scalars. They keep the rules of ADR-0059: they
are copy types without a declaration, `copy(value: abs x)` duplicates them,
and a scalar passed to a parameter is a value, as in Ada, the language of most
airborne software. Where a scalar binding moves and where it is read is a
separate question (bug #93) for its own decision.

## Consequences

- Every duplication of a struct or an enum is visible in the source as
  `copy(...)`, and every such type that can be duplicated says so at its
  declaration.
- Code that relied on a silent copy no longer compiles and reports the use
  after move (`E1005`) at the later use. The test fixtures that did so and
  twin-e change to write the copy; the standard library and the examples did
  not rely on it.
  No existing behaviour that was correct changes.
- `E1146` is added to the diagnostic catalog.
- ADR-0059 item 1 ("an explicit copy or clone operation for values whose type
  is declared safe to duplicate") is completed for structs and enums.

## Alternatives considered

### Copy every plain struct implicitly

Rejected. It is how C and Go treat structs and it is the defect class this
decision removes: a reader cannot tell from an assignment whether the source
is still the same value.

### Implicit copy for types declared `meta copy`

Rejected. It is how Rust's `Copy` works: the declaration is explicit, but
each copy is not. Actus keeps each copy visible, so a reviewer sees every
place where two values that were one diverge.
