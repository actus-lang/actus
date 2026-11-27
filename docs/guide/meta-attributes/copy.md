# `meta copy`

`meta copy` declares that a struct, an enum or a pack may be duplicated.
Without it, no struct, enum or pack value is ever duplicated: a binding, an assignment, a struct
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
- another struct, enum or pack declared `meta copy`;
- an `Array` of copy types.

A `Buffer`, an `Arena`, an `abs` view, a struct, an enum or a pack that is
not `meta copy`, and a generic parameter are not copy types, and a declaration that
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

`meta copy` is valid before a struct, an enum or a pack declaration. On a verb, a
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

## Packs

A [`pack`](../language/packs.md) follows the same rules as a struct. It moves in
every place a struct moves, a field read such as `header.payload_len` does not
move it, and it is duplicated only by `copy(value: abs x)` once it is declared
`meta copy`. The attribute goes before `pack` (and before `open pack`); it is the
only attribute a pack accepts.

```actus
meta copy
pack Control {
    erg storage: u32;
    layout little;
    fields {
        erg enabled: u1 at 0;
        erg channel: u5 at 1;
        abs _reserved: u26 at 6 = 0;
    }
}

verb main() -> Int {
    erg original = Control { storage: 0u32, };
    original.channel = 3u8;
    erg duplicate = copy(value: abs original);
    duplicate.channel = 9u8;
    return (original.channel as Int) + (duplicate.channel as Int);
}
```

Every field of a pack is a fixed-width integer or a bounded array of them, and
its storage is a fixed-width integer or `Array[u8, N]`, so `meta copy` on a
pack is always valid unless the pack has a drop contract. The copy has its own
storage whichever storage the pack uses. Without `meta copy`, `copy` on a pack
is rejected with `E1021`.

## Not covered

`meta copy` does not make a resource copyable. A value that owns a buffer or an
arena moves; to hand out a second owner, build a new resource explicitly.
