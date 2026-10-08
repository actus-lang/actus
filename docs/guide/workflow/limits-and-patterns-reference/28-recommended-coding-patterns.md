# Complete reference details

This page preserves the complete technical detail for this source section. The shorter handbook pages explain the subject first in everyday language.

## 28. Recommended coding patterns

### Typed failure instead of sentinel status

```act
verb parse_length(abs frame: Buffer) -> Result[u16, DecodeError] {
    if length(frame) < HEADER_SIZE {
        return Result[u16, DecodeError].Err(DecodeError.ShortFrame);
    }
    return Result[u16, DecodeError].Ok(read_length(frame: abs frame));
}
```

Prefer `Result` at public boundaries. Translate foreign negative statuses once
and preserve the typed error after that.

### Explicit ownership in a pipeline

```act
verb process(ins storage: Buffer) -> Result[Int, IoError] {
    erg bytes = read_line(buffer: ins storage)?;
    inspect(input: abs bytes)?;
    return write(buffer: abs bytes);
}
```

Make every transfer visible at the call site. Avoid hidden copies and do not
pass an owner as an ordinary argument when the callee needs a different role.

### Fixed-width protocol arithmetic

```act
const HEADER_SIZE: u16 = 8u16;
const FLAG_VALID: u8 = 0x01u8;

verb valid_flags(erg flags: u8) -> Bool {
    return (flags & FLAG_VALID) != 0u8;
}
```

Use typed literals and constants. Avoid `Int` for fields whose width is part of
the wire or hardware contract.
