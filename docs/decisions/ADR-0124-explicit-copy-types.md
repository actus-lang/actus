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

A `pack` (ADR-0026) was left out of that change and kept the old behaviour: an
assignment, a field initializer, an element assignment, an `erg` argument and an
enum payload duplicated it silently while the same operations moved a struct
(bug #102). The standard library's wire header is a pack and relied on it.

A hidden copy is a defect class of its own: code changes the copy while the
reader believes it changes the original, or the reverse. Standards for
safety-critical software, such as DO-178C for airborne software and MISRA C,
ask for code whose data flow can be traced from the source alone. Actus aims
to meet that standard: nothing that duplicates a value may be invisible.

## Decision

### No implicit copy of a struct, an enum or a pack

No struct, enum or pack value is ever duplicated implicitly. A binding, an
assignment, a struct field initializer, an enum payload, an `erg` or `dat`
argument and a return move their source, whether the type holds a resource or
only plain data. Reading a value does not move it: an operand, a comparison, a
field read (`header.payload_len`), an `abs` argument and a `case` on an `abs`
subject leave the source usable.

### Reading through a view is not a copy

A field read by value does not copy either, and through a view it cannot move.
Reading a struct, an enum, a pack, an array or a resource from a field or an
element whose path starts at an `abs` view or an `ins` loan (`return
parser.header;` with `abs parser`, `erg x = parser.header;`, an argument, a
struct field initializer, an enum payload, an assignment) would neither give up
ownership, since a view owns nothing, nor write a copy, so two names would share
one value silently. The compiler rejects it with `E1010` for an `abs` view and
`E1082` for an `ins` loan (bug #108; the same rule replaces the use of a moved
view, `E1005`, that moving a field out of an `abs` binding used to report,
bug #113). The message names the path. When the type is a copy type the fix is
the written copy, `copy(value: abs parser.header)`; otherwise the value cannot
leave the view, which can only be borrowed with `abs` while the owner keeps it.
Scalars read through a view are unchanged, and so is taking a view of the
field (`abs h = ref parser.header`, an `abs` argument, `-> abs`).

### Reading an array element is not a copy

An array element is owned by its array, and the language does not track moves
of single elements: the array releases every element at its own cleanup
(ADR-0125). Reading an element of a struct, enum, pack or array type BY VALUE
(`erg p = points[i];`) is therefore a move out of an indexed element, which
cannot be honoured, and nothing writes a copy, so it is not a copy either. The
compiler rejects it with `E1010` for every such type, whether it holds a
resource or only plain data (bug #123; bug #126 first closed it for the types
that hold a resource, which a move would release twice). The rule applies at a
constant or a run-time index, through a field of an element
(`lines[i].start`), and in every position that stores or hands over a value: a
binding, an assignment, an element assignment, a struct field initializer, an
enum payload, an `erg` or `dat` argument, a return and the value of an `if`
branch or a `case` arm. The message names the path. When the element type is a
copy type the fix is the written copy, `copy(value: abs points[i])`; otherwise
the element can be borrowed with `abs`, or replaced in place by an assignment,
which releases the value it held. A scalar element (`counts[i]`, `points[i].x`)
is a value and is unchanged, and so is a read by view (an `abs` argument, `ref
points[i]`).

### Declared copy types

A struct, an enum or a pack can declare that it may be duplicated:

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
struct, enum or pack cannot be `meta copy`; the declaration is rejected with a new
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

A `pack` follows the same two rules as a struct, whatever its storage (a
fixed-width integer or `Array[u8, N]`):

- A pack value moves in every place a struct moves: a binding, an assignment,
  a field initializer, an enum payload (`Option` and `Result`), an
  `erg` or `dat` argument, an element assignment and a return. Reads leave the
  source usable: a field read (`header.payload_len`), an operand, an `abs`
  argument.
- A pack can declare `meta copy`, written before the declaration, and is then
  duplicated only by `copy(value: abs x)`:

```actus
meta copy
pack Control {
    erg storage: u32;
    layout little;
    fields {
        erg enabled: u1 at 0;
        abs _reserved: u31 at 1 = 0;
    }
}
```

`meta copy` is the only attribute a pack accepts. The fields of a pack are
fixed-width integers and bounded arrays of them (ADR-0026), and its storage is
a fixed-width integer or `Array[u8, N]`, so every pack satisfies the field rule
of the previous section and the declaration is always valid, apart from a drop
contract, which `E1146` rejects as for any other type. A pack that is not
declared `meta copy` keeps the rejection of `copy` (`E1021`).

The copy has storage of its own on every back end, for a pack backed by an
integer as for a pack backed by a byte array: changing the copy does not change
the original.

### Scalars

This decision does not change scalars. They keep the rules of ADR-0059: they
are copy types without a declaration, `copy(value: abs x)` duplicates them,
and a scalar passed to a parameter is a value, as in Ada, the language of most
airborne software. Where a scalar binding moves and where it is read is a
separate question (bug #93) for its own decision.

## Consequences

- Every duplication of a struct, an enum or a pack is visible in the source as
  `copy(...)`, and every such type that can be duplicated says so at its
  declaration.
- Code that relied on a silent copy no longer compiles and reports the use
  after move (`E1005`) at the later use. The test fixtures that did so and
  twin-e change to write the copy. The standard library's wire header, a pack,
  is declared `meta copy` and its four implicit copies (the two returns of
  `wire_header_decode` and `wire_decode_frame_with_header`, the header that the
  parser stores and the header that `wire_parser_header` returns) are written
  as `copy(value: abs header)`; the examples, the benchmarks and the test
  fixtures did not rely on it.
  No existing behaviour that was correct changes.
- An element read by value is rejected for every aggregate type (bug #123).
  Two compiler tests that copied a plain struct out of an array element
  (`Point`, and the large `Fabric` with its `Cell`) declare those types
  `meta copy` and write `copy(value: abs ...)`; the standard library, the
  examples and twin-e did not rely on it.
- `E1146` is added to the diagnostic catalog.
- ADR-0059 item 1 ("an explicit copy or clone operation for values whose type
  is declared safe to duplicate") is completed for structs, enums and packs.

## Alternatives considered

### Copy every plain struct implicitly

Rejected. It is how C and Go treat structs and it is the defect class this
decision removes: a reader cannot tell from an assignment whether the source
is still the same value.

### Implicit copy for types declared `meta copy`

Rejected. It is how Rust's `Copy` works: the declaration is explicit, but
each copy is not. Actus keeps each copy visible, so a reviewer sees every
place where two values that were one diverge.
