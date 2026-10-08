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

## Payload ownership

A payload binding follows the subject's ownership contract. A `dat` subject may
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
