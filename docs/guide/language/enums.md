# Enums and pattern matching

An enum defines a closed set of named variants. Variants may be unit variants or
carry typed payloads.

```actus
enum Status {
    Ready,
    Failed,
    Count(u32),
}
```

Construct a variant with its declared type:

```actus
erg status: Status = Status.Count(3u32);
```

## `case`

Use `case` to inspect an enum:

```actus
case status {
    Status.Ready => {
        return 0;
    },
    Status.Failed => {
        return 1;
    },
    Status.Count(value) => {
        return value as Int;
    },
}
```

Patterns must be exhaustive unless the language form explicitly permits a
validated wildcard. Duplicate and unreachable patterns are rejected.

Unit-only enums support `==` and `!=` when both operands have the same enum
type. The comparison uses the enum discriminant and is lowered consistently by
the Cranelift and LLVM backends. Enums with payloads do not support whole-value
equality; compare their selected payloads explicitly after pattern matching.

## Enums are moved, not copied

An enum value is never duplicated implicitly (ADR-0124). A binding, an
assignment, a struct field initializer, an enum payload, an `erg` or `dat`
argument and a return move the enum, even when it is a plain unit-only enum.
Reading does not: a comparison, an `abs` argument and a `case` on an `abs`
subject leave the source usable.

```actus fails=E1005
enum Mode {
    Idle,
    Busy,
}

verb main() {
    erg mode = Mode.Idle;
    erg other = mode;
    print(bool: mode == Mode.Idle);
}
```

An enum whose payloads are copy types can declare
[`meta copy`](../meta-attributes/copy.md) and be duplicated by a written
`copy(value: abs mode)`:

```actus
meta copy
enum Mode {
    Idle,
    Busy,
}

verb main() {
    erg mode = Mode.Idle;
    erg other = copy(value: abs mode);
    print(bool: mode == Mode.Idle);
}
```

## Payload ownership

A value placed in a payload moves into the enum value. A buffer or an arena
given to `Option`, `Result`, or a user enum cannot be used again through its
source binding, and the source of an `abs` payload stays frozen while the enum
value lives. A payload binding follows the subject's ownership contract. A `dat` subject may
be consumed by the match; an `abs` subject remains available as a read-only
owner after the branch. Payload moves are branch-local and cleanup is planned
for every path.

`Option[T]` and `Result[T, E]` use the same pattern rules:

```actus
case maybe_value {
    Option[u32].Some(value) => {
        return value as Int;
    },
    Option[u32].None => {
        return 0;
    },
}
```

A result branch can propagate a compatible error with `?` from nested calls.
