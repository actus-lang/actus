# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 10. Enums and pattern matching

### 10.1 Enum declaration

```act
enum DecodeError {
    Empty,
    InvalidLength,
    Checksum(Int),
}
```

Variants may be unit, tuple, or named-field payloads:

```act
enum Message {
    Ping,
    Data { length: u16, flags: u8 },
}
```

### 10.2 `case`

`case` is the pattern-matching construct:

```act
erg result = read_value();
case dat result {
    Result.Ok(value) => process(dat frame),
    Result.Err(error) => {
        return report_error(error: dat error);
    },
};
```

The `abs` or `dat` mode after `case` controls how the subject is inspected or
transferred where the contract permits. Use `_` for a wildcard:

```act
case status {
    0 => 0,
    _ => 1,
};
```

A boolean guard may follow a pattern:

```act
case value {
    number if number > 0 => number,
    _ => 0,
};
```

The branch body may be an expression or a block. Branches must satisfy the
expected type when used as an expression. Diverging branches such as `return`
do not produce a value and do not incorrectly force an unrelated type join.

### 10.3 `Option` and `Result` patterns

Use qualified variants when the type context is ambiguous:

```act
case dat decoded {
    Result.Ok(frame) => process(dat frame),
    Result.Err(error) => recover(dat error),
};
```

The `_` pattern deliberately ignores a value. If the ignored value owns a
resource, verify that the compiler's cleanup plan is correct; do not hide an
ownership transfer in a wildcard without testing it.
