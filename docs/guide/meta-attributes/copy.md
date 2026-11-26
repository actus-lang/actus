# `meta copy`

`meta copy` declares that a struct or an enum may be duplicated. Without it, no
struct or enum value is ever duplicated: a binding, an assignment, a struct
field initializer, an enum payload, an `erg` or `dat` argument and a return all
move their source, whether the type holds a resource or only plain data
(ADR-0124).

```actus
meta copy
struct Header {
    kind: u8,
    payload_len: u16,
}
```

## The copy is written

`meta copy` does not make any copy implicit. A copy type is still moved by
every operation above, and it is duplicated only by the `copy` intrinsic:

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

The argument must be passed as `abs`. The copy has its own storage: changing
the copy does not change the original. A read, such as `header.payload_len`, a
comparison, an `abs` argument or a `case` on an `abs` subject, does not move
the source, so the original stays usable.

`copy` on a type that is not a copy type is rejected with `E1021`. Scalars are
copy types without a declaration and keep the rules of ADR-0059.

## What may be `meta copy`

Every field of a struct and every payload of an enum must itself be a copy
type:

- `Int`, `Usize`, `Bool`, the fixed-width integers and the floats;
- another struct or enum declared `meta copy`;
- an `Array` of copy types.

A `Buffer`, an `Arena`, an `abs` view, a struct or an enum that is not
`meta copy`, and a generic parameter are not copy types, and a declaration that
has a drop contract (`perform Drop`) cannot be `meta copy`. The declaration is
rejected with [`E1146`](../../language/diagnostics/E1146.md)
and the message names the field that prevents it.

```actus fails=E1146
meta copy
struct Frame {
    length: u16,
    payload: Buffer,
}
```

## Placement

`meta copy` is valid before a struct or an enum declaration. On a verb, a
field or anything else it is the usual metadata target error, and writing it
twice on one declaration is the duplicate metadata error.

An enum can be `meta copy` when its payloads are copy types:

```actus
meta copy
enum Sample {
    Missing,
    Reading(u16),
}
```

## Not covered

`meta copy` does not change `pack` declarations, and it does not make a
resource copyable. A value that owns a buffer or an arena moves; to hand out a
second owner, build a new resource explicitly.
